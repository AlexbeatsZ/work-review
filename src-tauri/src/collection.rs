// 采集引擎：状态、数据目录解析、采集循环、小时摘要。框架无关（Tauri / FFI 共用）。
use crate::activity_classifier;
use crate::analysis;
use crate::config::AppConfig;
use crate::database;
use crate::database::Database;
use crate::AUTOSTART_LAUNCH_ARG;
use crate::error::Result;
use crate::events;
use crate::idle_detector;
use crate::monitor;
use crate::ocr;
use crate::privacy;
use crate::privacy::PrivacyFilter;
use crate::screen_lock;
use crate::screenshot;
use crate::screenshot::ScreenshotService;
use crate::storage::StorageManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct AppState {
    pub config: AppConfig,
    pub database: Database,
    pub privacy_filter: PrivacyFilter,
    pub screenshot_service: ScreenshotService,
    pub storage_manager: StorageManager,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
    pub is_recording: bool,
    pub is_paused: bool,
    pub generating_report: bool,
    /// 活动窗口缓存（时间戳 + 窗口信息），供后台截图循环复用
    pub cached_active_window: Option<(std::time::Instant, monitor::ActiveWindow)>,
}


#[derive(Serialize, Deserialize)]
pub struct DataDirPreference {
    data_dir: String,
}


pub fn launch_args_contain_autostart(args: &[String]) -> bool {
    args.iter().any(|arg| arg == AUTOSTART_LAUNCH_ARG)
}

pub fn args_include_explicit_hidden_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--hidden" | "--minimized"))
}

#[cfg(not(windows))]
pub fn launch_args_request_hidden_window(args: &[String]) -> bool {
    launch_args_contain_autostart(args) || args_include_explicit_hidden_flag(args)
}

pub fn should_hide_main_window_on_setup(_config: &AppConfig, launch_args: &[String]) -> bool {
    #[cfg(windows)]
    {
        // Windows 下注册表参数由 silent 选择动态写入：silent 模式带 `--hidden`，
        // show 模式只写 `--autostart`。显隐决策直接看 launch args，
        // 不再依赖 config.json，彻底消除前端忘保存带来的失同步。
        args_include_explicit_hidden_flag(launch_args)
    }

    #[cfg(not(windows))]
    {
        // macOS/Linux: tauri_plugin_autostart 的 args 在 plugin init 时固定，
        // 仍由 config.auto_start_silent 作为显隐信源。
        _config.auto_start
            && _config.auto_start_silent
            && launch_args_request_hidden_window(launch_args)
    }
}



pub struct WindowsSystemDialogRule {
    executable_names: &'static [&'static str],
    exact_window_texts: &'static [&'static str],
}

pub const WINDOWS_SYSTEM_DIALOG_RULES: &[WindowsSystemDialogRule] = &[
    WindowsSystemDialogRule {
        executable_names: &["taskmgr"],
        exact_window_texts: &[
            "task manager",
            "task manager (not responding)",
            "任务管理器",
            "任务管理器 (未响应)",
            "任务管理器（未响应）",
        ],
    },
    WindowsSystemDialogRule {
        executable_names: &["consent", "credentialuibroker"],
        exact_window_texts: &[
            "user account control",
            "windows security",
            "用户账户控制",
            "用户帐户控制",
            "windows 安全",
            "windows 安全中心",
        ],
    },
];

pub fn normalized_windows_system_window_text(value: &str) -> String {
    value.trim().to_lowercase()
}

pub fn windows_path_file_stem(path: &str) -> Option<String> {
    let file_name = path
        .rsplit(['\\', '/'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let stem = file_name
        .strip_suffix(".exe")
        .or_else(|| file_name.strip_suffix(".EXE"))
        .unwrap_or(file_name)
        .trim();

    if stem.is_empty() {
        None
    } else {
        Some(stem.to_lowercase())
    }
}

pub fn windows_executable_name(active_window: &monitor::ActiveWindow) -> Option<String> {
    active_window
        .executable_path
        .as_deref()
        .and_then(windows_path_file_stem)
        .or_else(|| {
            let normalized_name = active_window
                .app_name
                .trim()
                .trim_end_matches(".exe")
                .trim_end_matches(".EXE")
                .trim();

            if normalized_name.is_empty() {
                None
            } else {
                Some(normalized_name.to_lowercase())
            }
        })
}

pub fn matches_windows_system_dialog_rule(
    active_window: &monitor::ActiveWindow,
    rule: &WindowsSystemDialogRule,
) -> bool {
    let executable_name = windows_executable_name(active_window);
    if executable_name.as_deref().is_some_and(|name| {
        rule.executable_names
            .iter()
            .any(|candidate| candidate == &name)
    }) {
        return true;
    }

    let app_name = normalized_windows_system_window_text(&active_window.app_name);
    let window_title = normalized_windows_system_window_text(&active_window.window_title);
    let allow_exact_text_fallback =
        !app_name.is_empty() && (window_title.is_empty() || app_name == window_title);

    allow_exact_text_fallback
        && rule
            .exact_window_texts
            .iter()
            .any(|candidate| candidate == &app_name)
}

pub fn is_windows_system_dialog(active_window: &monitor::ActiveWindow) -> bool {
    WINDOWS_SYSTEM_DIALOG_RULES
        .iter()
        .any(|rule| matches_windows_system_dialog_rule(active_window, rule))
}

pub fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("work-review"))
        .unwrap_or_else(|| PathBuf::from("./data"))
}

pub fn data_dir_preference_path() -> PathBuf {
    dirs::config_dir()
        .map(|d| d.join("work-review").join("data-location.json"))
        .unwrap_or_else(|| PathBuf::from("./work-review-data-location.json"))
}

pub fn load_data_dir_preference() -> Option<PathBuf> {
    let path = data_dir_preference_path();
    let content = std::fs::read_to_string(path).ok()?;
    let preference: DataDirPreference = serde_json::from_str(&content).ok()?;
    let data_dir = preference.data_dir.trim();
    if data_dir.is_empty() {
        None
    } else {
        Some(PathBuf::from(data_dir))
    }
}

pub fn save_data_dir_preference(data_dir: &Path) -> std::io::Result<()> {
    let default_dir = default_data_dir();
    let preference_path = data_dir_preference_path();

    if data_dir == default_dir {
        if preference_path.exists() {
            std::fs::remove_file(preference_path)?;
        }
        return Ok(());
    }

    if let Some(parent) = preference_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&DataDirPreference {
        data_dir: data_dir.to_string_lossy().to_string(),
    })
    .map_err(std::io::Error::other)?;

    std::fs::write(preference_path, content)?;
    Ok(())
}

pub fn ensure_data_dir(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(path)?;
    Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

/// 获取数据目录
pub fn resolve_data_dir() -> PathBuf {
    let default_dir = default_data_dir();
    let preferred_dir = load_data_dir_preference().unwrap_or_else(|| default_dir.clone());

    match ensure_data_dir(&preferred_dir) {
        Ok(dir) => {
            migrate_legacy_data_dir(&dir);
            dir
        }
        Err(error) => {
            log::warn!("创建数据目录失败，回退默认目录: {error}");

            if preferred_dir != default_dir {
                if let Ok(dir) = ensure_data_dir(&default_dir) {
                    migrate_legacy_data_dir(&dir);
                    let _ = save_data_dir_preference(&dir);
                    return dir;
                }
            }

            let fallback_dir = PathBuf::from("./data");
            if let Err(fallback_error) = std::fs::create_dir_all(&fallback_dir) {
                log::warn!("创建兜底数据目录失败: {fallback_error}");
            }
            migrate_legacy_data_dir(&fallback_dir);
            fallback_dir
        }
    }
}

pub fn resolve_database_path(data_dir: &Path, config: &AppConfig) -> PathBuf {
    config
        .database_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("workreview.db"))
}

pub fn migrate_legacy_data_dir(target_dir: &PathBuf) {
    let legacy_dir = match std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("data")))
    {
        Some(path) => path,
        None => return,
    };

    if legacy_dir == *target_dir || !legacy_dir.exists() {
        return;
    }

    let target_has_data = target_dir.join("config.json").exists()
        || target_dir.join("workreview.db").exists()
        || target_dir.join("screenshots").exists();
    if target_has_data {
        return;
    }

    if let Err(error) = copy_dir_contents(&legacy_dir, target_dir, false) {
        log::warn!("迁移旧版数据目录失败: {error}");
    } else {
        log::info!("已将旧版数据目录迁移到稳定目录: {:?}", target_dir);
    }
}

pub fn copy_dir_contents(
    from: &Path,
    to: &Path,
    overwrite_existing: bool,
) -> std::io::Result<u64> {
    std::fs::create_dir_all(to)?;
    let mut copied_files = 0;

    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = to.join(entry.file_name());

        if source_path.is_dir() {
            copied_files += copy_dir_contents(&source_path, &target_path, overwrite_existing)?;
            continue;
        }

        if overwrite_existing || !target_path.exists() {
            std::fs::copy(&source_path, &target_path)?;
            copied_files += 1;
        }
    }

    Ok(copied_files)
}

/// 浏览器 URL 采集偶发失败时，尝试从最近同窗口标题的活动里恢复 URL。
/// 这是近似统计兜底：优先减少同一页面被切碎成多段或掉成 0 站点 0 页面。
pub fn recover_recent_browser_url(
    database: &Database,
    app_name: &str,
    window_title: &str,
    now_ts: i64,
    max_age_secs: i64,
) -> Option<String> {
    if !monitor::is_browser_app(app_name) || window_title.is_empty() {
        return None;
    }

    database
        .get_latest_activity_by_app_title(app_name, window_title)
        .ok()
        .flatten()
        .and_then(|activity| {
            let age = now_ts - activity.timestamp;
            if age <= max_age_secs {
                activity.browser_url.filter(|url| !url.is_empty())
            } else {
                None
            }
        })
}

pub fn resolve_activity_classification(
    config: &AppConfig,
    app_name: &str,
    window_title: &str,
    browser_url: Option<&str>,
) -> activity_classifier::ActivityClassification {
    let base_category = monitor::categorize_app_with_rules(
        &config.app_category_rules,
        app_name,
        window_title,
        &config.custom_categories,
    );
    let mut classification = activity_classifier::classify_activity_with_base_category(
        app_name,
        window_title,
        browser_url,
        &base_category,
    );

    if let Some(semantic_category) =
        monitor::find_website_semantic_override(&config.website_semantic_rules, browser_url)
    {
        classification.base_category = monitor::semantic_category_to_base_category(
            &semantic_category,
            &classification.base_category,
        );
        classification.semantic_category = semantic_category.clone();
        classification.confidence = classification.confidence.max(100);
        classification
            .evidence
            .push(format!("命中网站语义规则: {semantic_category}"));
    }

    classification
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordingLoopDecision {
    pub should_continue: bool,
    pub screenshot_interval: u64,
    pub reset_capture_clock: bool,
}

pub const ACTIVITY_INPUT_IDLE_SUSPECT_MINUTES_DEFAULT: u64 = 5;
pub const ACTIVITY_INPUT_IDLE_HARD_STOP_MINUTES: u64 = 20;
pub const ACTIVITY_INPUT_IDLE_HARD_STOP_SECS: u64 = ACTIVITY_INPUT_IDLE_HARD_STOP_MINUTES * 60;

pub fn should_confirm_idle(
    input_idle: bool,
    input_idle_seconds: u64,
    screenshots_enabled: bool,
    screenshot_confirmed: bool,
) -> bool {
    if !input_idle {
        return false;
    }

    // 长时间无输入时直接切断时长，避免后台程序或动态页面无限续时。
    if input_idle_seconds >= ACTIVITY_INPUT_IDLE_HARD_STOP_SECS {
        return true;
    }

    if screenshots_enabled {
        screenshot_confirmed
    } else {
        true
    }
}

pub fn previous_app_backfill_duration(
    app_changed: bool,
    duration_to_record: i64,
    was_input_idle: bool,
    is_confirmed_idle: bool,
) -> i64 {
    if !app_changed || duration_to_record <= 0 || was_input_idle || is_confirmed_idle {
        0
    } else {
        duration_to_record
    }
}

pub fn should_persist_merge_update(effective_duration: i64, keep_record_active: bool) -> bool {
    effective_duration > 0 || keep_record_active
}

pub fn resolve_previous_activity_to_backfill(
    state: &Arc<Mutex<AppState>>,
    previous_app_name: Option<&str>,
    previous_browser_url: Option<&str>,
    previous_window_title: Option<&str>,
) -> Option<database::Activity> {
    let Some(previous_app_name) = previous_app_name else {
        return None;
    };

    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(previous_url) = previous_browser_url.filter(|url| !url.is_empty()) {
        state_guard
            .database
            .get_latest_activity_by_url(previous_url)
            .ok()
            .flatten()
    } else if monitor::is_browser_app(previous_app_name) {
        previous_window_title
            .filter(|title| !title.is_empty())
            .and_then(|title| {
                state_guard
                    .database
                    .get_latest_activity_by_app_title(previous_app_name, title)
                    .ok()
                    .flatten()
            })
            .or_else(|| {
                state_guard
                    .database
                    .get_latest_activity_by_app(previous_app_name)
                    .ok()
                    .flatten()
            })
    } else {
        state_guard
            .database
            .get_latest_activity_by_app(previous_app_name)
            .ok()
            .flatten()
    }
}

pub fn backfill_previous_activity_if_needed(
    state: &Arc<Mutex<AppState>>,
    previous_activity: Option<&database::Activity>,
    duration_delta: i64,
    current_timestamp: i64,
    current_app_name: &str,
) {
    if duration_delta <= 0 {
        return;
    }

    let Some(previous_activity) = previous_activity else {
        return;
    };
    let Some(previous_id) = previous_activity.id else {
        return;
    };

    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
    let _ = state_guard.database.merge_activity(
        previous_id,
        duration_delta,
        None,
        &previous_activity.screenshot_path,
        current_timestamp,
    );
    log::debug!(
        "⏱️ 时长回补: {} +{}s (切换到 {})",
        previous_activity.app_name,
        duration_delta,
        current_app_name
    );
}

pub fn recording_loop_decision(
    is_recording: bool,
    is_paused: bool,
    screenshot_interval: u64,
) -> RecordingLoopDecision {
    if !is_recording || is_paused {
        RecordingLoopDecision {
            should_continue: false,
            screenshot_interval: 1,
            reset_capture_clock: true,
        }
    } else {
        RecordingLoopDecision {
            should_continue: true,
            screenshot_interval,
            reset_capture_clock: false,
        }
    }
}

pub fn monitoring_poll_interval_ms_for_platform(is_macos: bool) -> u64 {
    if is_macos {
        1500
    } else {
        500
    }
}

pub fn monitoring_poll_interval_ms() -> u64 {
    monitoring_poll_interval_ms_for_platform(cfg!(target_os = "macos"))
}

pub const ACTIVE_WINDOW_CACHE_MAX_AGE_MS: u64 = 1250;
pub const MIN_CAPTURE_INTERVAL_MS: u128 = 3000;
pub const MIN_BROWSER_CHANGE_CAPTURE_INTERVAL_MS: u128 = 1200;

pub fn reusable_cached_active_window(
    cached: Option<&(std::time::Instant, monitor::ActiveWindow)>,
    now: std::time::Instant,
) -> Option<monitor::ActiveWindow> {
    let (sampled_at, active_window) = cached?;
    let age = now.checked_duration_since(*sampled_at)?;

    if age > Duration::from_millis(ACTIVE_WINDOW_CACHE_MAX_AGE_MS) {
        return None;
    }

    Some(active_window.clone())
}

pub fn should_probe_browser_url_before_change_detection(
    app_name: &str,
    window_title: &str,
    last_app_name: Option<&str>,
    last_window_title: Option<&str>,
    current_browser_url: Option<&str>,
) -> bool {
    if !monitor::is_browser_app(app_name) || window_title.is_empty() {
        return false;
    }
    // 首次遇到浏览器窗口时（last 为 None），也需要探测 URL
    if last_app_name.is_none() || last_window_title.is_none() {
        return current_browser_url.is_none();
    }
    // 同窗口持续使用时，探测 URL 变化
    last_app_name == Some(app_name) && last_window_title == Some(window_title)
}

pub fn browser_change_capture_min_interval_ms(
    app_name: &str,
    title_changed: bool,
    url_changed: bool,
) -> u128 {
    if monitor::is_browser_app(app_name) && (title_changed || url_changed) {
        MIN_BROWSER_CHANGE_CAPTURE_INTERVAL_MS
    } else {
        MIN_CAPTURE_INTERVAL_MS
    }
}

pub fn should_refresh_browser_url_before_record(app_name: &str, window_title: &str) -> bool {
    monitor::is_browser_app(app_name) && !window_title.is_empty()
}

pub fn screen_lock_check_interval_ms_for_platform(is_macos: bool) -> u64 {
    if is_macos {
        5000
    } else {
        1000
    }
}

pub fn screen_lock_check_interval_ms() -> u64 {
    screen_lock_check_interval_ms_for_platform(cfg!(target_os = "macos"))
}

pub fn should_skip_transient_window(active_window: &monitor::ActiveWindow) -> bool {
    let app_lower = active_window.app_name.to_lowercase();
    matches!(
        app_lower.as_str(),
        "dock"
            | "systemuiserver"
            | "control center"
            | "spotlight"
            | "notificationcenter"
            | "loginwindow"
            | "screencaptureui"
            | "universalaccessauthwarn"
            | "windowmanager"
            | "wallpaper"
    )
}

pub fn should_skip_system_window(active_window: &monitor::ActiveWindow) -> bool {
    let is_sys = monitor::is_system_process(&active_window.app_name);
    let is_minimized_window = active_window.is_minimized;
    let is_explorer_shell = {
        let name_lower = active_window.app_name.to_lowercase();
        let name_trimmed = name_lower.trim_end_matches(".exe");
        (name_trimmed == "explorer" || name_trimmed == "file explorer")
            && active_window.window_title.is_empty()
    };
    // Windows 在 UAC / 任务管理器异常时，进程名可能退化成标题或受保护进程名，
    // 需要结合标题与可执行路径一起兜底过滤。
    let is_windows_system_dialog = is_windows_system_dialog(active_window);

    is_sys || is_minimized_window || is_explorer_shell || is_windows_system_dialog
}

// 系统托盘在 setup 钩子中使用 TrayIconBuilder 创建 (Tauri v2)


/// 使用 Arc<Mutex<AppState>> 而非 tauri::State，因为 State 无法在 async move 块中手动构造
pub async fn background_screenshot_task(state: Arc<Mutex<AppState>>) {
    // ===== 状态变量 =====
    let mut last_app_name: Option<String> = None;
    let mut last_app_window_title: Option<String> = None;
    let mut last_browser_url: Option<String> = None;

    let mut last_capture_time = std::time::Instant::now();

    // ===== 空闲检测器 =====
    // 先以输入空闲进入”疑似空闲”，再结合前台变化做短时保留。
    // 使用用户配置的空闲阈值，默认 5 分钟
    let idle_threshold_minutes = {
        let guard = state.lock().unwrap_or_else(|e| e.into_inner());
        guard.config.idle_threshold_minutes as u64
    };
    let idle_detector = idle_detector::IdleDetector::new(idle_threshold_minutes.max(1));
    let mut last_idle_log_time = std::time::Instant::now();
    let mut is_currently_idle = false; // 当前是否处于空闲状态

    let poll_interval_ms = monitoring_poll_interval_ms();

    // OCR 并发限制：最多 2 个 OCR 任务同时运行，防止任务堆积消耗内存
    let ocr_semaphore = Arc::new(tokio::sync::Semaphore::new(2));

    // 合并路径的截图哈希去重：用 Arc 共享给异步任务，避免 static 跨活动污染
    let merge_screenshot_hash = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // 锁屏检测器（无内部状态，复用同一实例避免重复分配）
    let screen_lock_monitor = screen_lock::ScreenLockMonitor::new();
    let mut last_screen_lock_check = std::time::Instant::now()
        .checked_sub(Duration::from_millis(screen_lock_check_interval_ms()))
        .unwrap_or_else(std::time::Instant::now);
    let mut cached_screen_locked = false;

    loop {
        // 首先检查录制状态并获取配置
        let decision = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            recording_loop_decision(
                state_guard.is_recording,
                state_guard.is_paused,
                state_guard.config.screenshot_interval,
            )
        };

        if decision.reset_capture_clock {
            last_capture_time = std::time::Instant::now();
        }

        if !decision.should_continue {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        if last_screen_lock_check.elapsed()
            >= Duration::from_millis(screen_lock_check_interval_ms())
        {
            cached_screen_locked = screen_lock_monitor.is_locked();
            last_screen_lock_check = std::time::Instant::now();
        }

        // 检测屏幕锁定状态，锁屏时不统计时长
        if cached_screen_locked {
            log::info!("🔒 屏幕已锁定，暂停活动统计");
            last_app_name = None; // 重置应用状态，解锁后视为新开始
            last_capture_time = std::time::Instant::now(); // 重置截图计时，避免解锁后累加锁屏时长
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        let screenshot_interval = decision.screenshot_interval;

        // 轮询检测活动窗口。
        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;

        // 获取当前活动窗口
        // 失败原因：Windows 睡眠/待机/UAC 时无前台窗口、macOS 权限不足等
        // 此时重置计时器，避免累积的时长被错误归属到下一个真实应用
        let active_window_now = std::time::Instant::now();
        let cached_active_window = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            reusable_cached_active_window(
                state_guard.cached_active_window.as_ref(),
                active_window_now,
            )
        };
        let mut active_window = if let Some(window) = cached_active_window {
            // 头像循环缓存不含浏览器 URL，如果是浏览器窗口则跳过缓存重新获取
            if monitor::is_browser_app(&window.app_name) && window.browser_url.is_none() {
                match monitor::get_active_window() {
                    Ok(w) => w,
                    Err(_) => window,
                }
            } else {
                window
            }
        } else {
            match monitor::get_active_window() {
                Ok(w) => w,
                Err(_) => {
                    last_capture_time = std::time::Instant::now();
                    continue;
                }
            }
        };

        // 再次检查状态
        let should_capture = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            state_guard.is_recording && !state_guard.is_paused
        };

        if !should_capture {
            continue;
        }

        // macOS 系统进程在用户切换应用、点击 Dock 时会短暂成为前台应用
        // 跳过这些进程避免它们偷走其他应用的使用时长
        // 不更新 last_app_name，时长会在下一个正常轮询中通过 elapsed_secs 自然回收
        {
            if should_skip_transient_window(&active_window) {
                log::debug!("跳过系统瞬态进程: {}", active_window.app_name);
                last_app_name = None;
                last_app_window_title = None;
                last_browser_url = None;
                last_capture_time = std::time::Instant::now();
                continue;
            }
        }

        // 跳过系统 shell / 锁屏 / 桌面进程，避免睡眠/唤醒时累积虚假时长
        // 注意 explorer 特殊处理：有窗口标题时是文件管理器，应该记录
        {
            if should_skip_system_window(&active_window) {
                log::debug!(
                    "跳过系统/桌面窗口: {} (title={}, minimized={})",
                    active_window.app_name,
                    active_window.window_title,
                    active_window.is_minimized
                );
                last_app_name = None;
                last_app_window_title = None;
                last_browser_url = None;
                last_capture_time = std::time::Instant::now();
                continue;
            }
        }

        let should_probe_browser_url = should_probe_browser_url_before_change_detection(
            &active_window.app_name,
            &active_window.window_title,
            last_app_name.as_deref(),
            last_app_window_title.as_deref(),
            active_window.browser_url.as_deref(),
        );
        if should_probe_browser_url {
            if let Some(resolved_url) = monitor::resolve_browser_url_for_window(
                &active_window.app_name,
                &active_window.window_title,
            ) {
                if last_browser_url.as_deref() != Some(resolved_url.as_str()) {
                    log::debug!(
                        "浏览器 URL 预探测命中: {} | {} -> {}",
                        active_window.app_name,
                        active_window.window_title,
                        resolved_url
                    );
                }
                active_window.browser_url = Some(resolved_url);
            }
        }

        // 浏览器 URL 存在瞬时采集失败时，尽量复用同窗口最近一次成功值，减少统计断裂。
        const BROWSER_URL_STICKY_GAP_SECS: i64 = 120;
        if active_window.browser_url.is_none()
            && monitor::is_browser_app(&active_window.app_name)
            && !active_window.window_title.is_empty()
        {
            let now_ts = chrono::Local::now().timestamp();

            let recovered_url = if last_app_name.as_deref() == Some(active_window.app_name.as_str())
                && last_app_window_title.as_deref() == Some(active_window.window_title.as_str())
            {
                last_browser_url.clone()
            } else {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                recover_recent_browser_url(
                    &state_guard.database,
                    &active_window.app_name,
                    &active_window.window_title,
                    now_ts,
                    BROWSER_URL_STICKY_GAP_SECS,
                )
            };

            if let Some(recovered_url) = recovered_url {
                log::debug!(
                    "恢复浏览器 URL: {} | {} -> {}",
                    active_window.app_name,
                    active_window.window_title,
                    recovered_url
                );
                active_window.browser_url = Some(recovered_url);
            }
        }

        // ===== 检测应用切换 =====
        let previous_window_title = last_app_window_title.clone();
        let previous_browser_url = last_browser_url.clone();

        let mut url_changed = match (&last_browser_url, &active_window.browser_url) {
            (Some(l), Some(r)) => l != r,
            (None, None) => false,
            _ => true,
        };

        // 只有当两个标题不同时才算切换
        let title_changed = match (&last_app_window_title, &active_window.window_title) {
            (Some(last_title), active_title) => last_title != active_title,
            (None, _) => true,
        };

        let mut app_changed = match &last_app_name {
            Some(last) => last != &active_window.app_name || url_changed || title_changed,
            None => true,
        };
        let capture_min_interval_ms = browser_change_capture_min_interval_ms(
            &active_window.app_name,
            title_changed,
            url_changed,
        );

        // 计算距离上次截图的时间
        let elapsed_since_capture = last_capture_time.elapsed();
        let elapsed_secs = elapsed_since_capture.as_secs();

        // ===== 应用切换日志 =====
        if app_changed && last_app_name.is_some() {
            log::info!(
                "📊 应用切换: {} [{}] → {} [{}]",
                last_app_name.as_deref().unwrap_or("无"),
                previous_window_title.as_deref().unwrap_or(""),
                &active_window.app_name,
                &active_window.window_title,
            );
        }

        // ===== 空闲检测第一阶段：键鼠活动检查 =====
        let input_idle_seconds = idle_detector.get_idle_seconds();
        let input_idle = input_idle_seconds >= idle_threshold_minutes * 60;

        let was_input_idle = is_currently_idle;
        // 每 30 秒打印一次空闲状态日志（避免刷屏）
        if last_idle_log_time.elapsed() >= Duration::from_secs(30) {
            if input_idle != is_currently_idle {
                if input_idle {
                    log::info!("⏸️  键鼠超时，等待截图确认空闲状态...");
                } else {
                    log::info!("▶️  检测到用户活动，恢复正常记录");
                    idle_detector.reset();
                }
            }
            last_idle_log_time = std::time::Instant::now();
        }
        is_currently_idle = input_idle;

        // ===== 判断是否截图 =====
        // 1. 定时触发：到达配置的间隔时间
        // 2. 应用切换触发：满足最小间隔
        let should_take_screenshot = if elapsed_secs >= screenshot_interval {
            log::debug!("定时截图触发");
            true
        } else if app_changed && elapsed_since_capture.as_millis() >= capture_min_interval_ms {
            if capture_min_interval_ms < MIN_CAPTURE_INTERVAL_MS {
                log::debug!("浏览器导航截图触发");
            } else {
                log::debug!("应用切换截图触发");
            }
            true
        } else {
            false
        };

        // 保存 app_name 副本供浮动窗口检测使用（在 move 之前）
        let frontmost_app_name = active_window.app_name.clone();

        if !should_take_screenshot {
            // 如果是因为冷却时间未到而没有截图，但应用/标签页实际上已经变化了
            // 那么我们不要更新 last_* 变量，这样下一个轮询周期 app_changed 仍然为 true
            if !app_changed {
                last_app_name = Some(active_window.app_name.clone());
                last_app_window_title = Some(active_window.window_title.clone());
                last_browser_url = active_window.browser_url.clone();
            }
            continue;
        }

        if should_refresh_browser_url_before_record(
            &active_window.app_name,
            &active_window.window_title,
        ) {
            if let Some(resolved_url) = monitor::resolve_browser_url_for_window(
                &active_window.app_name,
                &active_window.window_title,
            ) {
                if active_window.browser_url.as_deref() != Some(resolved_url.as_str()) {
                    log::debug!(
                        "浏览器 URL 落库前刷新: {} | {} -> {}",
                        active_window.app_name,
                        active_window.window_title,
                        resolved_url
                    );
                }
                active_window.browser_url = Some(resolved_url);
            }
            url_changed = match (&last_browser_url, &active_window.browser_url) {
                (Some(l), Some(r)) => l != r,
                (None, None) => false,
                _ => true,
            };
            app_changed = match &last_app_name {
                Some(last) => last != &active_window.app_name || url_changed || title_changed,
                None => true,
            };
        }

        // 保存切换前的应用名，用于时长归属修正
        let previous_app_name = if app_changed {
            last_app_name.clone()
        } else {
            None
        };

        // 取决定截图后，才更新上一个应用的信息
        last_app_name = Some(active_window.app_name.clone());
        last_app_window_title = Some(active_window.window_title.clone());
        last_browser_url = active_window.browser_url.clone();

        // 更新截图时间
        last_capture_time = std::time::Instant::now();

        // 使用距离上次截图的实际经过时间作为本次记录的时长
        // 而非固定的轮询间隔，避免截图间隔大于轮询间隔时丢失时长
        let (privacy_action, duration_to_record) = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            let action = state_guard.privacy_filter.check_privacy_full(
                &active_window.app_name,
                &active_window.window_title,
                active_window.browser_url.as_deref(),
            );
            // elapsed_secs 是距离上次截图的真实秒数，确保时长不丢失
            let duration = elapsed_secs.max(1) as i64;
            (action, duration)
        };
        // 锁已释放

        let current_timestamp = chrono::Local::now().timestamp();
        let previous_activity_to_backfill = if app_changed {
            resolve_previous_activity_to_backfill(
                &state,
                previous_app_name.as_deref(),
                previous_browser_url.as_deref(),
                previous_window_title.as_deref(),
            )
        } else {
            None
        };
        let adjusted_duration = if app_changed {
            0i64
        } else {
            duration_to_record
        };

        use privacy::PrivacyAction;
        let result: Option<database::Activity> = match privacy_action {
            PrivacyAction::Skip => {
                log::debug!(
                    "完全跳过: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                None
            }
            PrivacyAction::Anonymize => {
                log::debug!(
                    "内容脱敏: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                let classification = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    crate::resolve_activity_classification(
                        &state_guard.config,
                        &active_window.app_name,
                        &active_window.window_title,
                        active_window.browser_url.as_deref(),
                    )
                };
                let anonymized_is_confirmed_idle =
                    should_confirm_idle(input_idle, input_idle_seconds, false, false);
                let previous_effective_duration = previous_app_backfill_duration(
                    app_changed,
                    duration_to_record,
                    was_input_idle,
                    anonymized_is_confirmed_idle,
                );
                backfill_previous_activity_if_needed(
                    &state,
                    previous_activity_to_backfill.as_ref(),
                    previous_effective_duration,
                    current_timestamp,
                    &active_window.app_name,
                );
                let effective_duration = if anonymized_is_confirmed_idle {
                    log::debug!("空闲确认: 脱敏活动跳过时长记录");
                    0
                } else {
                    adjusted_duration
                };

                if effective_duration <= 0 && !app_changed {
                    None
                } else {
                    let activity = database::Activity {
                        id: None,
                        timestamp: current_timestamp,
                        app_name: active_window.app_name,
                        window_title: "[内容已脱敏]".to_string(),
                        screenshot_path: String::new(),
                        ocr_text: None,
                        category: classification.base_category,
                        duration: effective_duration,
                        browser_url: None,
                        executable_path: active_window.executable_path,
                        semantic_category: Some(classification.semantic_category),
                        semantic_confidence: Some(i32::from(classification.confidence)),
                        ..database::Activity::default()
                    };

                    // 短暂获取锁写入数据库
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    match state_guard.database.insert_activity(&activity) {
                        Ok(_) => Some(activity),
                        Err(e) => {
                            log::error!("保存活动记录失败: {e}");
                            None
                        }
                    }
                }
            }
            PrivacyAction::Record => {
                let (classification, screenshots_enabled) = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    (
                        crate::resolve_activity_classification(
                            &state_guard.config,
                            &active_window.app_name,
                            &active_window.window_title,
                            active_window.browser_url.as_deref(),
                        ),
                        state_guard.config.storage.screenshots_enabled,
                    )
                };
                let category = classification.base_category.clone();

                // 先检查是否有可合并的记录（在截屏之前判断，避免不必要的截图保存）
                let latest_activity = {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(url) = active_window
                        .browser_url
                        .as_deref()
                        .filter(|url| !url.is_empty())
                    {
                        state_guard
                            .database
                            .get_latest_activity_by_url(url)
                            .ok()
                            .flatten()
                    } else if monitor::is_browser_app(&active_window.app_name)
                        && !active_window.window_title.is_empty()
                    {
                        state_guard
                            .database
                            .get_latest_activity_by_app_title(
                                &active_window.app_name,
                                &active_window.window_title,
                            )
                            .ok()
                            .flatten()
                    } else {
                        state_guard
                            .database
                            .get_latest_activity_by_app(&active_window.app_name)
                            .ok()
                            .flatten()
                    }
                };

                // "Unknown" 进程名不做合并：无法区分是哪个进程，强制新建
                // 防止所有识别失败的进程时长累积到同一条记录导致统计失真
                // 时间间隔超过 10 分钟也不合并：上午/下午用同一个 app 属于不同工作段
                const MERGE_GAP_SECS: i64 = 600;
                let is_merge = if let Some(ref latest) = latest_activity {
                    let mut merge = active_window.app_name != "Unknown"
                        && (current_timestamp - latest.timestamp) <= MERGE_GAP_SECS;

                    // 如果由于某种原因 browser_url 获取失败，但它确实是一个浏览器
                    // 我们必须强制让 window_title 完全相同才能合并，否则不同标签页的切换会被死死合并成一条记录。
                    if merge
                        && active_window.browser_url.is_none()
                        && monitor::is_browser_app(&active_window.app_name)
                        && latest.window_title != active_window.window_title
                    {
                        merge = false;
                    }

                    merge
                } else {
                    false
                };

                if is_merge {
                    // === 合并路径：不保存截图，只做 OCR ===
                    let latest = latest_activity.unwrap();
                    let latest_id = match latest.id {
                        Some(id) => id,
                        None => {
                            log::error!("合并活动记录缺少 id，跳过");
                            continue;
                        }
                    };
                    let previous_screenshot_path = latest.screenshot_path.clone();

                    // 截屏到内存，保存为临时文件供 OCR 使用
                    let screenshot_result = if screenshots_enabled {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        state_guard
                            .screenshot_service
                            .capture_for_window(Some(&active_window))
                            .ok()
                    } else {
                        None
                    };

                    // ===== 空闲检测第二阶段：截图哈希确认 =====
                    // 只有键鼠超时时才检查屏幕变化，避免正常使用时的额外计算
                    let screenshot_idle = if input_idle {
                        if let Some(ref screenshot) = screenshot_result {
                            let hash = screenshot::ScreenshotService::calculate_image_hash(
                                &screenshot.path,
                            )
                            .unwrap_or(0);
                            idle_detector.confirm_idle_with_hash(hash)
                        } else {
                            false
                        }
                    } else {
                        // 有键鼠活动，重置空闲检测器
                        idle_detector.reset();
                        false
                    };
                    let is_confirmed_idle = should_confirm_idle(
                        input_idle,
                        input_idle_seconds,
                        screenshots_enabled,
                        screenshot_idle,
                    );
                    let previous_effective_duration = previous_app_backfill_duration(
                        app_changed,
                        duration_to_record,
                        was_input_idle,
                        is_confirmed_idle,
                    );
                    backfill_previous_activity_if_needed(
                        &state,
                        previous_activity_to_backfill.as_ref(),
                        previous_effective_duration,
                        current_timestamp,
                        &active_window.app_name,
                    );

                    // 如果确认空闲，跳过时长记录
                    let effective_duration = if is_confirmed_idle {
                        log::debug!("空闲确认: 跳过本次时长记录");
                        0
                    } else {
                        adjusted_duration
                    };

                    // 合并记录（不更新 screenshot_path，保留活动创建时的原始截图）
                    // 即使 effective_duration 为 0，也需要更新时间戳以保持记录活跃
                    let (latest_archive_path, ocr_input_path, temporary_ocr_source_path) =
                        if let Some(ref screenshot) = screenshot_result {
                            (
                                Some(screenshot.path.clone()),
                                screenshot
                                    .ocr_source_path
                                    .clone()
                                    .unwrap_or_else(|| screenshot.path.clone()),
                                screenshot
                                    .ocr_source_path
                                    .clone()
                                    .filter(|path| path != &screenshot.path),
                            )
                        } else {
                            (None, PathBuf::new(), None)
                        };

                    let persisted_screenshot_path = previous_screenshot_path.clone();
                    let mut persisted_duration = latest.duration;

                    if should_persist_merge_update(effective_duration, true) {
                        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                        match state_guard.database.merge_activity(
                            latest_id,
                            effective_duration,
                            None,
                            &previous_screenshot_path,
                            current_timestamp,
                        ) {
                            Ok(_) => {
                                persisted_duration += effective_duration;
                                log::info!(
                                    "✅ 合并成功: {} (id={}, 新时长={}s)",
                                    active_window.app_name,
                                    latest_id,
                                    latest.duration + effective_duration
                                );
                            }
                            Err(e) => {
                                log::error!("合并活动记录失败: {e}");
                            }
                        }
                    }

                    // 对截图执行 OCR；若已成功合并，则保留最新截图并清理旧截图
                    if let Some(screenshot) = screenshot_result {
                        let latest_capture_path =
                            latest_archive_path.unwrap_or_else(|| screenshot.path.clone());
                        let state_clone = state.clone();
                        let data_dir_clone = {
                            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                            state_guard.data_dir.clone()
                        };

                        let ocr_sem = ocr_semaphore.clone();
                        let merge_hash = merge_screenshot_hash.clone();

                        tokio::spawn(async move {
                            use std::sync::atomic::Ordering;

                            // 非阻塞获取 permit，满载时跳过 OCR 避免任务堆积
                            let _permit = match ocr_sem.try_acquire_owned() {
                                Ok(p) => p,
                                Err(_) => {
                                    log::debug!("OCR 并发已满，跳过合并路径 OCR");
                                    if let Some(temp_path) = temporary_ocr_source_path.clone() {
                                        let _ = std::fs::remove_file(&temp_path);
                                    }
                                    let _ = std::fs::remove_file(&latest_capture_path);
                                    return;
                                }
                            };

                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                            // 计算哈希做去重判断
                            let current_hash = screenshot::ScreenshotService::calculate_image_hash(
                                &latest_capture_path,
                            )
                            .unwrap_or(0);
                            let last_hash = merge_hash.swap(current_hash, Ordering::Relaxed);

                            let should_ocr = if last_hash != 0 {
                                let similarity = screenshot::ScreenshotService::hash_similarity(
                                    last_hash,
                                    current_hash,
                                );
                                if similarity > 90 {
                                    log::debug!("合并截图相似度 {similarity}%，跳过 OCR");
                                    false
                                } else {
                                    log::debug!("合并截图相似度 {similarity}%，执行 OCR");
                                    true
                                }
                            } else {
                                true
                            };

                            if should_ocr {
                                let ocr_service = ocr::OcrService::new(&data_dir_clone);
                                if let Ok(Some(ocr_result)) =
                                    ocr_service.extract_text(&ocr_input_path)
                                {
                                    if !ocr_result.text.is_empty() {
                                        let filtered_text =
                                            ocr::filter_sensitive_text(&ocr_result.text);
                                        if let Ok(state_guard) = state_clone.lock() {
                                            let _ = state_guard.database.update_activity_ocr(
                                                latest_id,
                                                Some(filtered_text),
                                            );
                                            log::info!(
                                                "OCR 完成(合并): 活动 {} 识别到 {} 个字符",
                                                latest_id,
                                                ocr_result.text.len()
                                            );
                                        }
                                    }
                                }
                            }

                            if let Some(temp_path) = temporary_ocr_source_path {
                                let _ = std::fs::remove_file(&temp_path);
                            }

                            let _ = std::fs::remove_file(&latest_capture_path);
                            log::debug!("已删除仅用于合并 OCR 的临时截图: {latest_capture_path:?}");
                        });
                    }

                    Some(database::Activity {
                        id: Some(latest_id),
                        timestamp: current_timestamp,
                        app_name: active_window.app_name.clone(),
                        window_title: active_window.window_title,
                        screenshot_path: persisted_screenshot_path,
                        ocr_text: None,
                        category,
                        duration: persisted_duration,
                        browser_url: active_window.browser_url,
                        executable_path: active_window.executable_path,
                        semantic_category: Some(classification.semantic_category.clone()),
                        semantic_confidence: Some(i32::from(classification.confidence)),
                        ..database::Activity::default()
                    })
                } else {
                    // === 新建路径：正常截屏并保存 ===
                    if screenshots_enabled {
                        let screenshot_result = {
                            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                            state_guard
                                .screenshot_service
                                .capture_for_window(Some(&active_window))
                        };

                        match screenshot_result {
                            Ok(screenshot_result) => {
                                // ===== 空闲检测第二阶段：截图哈希确认 =====
                                let screenshot_idle = if input_idle {
                                    let hash = screenshot::ScreenshotService::calculate_image_hash(
                                        &screenshot_result.path,
                                    )
                                    .unwrap_or(0);
                                    idle_detector.confirm_idle_with_hash(hash)
                                } else {
                                    idle_detector.reset();
                                    false
                                };
                                let is_confirmed_idle = should_confirm_idle(
                                    input_idle,
                                    input_idle_seconds,
                                    screenshots_enabled,
                                    screenshot_idle,
                                );
                                let previous_effective_duration = previous_app_backfill_duration(
                                    app_changed,
                                    duration_to_record,
                                    was_input_idle,
                                    is_confirmed_idle,
                                );
                                backfill_previous_activity_if_needed(
                                    &state,
                                    previous_activity_to_backfill.as_ref(),
                                    previous_effective_duration,
                                    current_timestamp,
                                    &active_window.app_name,
                                );

                                // 如果确认空闲，跳过时长记录（但仍创建活动记录以保持截图）
                                let effective_duration = if is_confirmed_idle {
                                    log::debug!("空闲确认: 新活动时长设为 0");
                                    0
                                } else {
                                    adjusted_duration
                                };

                                let (
                                    relative_path,
                                    archive_path,
                                    ocr_input_path,
                                    temporary_ocr_source_path,
                                    data_dir_clone,
                                ) = {
                                    let state_guard =
                                        state.lock().unwrap_or_else(|e| e.into_inner());
                                    (
                                        state_guard
                                            .screenshot_service
                                            .get_relative_path(&screenshot_result.path),
                                        screenshot_result.path.clone(),
                                        screenshot_result
                                            .ocr_source_path
                                            .clone()
                                            .unwrap_or_else(|| screenshot_result.path.clone()),
                                        screenshot_result
                                            .ocr_source_path
                                            .clone()
                                            .filter(|path| path != &screenshot_result.path),
                                        state_guard.data_dir.clone(),
                                    )
                                };

                                let activity = database::Activity {
                                    id: None,
                                    timestamp: screenshot_result.timestamp,
                                    app_name: active_window.app_name.clone(),
                                    window_title: active_window.window_title,
                                    screenshot_path: relative_path.clone(),
                                    ocr_text: None,
                                    category,
                                    duration: effective_duration,
                                    browser_url: active_window.browser_url,
                                    executable_path: active_window.executable_path,
                                    semantic_category: Some(
                                        classification.semantic_category.clone(),
                                    ),
                                    semantic_confidence: Some(i32::from(classification.confidence)),
                                    ..database::Activity::default()
                                };

                                let inserted = {
                                    let state_guard =
                                        state.lock().unwrap_or_else(|e| e.into_inner());
                                    state_guard.database.insert_activity(&activity)
                                };

                                match inserted {
                                    Ok(activity_id) => {
                                        log::info!(
                                            "📝 新建活动: {} (id={})",
                                            active_window.app_name,
                                            activity_id
                                        );

                                        // 异步 OCR（新建活动的截图已保存，不删除）
                                        let state_clone = state.clone();
                                        let ocr_sem = ocr_semaphore.clone();
                                        tokio::spawn(async move {
                                            // 非阻塞获取 permit，满载时跳过 OCR
                                            let _permit = match ocr_sem.try_acquire_owned() {
                                                Ok(p) => p,
                                                Err(_) => {
                                                    log::debug!("OCR 并发已满，跳过新建路径 OCR");
                                                    if let Some(temp_path) =
                                                        temporary_ocr_source_path.clone()
                                                    {
                                                        let _ = std::fs::remove_file(&temp_path);
                                                    }
                                                    return;
                                                }
                                            };

                                            tokio::time::sleep(tokio::time::Duration::from_secs(1))
                                                .await;

                                            let ocr_service = ocr::OcrService::new(&data_dir_clone);

                                            if let Ok(Some(ocr_result)) =
                                                ocr_service.extract_text(&ocr_input_path)
                                            {
                                                if !ocr_result.text.is_empty() {
                                                    let filtered_text = ocr::filter_sensitive_text(
                                                        &ocr_result.text,
                                                    );
                                                    if let Ok(state_guard) = state_clone.lock() {
                                                        let _ = state_guard
                                                            .database
                                                            .update_activity_ocr(
                                                                activity_id,
                                                                Some(filtered_text),
                                                            );
                                                        log::info!(
                                                        "OCR 完成(新建): 活动 {} 识别到 {} 个字符",
                                                        activity_id,
                                                        ocr_result.text.len()
                                                    );
                                                    }
                                                }
                                            }

                                            if let Some(temp_path) = temporary_ocr_source_path {
                                                let _ = std::fs::remove_file(&temp_path);
                                            }
                                        });

                                        Some(database::Activity {
                                            id: Some(activity_id),
                                            ..activity
                                        })
                                    }
                                    Err(e) => {
                                        log::error!("保存活动记录失败: {e}");
                                        let _ = std::fs::remove_file(&archive_path);
                                        if let Some(temp_path) = temporary_ocr_source_path {
                                            let _ = std::fs::remove_file(&temp_path);
                                        }
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("截屏失败: {e}");
                                None
                            }
                        }
                    } else {
                        let is_confirmed_idle = should_confirm_idle(
                            input_idle,
                            input_idle_seconds,
                            screenshots_enabled,
                            false,
                        );
                        let previous_effective_duration = previous_app_backfill_duration(
                            app_changed,
                            duration_to_record,
                            was_input_idle,
                            is_confirmed_idle,
                        );
                        backfill_previous_activity_if_needed(
                            &state,
                            previous_activity_to_backfill.as_ref(),
                            previous_effective_duration,
                            current_timestamp,
                            &active_window.app_name,
                        );
                        let effective_duration = if is_confirmed_idle {
                            log::debug!("关闭截图后按输入空闲判定，新活动时长设为 0");
                            0
                        } else {
                            adjusted_duration
                        };

                        let activity = database::Activity {
                            id: None,
                            timestamp: current_timestamp,
                            app_name: active_window.app_name.clone(),
                            window_title: active_window.window_title,
                            screenshot_path: String::new(),
                            ocr_text: None,
                            category,
                            duration: effective_duration,
                            browser_url: active_window.browser_url,
                            executable_path: active_window.executable_path,
                            semantic_category: Some(classification.semantic_category.clone()),
                            semantic_confidence: Some(i32::from(classification.confidence)),
                            ..database::Activity::default()
                        };

                        let inserted = {
                            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                            state_guard.database.insert_activity(&activity)
                        };

                        match inserted {
                            Ok(activity_id) => {
                                log::info!(
                                    "📝 新建无截图活动: {} (id={})",
                                    active_window.app_name,
                                    activity_id
                                );
                                Some(database::Activity {
                                    id: Some(activity_id),
                                    ..activity
                                })
                            }
                            Err(e) => {
                                log::error!("保存无截图活动记录失败: {e}");
                                None
                            }
                        }
                    }
                }
            }
        };

        // 发送事件到前端
        if let Some(activity) = result {
            events::emit_event("screenshot-taken", &activity);
        }

        // ===== 浮动窗口（PiP 画中画）检测 =====
        // 检测 layer > 0 的浮动窗口（如视频小窗），为它们记录使用时长
        // 浮动窗口不截图（截图已由主活动管理），仅记录时长
        let overlay_windows = monitor::get_overlay_windows(&frontmost_app_name);
        for ow in &overlay_windows {
            // 隐私检查
            let ow_privacy = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard
                    .privacy_filter
                    .check_privacy(&ow.app_name, &ow.window_title)
            };

            if ow_privacy == privacy::PrivacyAction::Skip {
                log::debug!("浮动窗口跳过(隐私): {}", ow.app_name);
                continue;
            }

            let overlay_is_confirmed_idle =
                should_confirm_idle(input_idle, input_idle_seconds, false, false);
            if overlay_is_confirmed_idle {
                log::debug!("浮动窗口空闲确认，跳过时长记录: {}", ow.app_name);
                continue;
            }

            let classification = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                crate::resolve_activity_classification(
                    &state_guard.config,
                    &ow.app_name,
                    &ow.window_title,
                    ow.browser_url.as_deref(),
                )
            };
            let ow_category = classification.base_category.clone();
            let current_ts = chrono::Local::now().timestamp();
            let ow_duration = poll_interval_ms.div_ceil(1000) as i64;

            // 查找该应用的最近活动记录，尝试合并
            let latest = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard
                    .database
                    .get_latest_activity_by_app(&ow.app_name)
                    .ok()
                    .flatten()
            };

            const OW_MERGE_GAP_SECS: i64 = 600;
            let can_merge = if let Some(ref act) = latest {
                ow.app_name != "Unknown" && (current_ts - act.timestamp) <= OW_MERGE_GAP_SECS
            } else {
                false
            };

            if can_merge {
                let act = latest.unwrap();
                if let Some(act_id) = act.id {
                    let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                    match state_guard.database.merge_activity(
                        act_id,
                        ow_duration,
                        None,
                        &act.screenshot_path,
                        current_ts,
                    ) {
                        Ok(_) => {
                            log::info!(
                                "🪟 浮动窗口合并: {} (id={}, +{}s, 总{}s)",
                                ow.app_name,
                                act_id,
                                ow_duration,
                                act.duration + ow_duration
                            );
                        }
                        Err(e) => log::error!("浮动窗口合并失败: {e}"),
                    }
                }
            } else {
                // 新建活动记录（无截图）
                let ow_title = if ow_privacy == privacy::PrivacyAction::Anonymize {
                    "[内容已脱敏]".to_string()
                } else {
                    ow.window_title.clone()
                };

                let activity = database::Activity {
                    id: None,
                    timestamp: current_ts,
                    app_name: ow.app_name.clone(),
                    window_title: ow_title,
                    screenshot_path: String::new(),
                    ocr_text: None,
                    category: ow_category,
                    duration: ow_duration,
                    browser_url: None,
                    executable_path: ow.executable_path.clone(),
                    semantic_category: Some(classification.semantic_category),
                    semantic_confidence: Some(i32::from(classification.confidence)),
                    ..database::Activity::default()
                };

                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                match state_guard.database.insert_activity(&activity) {
                    Ok(id) => {
                        log::info!(
                            "🪟 浮动窗口新建: {} (id={}, {}s)",
                            ow.app_name,
                            id,
                            ow_duration
                        );
                    }
                    Err(e) => log::error!("浮动窗口记录失败: {e}"),
                }
            }
        }
    }
}

/// 小时摘要生成任务
/// 每小时检查一次，为上一个完整小时生成摘要

/// 为指定日期和小时生成并保存摘要
pub fn generate_and_save_summary(state: &Arc<Mutex<AppState>>, date: &str, hour: i32) {
    use analysis::hourly::{generate_fallback_summary, HourlyStats};

    let activities = {
        let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
        state_guard.database.get_hourly_activities(date, hour)
    };

    match activities {
        Ok(acts) if !acts.is_empty() => {
            let stats = HourlyStats::from_activities(date, hour, acts);
            let summary = generate_fallback_summary(&stats);

            let hourly_summary = database::HourlySummary {
                id: None,
                date: date.to_string(),
                hour,
                summary,
                main_apps: stats.get_main_apps().join(", "),
                activity_count: stats.activity_count,
                total_duration: stats.total_duration,
                representative_screenshots: Some(
                    serde_json::to_string(&stats.representative_screenshots).unwrap_or_default(),
                ),
                created_at: chrono::Local::now().timestamp(),
            };

            let save_result = {
                let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
                state_guard.database.save_hourly_summary(&hourly_summary)
            };

            match save_result {
                Ok(_) => log::info!("小时摘要保存成功: {date} {hour}:00"),
                Err(e) => log::error!("保存小时摘要失败: {e}"),
            }
        }
        Ok(_) => {
            log::debug!("该小时无活动数据: {date} {hour}:00");
        }
        Err(e) => {
            log::error!("获取小时活动数据失败: {e}");
        }
    }
}

pub async fn hourly_summary_task(state: Arc<Mutex<AppState>>) {
    use chrono::{Local, Timelike};

    // 等待30秒后开始（给应用启动留时间，但不用等太久）
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // 启动时回填今天所有已过时段的摘要（覆盖旧格式数据）
    {
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let current_hour = now.hour() as i32;

        log::info!("回填今天 0:00 ~ {current_hour}:00 的小时摘要");
        for hour in 0..current_hour {
            generate_and_save_summary(&state, &date, hour);
        }
    }

    loop {
        let now = Local::now();
        let current_hour = now.hour() as i32;
        let date = now.format("%Y-%m-%d").to_string();

        // 为上一个小时生成摘要（如果还没有）
        let target_hour = if current_hour > 0 {
            current_hour - 1
        } else {
            23
        };
        let target_date = if current_hour > 0 {
            date.clone()
        } else {
            (now - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()
        };

        // 检查是否已有摘要
        let should_generate = {
            let state_guard = state.lock().unwrap_or_else(|e| e.into_inner());
            match state_guard
                .database
                .has_hourly_summary(&target_date, target_hour)
            {
                Ok(has) => !has,
                Err(e) => {
                    log::error!("检查小时摘要失败: {e}");
                    false
                }
            }
        };

        if should_generate {
            log::info!("开始生成 {target_date} {target_hour}:00 的小时摘要");
            generate_and_save_summary(&state, &target_date, target_hour);
        }

        // 休眠到下一个小时的第5分钟
        let next_check = (now + chrono::Duration::hours(1))
            .with_minute(5)
            .unwrap()
            .with_second(0)
            .unwrap();
        let sleep_duration = (next_check - now).num_seconds().max(60) as u64;
        tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
    }
}

