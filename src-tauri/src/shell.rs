// Tauri 外壳层：窗口外观、托盘、Dock 可见性等与 AppHandle 强绑定的部分。
// 采集与存储逻辑见 collection.rs（框架无关）。
use crate::collection::AppState;
use crate::config::AppConfig;
use crate::events::{CONFIG_CHANGED_EVENT, RECORDING_STATE_CHANGED_EVENT, RecordingStatePayload};
use crate::commands;
use crate::error;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use tauri::menu::{MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
pub const MAIN_WINDOW_LABEL: &str = "main";
pub const TRAY_MENU_SHOW_ID: &str = "show";
pub const TRAY_MENU_RECORDING_TOGGLE_ID: &str = "recording-toggle";
pub const TRAY_MENU_QUIT_ID: &str = "quit";

pub type AppMenuItem = MenuItem<tauri::Wry>;
pub struct TrayMenuState {
    pub recording_toggle: AppMenuItem,
}



#[cfg(target_os = "windows")]
pub fn build_windows_window_icon() -> Option<tauri::image::Image<'static>> {
    match image::load_from_memory_with_format(
        include_bytes!("../icons/windows-icon.png"),
        image::ImageFormat::Png,
    ) {
        Ok(decoded) => {
            let decoded = if decoded.width() > 256 || decoded.height() > 256 {
                decoded.resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
            } else {
                decoded
            };

            let rgba = decoded.to_rgba8();
            let (width, height) = rgba.dimensions();
            Some(tauri::image::Image::new_owned(
                rgba.into_raw(),
                width,
                height,
            ))
        }
        Err(e) => {
            log::warn!("加载 Windows 专用窗口图标失败，回退默认图标: {e}");
            None
        }
    }
}

#[cfg(target_os = "windows")]
pub fn configure_windows_backdrop(window: &tauri::WebviewWindow) {
    use std::mem::size_of;
    use tauri::utils::config::Color;
    use winapi::ctypes::c_void;
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::windef::HWND;
    use winapi::um::dwmapi::DwmSetWindowAttribute;

    // Windows 11 DWM attributes used by WinUI 3 for a dark Mica main window.
    const DWMWA_USE_IMMERSIVE_DARK_MODE: DWORD = 20;
    const DWMWA_WINDOW_CORNER_PREFERENCE: DWORD = 33;
    const DWMWA_SYSTEMBACKDROP_TYPE: DWORD = 38;
    const DWMWCP_ROUND: i32 = 2;
    const DWMSBT_MAINWINDOW: i32 = 2;

    let Ok(hwnd) = window.hwnd() else {
        log::warn!("无法取得 Windows 主窗口句柄，使用纯色 WinUI 背景");
        return;
    };
    let raw_hwnd = hwnd.0 as HWND;

    fn set_dwm_attribute<T>(hwnd: HWND, attribute: DWORD, value: &T) -> bool {
        let result = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                value as *const T as *const c_void,
                size_of::<T>() as DWORD,
            )
        };
        result >= 0
    }

    let dark_mode: i32 = 1;
    let dark_mode_applied = set_dwm_attribute(raw_hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark_mode);
    let corners_applied =
        set_dwm_attribute(raw_hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND);
    let mica_applied = set_dwm_attribute(raw_hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &DWMSBT_MAINWINDOW);

    if !(dark_mode_applied && corners_applied && mica_applied) {
        log::warn!("部分 Windows 11 DWM 外观属性不可用，继续使用 WinUI 深色回退层");
    }

    if let Err(error) = window.set_background_color(Some(Color(0, 0, 0, 0))) {
        log::warn!("设置透明 WebView 背景失败，继续使用 WinUI 深色回退层: {error}");
    }
}

pub fn effective_dock_visibility(hide_dock_icon: bool, has_main_window: bool) -> bool {
    !hide_dock_icon && has_main_window
}

pub fn sync_effective_dock_visibility(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let hide_dock_icon = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        state.config.hide_dock_icon
    };
    let has_main_window = app.get_webview_window(MAIN_WINDOW_LABEL).is_some();
    let visible = effective_dock_visibility(hide_dock_icon, has_main_window);
    commands::apply_dock_visibility(visible, false);
}

pub fn configure_main_window(_window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        if let Some(icon) = build_windows_window_icon() {
            if let Err(e) = _window.set_icon(icon) {
                log::warn!("设置 Windows 主窗口图标失败，继续使用默认图标: {e}");
            }
        }

        configure_windows_backdrop(_window);
    }

    #[cfg(target_os = "macos")]
    {
        use tauri::TitleBarStyle;

        let _ = _window.set_decorations(true);
        let _ = _window.set_title_bar_style(TitleBarStyle::Transparent);
        configure_main_window_collection_behavior(_window);
    }
}

#[cfg(target_os = "macos")]
pub fn configure_main_window_collection_behavior(window: &tauri::WebviewWindow) {
    use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
    use cocoa::base::id;

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_window = ns_window as id;
            let mut behavior = ns_window.collectionBehavior();
            behavior |= NSWindowCollectionBehavior::NSWindowCollectionBehaviorMoveToActiveSpace;
            ns_window.setCollectionBehavior_(behavior);
        }
    }
}

pub fn align_window_to_reference_monitor(
    window: &tauri::WebviewWindow,
    reference_window: Option<&tauri::WebviewWindow>,
) {
    let Some(reference_window) = reference_window else {
        return;
    };

    let Ok(Some(reference_monitor)) = reference_window.current_monitor() else {
        return;
    };
    let Ok(window_size) = window.outer_size() else {
        return;
    };

    let work_area = reference_monitor.work_area();
    let monitor_width = work_area.size.width as i32;
    let monitor_height = work_area.size.height as i32;
    let window_width = window_size.width as i32;
    let window_height = window_size.height as i32;

    let target_x = work_area.position.x + ((monitor_width - window_width).max(0) / 2);
    let target_y = work_area.position.y + ((monitor_height - window_height).max(0) / 2);

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        target_x, target_y,
    )));
}

pub fn ensure_main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, error::AppError> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == MAIN_WINDOW_LABEL)
        .or_else(|| app.config().app.windows.first())
        .ok_or_else(|| error::AppError::Unknown("未找到主窗口配置".to_string()))?;

    let window = tauri::WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|e| error::AppError::Unknown(format!("创建主窗口构建器失败: {e}")))?
        .build()
        .map_err(|e| error::AppError::Unknown(format!("重建主窗口失败: {e}")))?;

    configure_main_window(&window);
    Ok(window)
}

pub fn reveal_main_window(
    app: &AppHandle,
    source_window_label: Option<&str>,
) -> Result<(), error::AppError> {
    let window = ensure_main_window(app)?;
    let reference_window = source_window_label.and_then(|label| app.get_webview_window(label));
    align_window_to_reference_monitor(&window, reference_window.as_ref());
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    sync_effective_dock_visibility(app);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingToggleAction {
    Start,
    Pause,
    Resume,
}

pub fn tray_recording_toggle_action(is_recording: bool, is_paused: bool) -> RecordingToggleAction {
    if !is_recording {
        RecordingToggleAction::Start
    } else if is_paused {
        RecordingToggleAction::Resume
    } else {
        RecordingToggleAction::Pause
    }
}

pub fn tray_recording_toggle_label(is_recording: bool, is_paused: bool) -> &'static str {
    match tray_recording_toggle_action(is_recording, is_paused) {
        RecordingToggleAction::Start => "开始录制",
        RecordingToggleAction::Pause => "暂停录制",
        RecordingToggleAction::Resume => "恢复录制",
    }
}

pub fn refresh_tray_menu(app: &AppHandle) {
    let Some(tray_menu) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let (is_recording, is_paused) = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        (state.is_recording, state.is_paused)
    };

    let _ = tray_menu
        .recording_toggle
        .set_text(tray_recording_toggle_label(is_recording, is_paused));
}

pub fn emit_recording_state_changed(app: &AppHandle) {
    let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() else {
        return;
    };

    let payload = {
        let state = state.lock().unwrap_or_else(|e| e.into_inner());
        RecordingStatePayload {
            is_recording: state.is_recording,
            is_paused: state.is_paused,
        }
    };

    let _ = app.emit(RECORDING_STATE_CHANGED_EVENT, payload);
    refresh_tray_menu(app);
}

pub fn emit_config_changed(app: &AppHandle, config: &AppConfig) {
    let _ = app.emit(CONFIG_CHANGED_EVENT, config);
    refresh_tray_menu(app);
}

pub fn build_tray_icon(app: &tauri::App) -> tauri::image::Image<'static> {
    #[cfg(target_os = "macos")]
    {
        match image::load_from_memory_with_format(
            include_bytes!("../icons/tray-template.png"),
            image::ImageFormat::Png,
        ) {
            Ok(decoded) => {
                let rgba = decoded.to_rgba8();
                let (width, height) = rgba.dimensions();
                tauri::image::Image::new_owned(rgba.into_raw(), width, height)
            }
            Err(e) => {
                log::warn!("加载 macOS 状态栏专用图标失败，回退默认图标: {e}");
                app.default_window_icon()
                    .expect("应用默认图标缺失")
                    .clone()
                    .to_owned()
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        app.default_window_icon()
            .expect("应用默认图标缺失")
            .clone()
            .to_owned()
    }
}

/// 应用状态

#[derive(Default)]
pub struct AppLifecycleState {
    pub suppress_next_exit: bool,
    pub explicit_quit_requested: bool,
}


pub fn should_prevent_exit(suppress_next_exit: bool, explicit_quit_requested: bool) -> bool {
    suppress_next_exit && !explicit_quit_requested
}


pub fn should_request_screen_capture_permission(
    has_screen_capture_permission: bool,
    already_prompted: bool,
) -> bool {
    !has_screen_capture_permission && !already_prompted
}


#[cfg(windows)]
pub fn configure_windows_webview_low_power_mode() {
    const LOW_POWER_ARGS: &[&str] = &[
        "--disable-gpu",
        "--disable-gpu-compositing",
        "--disable-gpu-rasterization",
        "--disable-accelerated-2d-canvas",
    ];

    let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let mut args = existing
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();

    for arg in LOW_POWER_ARGS {
        if !args.iter().any(|existing_arg| existing_arg == arg) {
            args.push((*arg).to_string());
        }
    }

    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args.join(" "));
}
