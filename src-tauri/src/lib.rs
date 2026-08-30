pub mod activity_classifier;
pub mod analysis;
pub mod autostart;
pub mod collection;
pub mod commands;
pub mod config;
pub mod database;
pub mod error;
pub mod events;
pub mod idle_detector;
pub mod linux_session;
pub mod monitor;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod ocr;
pub mod ocr_logger;
pub mod privacy;
pub mod screenshot;
pub mod screen_lock;
pub mod shell;
pub mod storage;
pub mod work_intelligence;

use collection::{
    background_screenshot_task, hourly_summary_task, resolve_data_dir, resolve_database_path,
    should_hide_main_window_on_setup,
};
pub use collection::{
    copy_dir_contents, default_data_dir, generate_and_save_summary, resolve_activity_classification,
    save_data_dir_preference, AppState,
};
pub use shell::{
    build_tray_icon, configure_main_window, configure_windows_webview_low_power_mode,
    effective_dock_visibility, emit_config_changed, emit_recording_state_changed, ensure_main_window,
    refresh_tray_menu, reveal_main_window, should_prevent_exit, should_request_screen_capture_permission,
    sync_effective_dock_visibility, tray_recording_toggle_action, tray_recording_toggle_label,
    AppLifecycleState, RecordingToggleAction, TrayMenuState, MAIN_WINDOW_LABEL,
    TRAY_MENU_QUIT_ID, TRAY_MENU_RECORDING_TOGGLE_ID, TRAY_MENU_SHOW_ID,
};
use shell::{build_tray_icon as _build_tray_icon_unused, configure_main_window as _cmw_unused};
use config::AppConfig;
use database::Database;
use once_cell::sync::OnceCell;
use privacy::PrivacyFilter;
use screenshot::ScreenshotService;
use std::sync::{Arc, Mutex};
use storage::StorageManager;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};
use shell::APP_HANDLE;

pub const AUTOSTART_LAUNCH_ARG: &str = "--autostart";

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

pub fn run_tauri_app() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    #[cfg(windows)]
    configure_windows_webview_low_power_mode();

    // Linux: 修复 sudo 下丢失的 Wayland/DBus 环境变量
    #[cfg(target_os = "linux")]
    linux_session::fix_wayland_env_if_sudo();

    log::info!("work回顾助手启动中...");

    // 获取数据目录
    let data_dir = resolve_data_dir();
    log::info!("数据目录: {data_dir:?}");

    // 加载配置
    let config_path = data_dir.join("config.json");
    #[allow(unused_mut)]
    let mut config = AppConfig::load(&config_path).unwrap_or_else(|e| {
        log::warn!("加载配置失败，使用默认配置: {e}");
        AppConfig::default()
    });
    config.lightweight_mode = true;

    // 迁移旧版 excluded_apps → app_rules
    if config.privacy.migrate_legacy_excluded_apps() {
        log::info!("已迁移旧版 excluded_apps 到 app_rules");
        if let Err(e) = config.save(&config_path) {
            log::warn!("保存迁移后的配置失败: {e}");
        }
    }

    // 初始化数据库
    let db_path = resolve_database_path(&data_dir, &config);
    let database = Database::new(&db_path).expect("初始化数据库失败");

    // 首次启动或升级后重建 FTS 索引，确保历史数据可被全文检索
    if let Err(e) = database.rebuild_fts_index() {
        log::warn!("FTS 索引重建失败（不影响核心功能）: {e}");
    }

    // 初始化隐私过滤器
    let privacy_filter = PrivacyFilter::from_config(&config.privacy);

    // 初始化截屏服务
    let screenshot_service = ScreenshotService::new(&data_dir, &config.storage);

    // macOS: 启动时检查并请求必要的系统权限
    #[cfg(target_os = "macos")]
    {
        // 1. 屏幕录制权限（截图功能必需）
        let has_screen_capture_permission = screenshot::has_screen_capture_permission();
        let already_prompted = config.macos_screen_capture_permission_prompted;
        if should_request_screen_capture_permission(has_screen_capture_permission, already_prompted)
        {
            log::warn!("⚠️  屏幕录制权限未授权，正在请求...");
            log::warn!(
                "   请在「系统设置 → 隐私与安全性 → 屏幕录制」中授权 Work Review，然后重启应用"
            );
            screenshot::request_screen_capture_permission();
        } else if !has_screen_capture_permission {
            log::warn!("⚠️  屏幕录制权限仍未授权，跳过重复请求，请在系统设置中确认后重启应用");
        } else {
            log::info!("✅ 屏幕录制权限已授权");
        }
        config.macos_screen_capture_permission_prompted = !has_screen_capture_permission;
        if config.macos_screen_capture_permission_prompted != already_prompted {
            if let Err(e) = config.save(&config_path) {
                log::warn!("保存 macOS 录屏权限提示状态失败: {e}");
            }
        }

        // 2. 辅助功能权限（读取窗口标题、浏览器 URL 必需）
        if !screenshot::has_accessibility_permission(false) {
            log::warn!("⚠️  辅助功能权限未授权，正在请求...");
            log::warn!("   请在「系统设置 → 隐私与安全性 → 辅助功能」中授权 Work Review");
            // prompt=true 会弹出系统引导对话框
            screenshot::has_accessibility_permission(true);
        } else {
            log::info!("✅ 辅助功能权限已授权");
        }
    }

    // 初始化存储管理器
    let storage_manager = StorageManager::new(&data_dir, config.storage.clone());

    // 启动时执行一次清理
    if let Err(e) = storage_manager.cleanup() {
        log::warn!("启动时清理存储失败: {e}");
    }

    // 创建应用状态，使用 Arc 包装以便在多个地方共享
    let app_state = Arc::new(Mutex::new(AppState {
        config,
        database,
        privacy_filter,
        screenshot_service,
        storage_manager,
        data_dir,
        db_path,
        config_path,
        is_recording: true,
        is_paused: false,
        generating_report: false,
        cached_active_window: None,
    }));
    let app_lifecycle_state = Arc::new(Mutex::new(AppLifecycleState::default()));

    // 构建 Tauri 应用
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init());
    #[cfg(not(windows))]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        Some(vec![AUTOSTART_LAUNCH_ARG, "--hidden"]),
    ));
    builder
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // 如果第二个实例是自启动触发的，保持静默不弹窗
            if argv.iter().any(|arg| arg == AUTOSTART_LAUNCH_ARG) {
                log::info!("检测到重复自启动，保持静默 | 参数: {argv:?}");
                return;
            }
            // 当用户尝试打开第二个实例时，将焦点给到现有窗口
            if let Err(e) = reveal_main_window(&app.clone(), None) {
                log::warn!("恢复主窗口失败: {e}");
            }
            log::info!("检测到重复打开，参数: {argv:?}, 工作目录: {cwd}");
        }))
        .manage(app_state.clone())
        .manage(app_lifecycle_state.clone())
        // 系统托盘在 setup 中创建 (Tauri v2)
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(lifecycle_state) = window.try_state::<Arc<Mutex<AppLifecycleState>>>() {
                    let mut lifecycle_state =
                        lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                    lifecycle_state.suppress_next_exit = true;
                }
            } else if let tauri::WindowEvent::Destroyed = event {
                sync_effective_dock_visibility(&window.app_handle());
            }
        })
        .setup(|app| {
            if let Err(e) = autostart::init_autostart(&app.handle()) {
                log::warn!("初始化开机自启功能失败: {e}");
            }

            let window = app
                .get_webview_window("main")
                .expect("main window should exist at setup");
            configure_main_window(&window);
            let launch_args = std::env::args().collect::<Vec<_>>();
            // 获取 Arc<Mutex<AppState>> 并克隆以便在异步任务中使用
            let state = app.state::<Arc<Mutex<AppState>>>();
            let should_hide_main_window = {
                let state_guard = state.inner().lock().unwrap_or_else(|e| e.into_inner());
                if state_guard.config.auto_start {
                    if let Err(e) = autostart::enable_autostart(
                        app.handle().clone(),
                        state_guard.config.auto_start_silent,
                    ) {
                        log::warn!("同步修复开机自启注册项失败: {e}");
                    }
                }
                let result = should_hide_main_window_on_setup(&state_guard.config, &launch_args);
                log::info!(
                    "启动窗口决策: show={} | auto_start={} auto_start_silent={} args={:?}",
                    !result,
                    state_guard.config.auto_start,
                    state_guard.config.auto_start_silent,
                    launch_args,
                );
                result
            };

            if should_hide_main_window {
                let _ = window.hide();
            } else {
                let _ = window.show();
            }

            let state_clone = state.inner().clone();
            let state_clone2 = state.inner().clone();
            let state_for_tray = state.inner().clone();
            let screenshot_app_handle = app.handle().clone();

            // 创建 Tauri v2 系统托盘
            let show = MenuItemBuilder::with_id(TRAY_MENU_SHOW_ID, "显示窗口").build(app)?;
            let recording_toggle = MenuItemBuilder::with_id(
                TRAY_MENU_RECORDING_TOGGLE_ID,
                tray_recording_toggle_label(true, false),
            )
            .build(app)?;
            let quit = MenuItemBuilder::with_id(TRAY_MENU_QUIT_ID, "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&show)
                .separator()
                .item(&recording_toggle)
                .separator()
                .item(&quit)
                .build()?;

            app.manage(TrayMenuState {
                recording_toggle: recording_toggle.clone(),
            });
            refresh_tray_menu(&app.handle());

            let tray_icon = build_tray_icon(app);
            let tray_builder = TrayIconBuilder::new().icon(tray_icon).menu(&menu);

            #[cfg(target_os = "macos")]
            let tray_builder = tray_builder.icon_as_template(true);

            let _tray = tray_builder
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    TRAY_MENU_QUIT_ID => {
                        if let Some(lifecycle_state) =
                            app.try_state::<Arc<Mutex<AppLifecycleState>>>()
                        {
                            let mut lifecycle_state =
                                lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                            lifecycle_state.explicit_quit_requested = true;
                        }
                        app.exit(0);
                    }
                    TRAY_MENU_SHOW_ID => {
                        if let Err(e) = reveal_main_window(&app.clone(), None) {
                            log::warn!("从托盘恢复主窗口失败: {e}");
                        }
                    }
                    TRAY_MENU_RECORDING_TOGGLE_ID => {
                        {
                            let mut state =
                                state_for_tray.lock().unwrap_or_else(|e| e.into_inner());
                            let action =
                                tray_recording_toggle_action(state.is_recording, state.is_paused);
                            match action {
                                RecordingToggleAction::Start => {
                                    state.is_recording = true;
                                    state.is_paused = false;
                                    log::info!("托盘操作：开始录制");
                                }
                                RecordingToggleAction::Pause => {
                                    state.is_paused = true;
                                    log::info!("托盘操作：暂停录制");
                                }
                                RecordingToggleAction::Resume => {
                                    state.is_paused = false;
                                    log::info!("托盘操作：恢复录制");
                                }
                            }
                        }
                        emit_recording_state_changed(&app);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |_tray, event| {
                    // 处理托盘图标点击
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app_handle = _tray.app_handle();
                        if let Err(e) = reveal_main_window(&app_handle, None) {
                            log::warn!("点击托盘恢复主窗口失败: {e}");
                        }
                    }
                })
                .build(app)?;

            // 启动后台截屏任务
            tauri::async_runtime::spawn(async move {
                background_screenshot_task(state_clone).await;
            });

            // 启动小时摘要生成任务（每小时检查一次）
            tauri::async_runtime::spawn(async move {
                hourly_summary_task(state_clone2).await;
            });

            // 启动时清理当天的重复记录
            {
                let state_guard = state.inner().lock().unwrap_or_else(|e| e.into_inner());
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                match state_guard.database.cleanup_duplicate_activities(&today) {
                    Ok((deleted, paths)) => {
                        if deleted > 0 {
                            log::warn!("🧹 启动清理: 删除 {deleted} 条重复记录");
                            // 删除对应的截图文件
                            for p in paths {
                                let path = state_guard.data_dir.join(&p);
                                if path.exists() {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("清理重复记录失败: {e}"),
                }

                // decorations 配置由 tauri.conf.json 控制，用户可通过设置中的开关动态修改
            }

            sync_effective_dock_visibility(&app.handle());

            // 保存 AppHandle 到全局变量，用于从 macOS Dock 点击恢复窗口
            let _ = APP_HANDLE.set(app.handle().clone());

            // 注: macOS Dock 点击恢复窗口通过系统托盘 LeftClick 事件处理
            // 用户需要点击状态栏的系统托盘图标来恢复窗口

            log::info!("应用初始化完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            autostart::enable_autostart,
            autostart::disable_autostart,
            autostart::is_autostart_enabled,
            commands::get_today_stats,
            commands::get_overview_stats,
            commands::get_daily_stats,
            commands::get_timeline,
            commands::get_hourly_app_breakdown,
            commands::generate_report,
            commands::get_saved_report,
            commands::update_report_content,
            commands::export_report_markdown,
            commands::get_config,
            commands::save_config,
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::get_recording_state,
            commands::get_data_dir,
            commands::get_database_path,
            commands::get_default_data_dir,
            commands::get_runtime_platform,
            commands::get_linux_session_support,
            commands::change_data_dir,
            commands::change_database_path,
            commands::cleanup_old_data_dir,
            commands::open_data_dir,
            commands::get_screenshot_thumbnail,
            commands::get_screenshot_full,
            commands::take_screenshot,
            commands::get_running_apps,
            commands::get_recent_apps,
            commands::get_app_category_overview,
            commands::set_app_category_rule,
            commands::set_domain_semantic_rule,
            commands::reclassify_app_history,
            commands::get_categories,
            commands::save_custom_category,
            commands::delete_custom_category,
            commands::get_semantic_categories,
            commands::save_custom_semantic_category,
            commands::delete_custom_semantic_category,
            commands::get_storage_stats,
            commands::get_hourly_summaries,
            commands::get_activity,
            commands::search_memory,
            commands::get_work_sessions,
            commands::recognize_work_intents,
            commands::generate_weekly_review,
            commands::extract_todo_items,
            commands::clear_old_activities,
            commands::get_ocr_log,
            commands::is_screen_locked,
            commands::check_permissions,
            commands::open_permission_settings,
            commands::is_work_time,
            commands::check_ocr_available,
            commands::run_ocr,
            commands::get_ocr_install_guide,
            commands::set_dock_visibility,
            commands::get_app_icon,
            commands::save_background_image,
            commands::get_background_image,
            commands::clear_background_image,
            commands::show_main_window,
            commands::get_manual_followups,
            commands::add_manual_followup,
            commands::update_manual_followup_status,
            commands::delete_manual_followup,
            commands::get_platform,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri 应用时出错")
        .run(|_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if let Some(lifecycle_state) =
                    _app_handle.try_state::<Arc<Mutex<AppLifecycleState>>>()
                {
                    let mut lifecycle_state =
                        lifecycle_state.lock().unwrap_or_else(|e| e.into_inner());
                    let should_prevent = should_prevent_exit(
                        lifecycle_state.suppress_next_exit,
                        lifecycle_state.explicit_quit_requested,
                    );
                    lifecycle_state.suppress_next_exit = false;

                    if should_prevent {
                        log::info!("拦截最后一个主窗口关闭导致的退出，保留后台与托盘");
                        api.prevent_exit();
                        return;
                    }
                }
            }
            // 处理 macOS Dock 点击：显示隐藏的窗口（仅 macOS）
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                if !has_visible_windows {
                    if let Err(e) = reveal_main_window(&_app_handle.clone(), None) {
                        log::warn!("Dock 恢复主窗口失败: {e}");
                    }
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
        use crate::collection::{
        browser_change_capture_min_interval_ms, launch_args_contain_autostart,
        monitoring_poll_interval_ms, monitoring_poll_interval_ms_for_platform,
        previous_app_backfill_duration, recording_loop_decision, resolve_activity_classification,
        reusable_cached_active_window, screen_lock_check_interval_ms_for_platform,
        should_confirm_idle, should_hide_main_window_on_setup, should_persist_merge_update,
        should_probe_browser_url_before_change_detection, should_skip_system_window,
    };
    use crate::shell::{
        effective_dock_visibility, should_prevent_exit, should_request_screen_capture_permission,
        tray_recording_toggle_action, tray_recording_toggle_label, RecordingToggleAction,
    };
    use crate::config::{AppConfig, WebsiteSemanticRule};
    use crate::monitor::ActiveWindow;
    use std::time::{Duration, Instant};

    #[test]
    fn 暂停录制时应重置截图计时器() {
        let decision = recording_loop_decision(true, true, 30);
        assert!(!decision.should_continue);
        assert!(decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 1);
    }

    #[test]
    fn 停止录制时应重置截图计时器() {
        let decision = recording_loop_decision(false, false, 30);
        assert!(!decision.should_continue);
        assert!(decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 1);
    }

    #[test]
    fn 正常录制时应保留截图间隔() {
        let decision = recording_loop_decision(true, false, 30);
        assert!(decision.should_continue);
        assert!(!decision.reset_capture_clock);
        assert_eq!(decision.screenshot_interval, 30);
    }

    #[test]
    fn 关闭截图后应直接按输入空闲判断为空闲() {
        assert!(should_confirm_idle(true, 5 * 60, false, false));
        assert!(!should_confirm_idle(false, 0, false, true));
    }

    #[test]
    fn 开启截图后仍应依赖截图确认空闲() {
        assert!(!should_confirm_idle(true, 5 * 60, true, false));
        assert!(should_confirm_idle(true, 5 * 60, true, true));
    }

    #[test]
    fn 长时间无输入时应强制停止累计活跃时长() {
        assert!(should_confirm_idle(true, 20 * 60, true, false));
        assert!(!should_confirm_idle(true, 19 * 60, true, false));
        assert!(should_confirm_idle(true, 25 * 60, true, false));
    }

    #[test]
    fn 已进入输入空闲后切换应用不应回补上一应用时长() {
        assert_eq!(previous_app_backfill_duration(true, 3600, true, false), 0);
        assert_eq!(previous_app_backfill_duration(true, 3600, false, true), 0);
        assert_eq!(
            previous_app_backfill_duration(true, 3600, false, false),
            3600
        );
        assert_eq!(previous_app_backfill_duration(false, 3600, false, false), 0);
    }

    #[test]
    fn 合并活动即使本轮不累计时长也应刷新记录() {
        assert!(should_persist_merge_update(120, true));
        assert!(should_persist_merge_update(0, true));
        assert!(!should_persist_merge_update(0, false));
    }

    #[test]
    fn 当前平台主监控轮询间隔应匹配平台策略() {
        assert_eq!(
            monitoring_poll_interval_ms(),
            monitoring_poll_interval_ms_for_platform(cfg!(target_os = "macos"))
        );
    }

    #[test]
    fn 域名语义规则应覆盖浏览器活动默认分类() {
        let mut config = AppConfig::default();
        config.website_semantic_rules = vec![WebsiteSemanticRule {
            domain: "github.com".to_string(),
            semantic_category: "任务规划".to_string(),
        }];
        config.normalize();

        let classification = resolve_activity_classification(
            &config,
            "Google Chrome",
            "Issue #28",
            Some("https://github.com/issues/28"),
        );

        assert_eq!(classification.base_category, "browser");
        assert_eq!(classification.semantic_category, "任务规划");
    }

    #[test]
    fn 非mac主监控轮询间隔应保持半秒() {
        assert_eq!(monitoring_poll_interval_ms_for_platform(false), 500);
    }

    #[test]
    fn mac主监控轮询间隔应降频() {
        assert_eq!(monitoring_poll_interval_ms_for_platform(true), 1500);
    }

    #[test]
    fn 同标题浏览器页应在切换判定前主动探测真实网址() {
        assert!(should_probe_browser_url_before_change_detection(
            "Google Chrome",
            "项目文档",
            Some("Google Chrome"),
            Some("项目文档"),
            None,
        ));
        assert!(!should_probe_browser_url_before_change_detection(
            "Google Chrome",
            "项目文档",
            Some("Google Chrome"),
            Some("另一个标签页"),
            None,
        ));
        assert!(!should_probe_browser_url_before_change_detection(
            "Cursor",
            "main.rs",
            Some("Cursor"),
            Some("main.rs"),
            None,
        ));
    }

    #[test]
    fn 首次遇到浏览器窗口时应探测url() {
        assert!(should_probe_browser_url_before_change_detection(
            "Google Chrome",
            "项目文档",
            None,
            None,
            None,
        ));
        assert!(!should_probe_browser_url_before_change_detection(
            "Google Chrome",
            "项目文档",
            None,
            None,
            Some("https://example.com"),
        ));
    }

    #[test]
    fn 浏览器导航变化应使用更短的截图冷却() {
        assert_eq!(
            browser_change_capture_min_interval_ms("Google Chrome", true, false),
            1200
        );
        assert_eq!(
            browser_change_capture_min_interval_ms("Google Chrome", false, true),
            1200
        );
        assert_eq!(
            browser_change_capture_min_interval_ms("Google Chrome", false, false),
            3000
        );
        assert_eq!(
            browser_change_capture_min_interval_ms("Cursor", true, false),
            3000
        );
    }

    #[test]
    fn mac锁屏检测轮询间隔应显著降频() {
        assert_eq!(screen_lock_check_interval_ms_for_platform(true), 5000);
    }

    #[test]
    fn 新鲜的活动窗口缓存应被截图循环复用() {
        let now = Instant::now();
        let cached_window = ActiveWindow {
            app_name: "Cursor".to_string(),
            window_title: "main.rs".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: false,
        };

        let reused = reusable_cached_active_window(Some(&(now, cached_window.clone())), now);

        assert!(reused.is_some());
        let reused = reused.expect("fresh cache should be reused");
        assert_eq!(reused.app_name, cached_window.app_name);
        assert_eq!(reused.window_title, cached_window.window_title);
    }

    #[test]
    fn 过期的活动窗口缓存不应被截图循环复用() {
        let now = Instant::now();
        let cached_window = ActiveWindow {
            app_name: "Cursor".to_string(),
            window_title: "main.rs".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: false,
        };
        let stale_at = now
            .checked_sub(Duration::from_millis(1500))
            .expect("stale timestamp should be valid");

        let reused = reusable_cached_active_window(Some(&(stale_at, cached_window)), now);

        assert!(reused.is_none());
    }

    #[test]
    fn dock可见性应考虑用户偏好与主窗口是否存在() {
        assert!(!effective_dock_visibility(true, true));
        assert!(effective_dock_visibility(false, true));
        assert!(!effective_dock_visibility(false, false));
    }

    #[test]
    fn 托盘录制按钮应根据当前状态切换动作() {
        assert_eq!(
            tray_recording_toggle_action(false, false),
            RecordingToggleAction::Start
        );
        assert_eq!(
            tray_recording_toggle_action(true, false),
            RecordingToggleAction::Pause
        );
        assert_eq!(
            tray_recording_toggle_action(true, true),
            RecordingToggleAction::Resume
        );
    }

    #[test]
    fn 托盘录制按钮文案应与状态一致() {
        assert_eq!(tray_recording_toggle_label(false, false), "开始录制");
        assert_eq!(tray_recording_toggle_label(true, false), "暂停录制");
        assert_eq!(tray_recording_toggle_label(true, true), "恢复录制");
    }

    #[test]
    fn 仅应拦截主窗口关闭导致的被动退出() {
        assert!(should_prevent_exit(true, false));
        assert!(!should_prevent_exit(false, false));
        assert!(!should_prevent_exit(true, true));
    }

    #[test]
    #[cfg(not(windows))]
    fn 非windows上应优先使用_autostart_或_hidden_参数并结合配置判定隐藏() {
        let mut config = AppConfig::default();
        config.auto_start = true;
        config.auto_start_silent = true;

        assert!(should_hide_main_window_on_setup(
            &config,
            &["work-review".to_string(), "--autostart".to_string()]
        ));
        assert!(!should_hide_main_window_on_setup(
            &config,
            &["work-review".to_string()]
        ));
        assert!(should_hide_main_window_on_setup(
            &config,
            &["work-review".to_string(), "--hidden".to_string()]
        ));
        assert!(should_hide_main_window_on_setup(
            &config,
            &["work-review".to_string(), "--minimized".to_string()]
        ));
    }

    #[test]
    #[cfg(windows)]
    fn windows上应仅凭_launch_args_里显式的_hidden_决定是否隐藏() {
        // 注册表参数由 silent 选择动态写入，显隐决策不再依赖 config，消除失同步翻车。
        let mut silent_config = AppConfig::default();
        silent_config.auto_start = true;
        silent_config.auto_start_silent = true;

        // silent 模式注册表 → --autostart --hidden → 隐藏
        assert!(should_hide_main_window_on_setup(
            &silent_config,
            &[
                "work-review".to_string(),
                "--autostart".to_string(),
                "--hidden".to_string(),
            ]
        ));
        // show 模式注册表 → 只有 --autostart → 显示
        assert!(!should_hide_main_window_on_setup(
            &silent_config,
            &["work-review".to_string(), "--autostart".to_string()]
        ));
        // 普通手动打开 → 显示
        assert!(!should_hide_main_window_on_setup(
            &silent_config,
            &["work-review".to_string()]
        ));

        // 即使 config 还没保存用户选择（失同步），注册表里的 --hidden 依然能强制隐藏
        let mut stale_config = AppConfig::default();
        stale_config.auto_start = false;
        stale_config.auto_start_silent = false;
        assert!(should_hide_main_window_on_setup(
            &stale_config,
            &["work-review".to_string(), "--hidden".to_string()]
        ));
        assert!(should_hide_main_window_on_setup(
            &stale_config,
            &["work-review".to_string(), "--minimized".to_string()]
        ));
    }

    #[test]
    fn 自启动参数判定应精确匹配_autostart() {
        assert!(launch_args_contain_autostart(&[
            "work-review".to_string(),
            "--autostart".to_string()
        ]));
        assert!(!launch_args_contain_autostart(&[
            "work-review".to_string(),
            "--autostarted".to_string()
        ]));
    }

    #[test]
    fn 任务管理器未响应时仍应识别为系统窗口() {
        let active_window = ActiveWindow {
            app_name: "任务管理器 (未响应)".to_string(),
            window_title: "任务管理器 (未响应)".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: false,
        };

        assert!(should_skip_system_window(&active_window));
    }

    #[test]
    fn uac提示窗口应识别为系统窗口() {
        let active_window = ActiveWindow {
            app_name: "consent.exe".to_string(),
            window_title: "用户账户控制".to_string(),
            browser_url: None,
            executable_path: Some(r"C:\Windows\System32\consent.exe".to_string()),
            window_bounds: None,
            is_minimized: false,
        };

        assert!(should_skip_system_window(&active_window));
    }

    #[test]
    fn windows_security提示窗口应识别为系统窗口() {
        let active_window = ActiveWindow {
            app_name: "Windows Security".to_string(),
            window_title: "Windows Security".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: false,
        };

        assert!(should_skip_system_window(&active_window));
    }

    #[test]
    fn windows最小化前台窗口应视为非工作窗口() {
        let active_window = ActiveWindow {
            app_name: "WeChat".to_string(),
            window_title: "微信".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: true,
        };

        assert!(should_skip_system_window(&active_window));
    }

    #[test]
    fn work_review自身窗口不应被当成系统窗口跳过() {
        let active_window = ActiveWindow {
            app_name: "Work Review".to_string(),
            window_title: "时间线".to_string(),
            browser_url: None,
            executable_path: Some(
                "/Applications/Work Review.app/Contents/MacOS/work-review".to_string(),
            ),
            window_bounds: None,
            is_minimized: false,
        };

        assert!(!should_skip_system_window(&active_window));
    }

    #[test]
    fn 普通应用标题提到任务管理器时不应被误判为系统窗口() {
        let active_window = ActiveWindow {
            app_name: "任务管理器实现说明".to_string(),
            window_title: "任务管理器实现说明".to_string(),
            browser_url: None,
            executable_path: None,
            window_bounds: None,
            is_minimized: false,
        };

        assert!(!should_skip_system_window(&active_window));
    }

    #[test]
    fn macos录屏权限已提示过时不应重复请求() {
        assert!(should_request_screen_capture_permission(false, false));
        assert!(!should_request_screen_capture_permission(false, true));
        assert!(!should_request_screen_capture_permission(true, false));
    }
}
