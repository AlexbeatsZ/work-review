// Windows FFI 引擎入口：供 WinUI 3（C#）宿主通过 P/Invoke 调用。
// 仅在 `ffi` feature 下编译；采集循环与存储与 Tauri 构建共用同一实现。
use crate::collection::{default_data_dir, resolve_database_path, AppState};
use crate::commands;
use crate::config::AppConfig;
use crate::database::Database;
use crate::error::AppError;
use crate::events;
use crate::privacy::PrivacyFilter;
use crate::screenshot::ScreenshotService;
use crate::storage::StorageManager;
use crate::AUTOSTART_LAUNCH_ARG;
use once_cell::sync::OnceCell;
use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

/// 宿主事件回调：event / payload 均为 UTF-8 NUL 结尾字符串。
/// 回调在 Rust 工作线程上触发，宿主负责自行调度回 UI 线程。
pub type EventCallback = extern "system" fn(event: *const c_char, payload: *const c_char);

static EVENT_CALLBACK: Mutex<Option<EventCallback>> = Mutex::new(None);
static RUNTIME: OnceCell<Runtime> = OnceCell::new();
static STATE: OnceCell<Arc<Mutex<AppState>>> = OnceCell::new();

const AUTOSTART_APP_NAME: &str = "Work Review";

/// 把事件投递给宿主（events::emit_event 在 ffi 构建下调用）。
pub fn emit_to_host(name: &str, payload: &Value) {
    let guard = EVENT_CALLBACK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(cb) = guard.as_ref() {
        let Ok(ev) = CString::new(name) else { return };
        let Ok(pl) = CString::new(payload.to_string()) else { return };
        cb(ev.as_ptr(), pl.as_ptr());
    }
}

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("初始化 tokio 运行时失败")
    })
}

fn state() -> &'static Arc<Mutex<AppState>> {
    STATE.get().expect("引擎尚未启动（engine_start）")
}

/// 注册（或清除）宿主事件回调。须在 engine_start 之前调用以免丢失事件。
#[no_mangle]
pub extern "C" fn engine_set_event_callback(callback: Option<EventCallback>) {
    *EVENT_CALLBACK.lock().unwrap_or_else(|e| e.into_inner()) = callback;
}

/// 启动采集引擎：初始化配置、数据库并启动后台采集任务。返回 0 表示成功。
#[no_mangle]
pub extern "C" fn engine_start() -> i32 {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    match runtime().block_on(bootstrap()) {
        Ok(()) => 0,
        Err(e) => {
            log::error!("engine_start 失败: {e}");
            1
        }
    }
}

async fn bootstrap() -> Result<(), AppError> {
    // 与 Tauri main 的 setup 顺序保持一致
    let data_dir = crate::collection::resolve_data_dir();
    log::info!("数据目录: {data_dir:?}");

    let config_path = data_dir.join("config.json");
    let mut config = AppConfig::load(&config_path).unwrap_or_else(|e| {
        log::warn!("加载配置失败，使用默认配置: {e}");
        AppConfig::default()
    });
    config.lightweight_mode = true;

    if config.privacy.migrate_legacy_excluded_apps() {
        log::info!("已迁移旧版 excluded_apps 到 app_rules");
        if let Err(e) = config.save(&config_path) {
            log::warn!("保存迁移后的配置失败: {e}");
        }
    }

    let db_path = resolve_database_path(&data_dir, &config);
    let database = Database::new(&db_path)?;
    if let Err(e) = database.rebuild_fts_index() {
        log::warn!("FTS 索引重建失败（不影响核心功能）: {e}");
    }

    let privacy_filter = PrivacyFilter::from_config(&config.privacy);
    let screenshot_service = ScreenshotService::new(&data_dir, &config.storage);
    let storage_manager = StorageManager::new(&data_dir, config.storage.clone());
    if let Err(e) = storage_manager.cleanup() {
        log::warn!("启动时清理存储失败: {e}");
    }

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

    // 启动时清理当天的重复记录
    {
        let state_guard = app_state.lock().unwrap_or_else(|e| e.into_inner());
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        match state_guard.database.cleanup_duplicate_activities(&today) {
            Ok((deleted, paths)) => {
                if deleted > 0 {
                    log::warn!("🧹 启动清理: 删除 {deleted} 条重复记录");
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
    }

    // 自启动同步修复：配置开启时重写注册表项（可修复指向旧安装位置的失效项）
    {
        let state_guard = app_state.lock().unwrap_or_else(|e| e.into_inner());
        if state_guard.config.auto_start {
            let silent = state_guard.config.auto_start_silent;
            drop(state_guard);
            if let Err(e) = autostart_set(true, silent) {
                log::warn!("同步修复开机自启注册项失败: {e}");
            }
        }
    }

    let _ = STATE.set(app_state.clone());

    tokio::spawn(crate::collection::background_screenshot_task(app_state.clone()));
    tokio::spawn(crate::collection::hourly_summary_task(app_state));

    log::info!("采集引擎初始化完成");
    Ok(())
}

/// 通用调用入口：method 为命令名，args 为 JSON 对象（键与 Tauri 参数一致，snake_case）。
/// 返回 JSON 字符串 `{"ok":true,"value":...}` 或 `{"ok":false,"error":"..."}`，
/// 由 engine_free_string 释放。
#[no_mangle]
pub extern "C" fn engine_invoke(method: *const c_char, args: *const c_char) -> *mut c_char {
    let payload = std::panic::catch_unwind(|| {
        let method = if method.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(method) }.to_string_lossy().to_string()
        };
        let args_raw = if args.is_null() {
            "{}".to_string()
        } else {
            unsafe { CStr::from_ptr(args) }.to_string_lossy().to_string()
        };
        let args: Value = serde_json::from_str(&args_raw).unwrap_or(Value::Null);
        dispatch(&method, &args)
    })
    .unwrap_or_else(|_| json!({"ok": false, "error": "引擎内部错误（panic）"}));

    match CString::new(payload.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => CString::new(r#"{"ok":false,"error":"payload contains NUL"}"#)
            .expect("static string has no NUL")
            .into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn engine_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

// ---------- 参数解析辅助 ----------

fn a_str(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn a_opt_str(v: &Value, k: &str) -> Option<String> {
    match v.get(k) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Null) | None => None,
        Some(other) => Some(other.to_string().trim_matches('"').to_string()),
    }
}

fn a_opt_u32(v: &Value, k: &str) -> Option<u32> {
    v.get(k).and_then(|x| x.as_u64()).map(|x| x as u32)
}

fn a_i64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or(0)
}

fn a_bool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or(false)
}

fn to_value<T: serde::Serialize>(r: Result<T, AppError>) -> Result<Value, AppError> {
    r.map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
}

// ---------- 分发 ----------

fn dispatch(method: &str, args: &Value) -> Value {
    let result: Result<Value, AppError> = runtime().block_on(async {
        let st = state();
        match method {
            "get_timeline" => to_value(commands::get_timeline_inner(
                a_str(args, "date"),
                a_opt_u32(args, "limit"),
                a_opt_u32(args, "offset"),
                st,
            )),
            "get_hourly_summaries" => to_value(
                commands::get_hourly_summaries_inner(&a_str(args, "date"), st),
            ),
            "get_activity" => to_value(commands::get_activity_inner(a_i64(args, "id"), st).await),
            "get_screenshot_thumbnail" => to_value(
                commands::get_screenshot_thumbnail_inner(a_str(args, "path"), st).await,
            ),
            "get_screenshot_full" => to_value(
                commands::get_screenshot_full_inner(a_str(args, "path"), st).await,
            ),
            "get_categories" => to_value(commands::get_categories_inner(st)),
            "get_semantic_categories" => {
                to_value(commands::get_semantic_categories_inner(st))
            }
            "save_custom_category" => to_value(
                commands::save_custom_category_inner(
                    a_str(args, "key"),
                    a_str(args, "name"),
                    a_str(args, "color"),
                    a_str(args, "icon"),
                    st,
                )
                .await,
            ),
            "delete_custom_category" => to_value(
                commands::delete_custom_category_inner(a_str(args, "key"), a_opt_str(args, "reassign_to"), st)
                    .await,
            ),
            "set_app_category_rule" => to_value(
                commands::set_app_category_rule_inner(
                    a_str(args, "app_name"),
                    a_str(args, "category"),
                    a_bool(args, "sync_history"),
                    st,
                )
                .await,
            ),
            "set_domain_semantic_rule" => to_value(
                commands::set_domain_semantic_rule_inner(
                    a_str(args, "domain"),
                    a_str(args, "semantic_category"),
                    a_bool(args, "sync_history"),
                    st,
                )
                .await,
            ),
            "reclassify_app_history" => to_value(
                commands::reclassify_app_history_inner(a_str(args, "app_name"), a_str(args, "category"), st)
                    .await,
            ),
            "get_recent_apps" => to_value(commands::get_recent_apps_inner(st)),
            "get_running_apps" => to_value(commands::get_running_apps().await),
            "get_app_icon" => to_value(
                commands::get_app_icon(a_str(args, "app_name"), a_opt_str(args, "executable_path")).await,
            ),
            "get_config" => to_value(commands::get_config_inner(st).await),
            "save_config" => {
                let config: AppConfig = args
                    .get("config")
                    .ok_or_else(|| AppError::Config("缺少 config 参数".to_string()))
                    .and_then(|c| serde_json::from_value(c.clone()).map_err(|e| AppError::Config(format!("config 解析失败: {e}"))))?;
                to_value(commands::save_config_inner(config, st).await)
            }
            "start_recording" => to_value(commands::start_recording_inner(st).await),
            "stop_recording" => to_value(commands::stop_recording_inner(st).await),
            "pause_recording" => to_value(commands::pause_recording_inner(st).await),
            "resume_recording" => to_value(commands::resume_recording_inner(st).await),
            "get_recording_state" => to_value(commands::get_recording_state_inner(st).await),
            "get_storage_stats" => to_value(commands::get_storage_stats_inner(st)),
            "clear_old_activities" => to_value(commands::clear_old_activities_inner(st).await),
            "get_data_dir" => to_value(commands::get_data_dir_inner(st).await),
            "get_database_path" => to_value(commands::get_database_path_inner(st).await),
            "get_default_data_dir" => to_value(commands::get_default_data_dir().await),
            "change_data_dir" => to_value(
                commands::change_data_dir_inner(a_str(args, "target_dir"), st).await,
            ),
            "change_database_path" => to_value(
                commands::change_database_path_inner(a_str(args, "target_path"), st).await,
            ),
            "cleanup_old_data_dir" => to_value(
                commands::cleanup_old_data_dir_inner(a_str(args, "target_dir"), st).await,
            ),
            "open_data_dir" => to_value(commands::open_data_dir_inner(st).await),
            "recognize_work_intents" => to_value(
                commands::recognize_work_intents_inner(
                    a_opt_str(args, "date_from"),
                    a_opt_str(args, "date_to"),
                    a_opt_u32(args, "limit"),
                    st,
                )
                .await,
            ),
            "check_permissions" => to_value(commands::check_permissions().await),
            "open_permission_settings" => {
                to_value(commands::open_permission_settings(a_str(args, "permission")).await)
            }
            "is_screen_locked" => to_value(commands::is_screen_locked().await),
            "get_runtime_platform" => to_value(commands::get_runtime_platform().await),
            "get_platform" => to_value(Ok(commands::get_platform())),
            "set_dock_visibility" => to_value(Ok::<(), AppError>(())), // macOS 专属，Windows 无操作
            "save_background_image" => to_value(
                commands::save_background_image_inner(a_str(args, "data"), st).await,
            ),
            "get_background_image" => to_value(commands::get_background_image_inner(st).await),
            "clear_background_image" => to_value(commands::clear_background_image_inner(st).await),
            "is_autostart_enabled" => to_value(autostart_is_enabled()),
            "enable_autostart" => to_value(autostart_set(true, a_bool(args, "silent"))),
            "disable_autostart" => to_value(autostart_set(false, false)),
            other => Err(AppError::Unknown(format!("未知命令: {other}"))),
        }
    });

    match result {
        Ok(v) => json!({"ok": true, "value": v}),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

// ---------- 开机自启（独立于 Tauri AppHandle，注册表项与原实现一致） ----------

#[cfg(windows)]
fn autostart_manager(silent: bool) -> Result<auto_launch::AutoLaunch, AppError> {
    use auto_launch::{AutoLaunchBuilder, WindowsEnableMode};

    let current_exe = std::env::current_exe()
        .map_err(|e| AppError::Unknown(format!("无法获取当前执行路径: {e}")))?;
    let quoted = format!(
        "\"{}\"",
        current_exe.display().to_string().trim().trim_matches('"')
    );

    let mut builder = AutoLaunchBuilder::new();
    builder.set_app_name(AUTOSTART_APP_NAME);
    builder.set_app_path(&quoted);
    builder.set_windows_enable_mode(WindowsEnableMode::Dynamic);
    let launch_args: &[&str] = if silent {
        &[AUTOSTART_LAUNCH_ARG, "--hidden"]
    } else {
        &[AUTOSTART_LAUNCH_ARG]
    };
    builder.set_args(launch_args);
    builder
        .build()
        .map_err(|e| AppError::Unknown(format!("构建 Windows 开机自启管理器失败: {e}")))
}

#[cfg(windows)]
fn autostart_set(enabled: bool, silent: bool) -> Result<(), AppError> {
    let auto = autostart_manager(silent)?;
    if enabled {
        auto.enable()
            .map_err(|e| AppError::Unknown(format!("开启开机自启失败: {e}")))?;
    } else {
        auto.disable()
            .map_err(|e| AppError::Unknown(format!("关闭开机自启失败: {e}")))?;
    }
    Ok(())
}

#[cfg(windows)]
fn autostart_is_enabled() -> Result<bool, AppError> {
    let auto = autostart_manager(false)?;
    auto.is_enabled()
        .map_err(|e| AppError::Unknown(format!("查询开机自启状态失败: {e}")))
}

#[cfg(not(windows))]
fn autostart_set(_enabled: bool, _silent: bool) -> Result<(), AppError> {
    Err(AppError::Unknown("当前平台不支持注册表自启动".to_string()))
}

#[cfg(not(windows))]
fn autostart_is_enabled() -> Result<bool, AppError> {
    Err(AppError::Unknown("当前平台不支持注册表自启动".to_string()))
}
