// 框架无关的事件出口：Tauri 构建下经全局 AppHandle 发送，FFI 构建下回调宿主（C#）。
use crate::config::AppConfig;
use serde::Serialize;
use serde_json::Value;

pub const RECORDING_STATE_CHANGED_EVENT: &str = "recording-state-changed";
pub const CONFIG_CHANGED_EVENT: &str = "config-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingStatePayload {
    pub is_recording: bool,
    pub is_paused: bool,
}


#[cfg(not(feature = "ffi"))]
mod imp {
    use super::*;
    use crate::shell;
    use once_cell::sync::OnceCell;
    use tauri::{AppHandle, Emitter, Manager};

    static TAURI_HANDLE: OnceCell<AppHandle> = OnceCell::new();

    pub fn set_tauri_handle(handle: AppHandle) {
        let _ = TAURI_HANDLE.set(handle);
    }

    pub fn emit_event<T: serde::Serialize + ?Sized>(name: &str, payload: &T) {
        let payload = &serde_json::to_value(payload).unwrap_or(Value::Null);
        let Some(handle) = TAURI_HANDLE.get() else {
            log::warn!("事件 {name} 无可用的 Tauri AppHandle，已丢弃");
            return;
        };
        let _ = Emitter::emit(handle, name, payload);
        if name == RECORDING_STATE_CHANGED_EVENT || name == CONFIG_CHANGED_EVENT {
            shell::refresh_tray_menu(handle);
        }
    }

    pub fn with_tauri_handle(f: impl FnOnce(&AppHandle)) {
        if let Some(handle) = TAURI_HANDLE.get() {
            f(handle);
        }
    }
}

#[cfg(feature = "ffi")]
mod imp {
    use super::*;
    pub fn emit_event<T: serde::Serialize + ?Sized>(name: &str, payload: &T) {
        let value = serde_json::to_value(payload).unwrap_or(Value::Null);
        crate::ffi::emit_to_host(name, &value);
    }
}

pub use imp::emit_event;
#[cfg(not(feature = "ffi"))]
pub use imp::{set_tauri_handle, with_tauri_handle};

/// 通知前端录制状态变化（Tauri 构建下同时刷新托盘菜单文案）。
pub fn notify_recording_state_changed(is_recording: bool, is_paused: bool) {
    let payload = RecordingStatePayload {
        is_recording,
        is_paused,
    };
    emit_event(
        RECORDING_STATE_CHANGED_EVENT,
        &serde_json::to_value(payload).unwrap_or(Value::Null),
    );
}

/// 通知前端配置已变更。
pub fn notify_config_changed(config: &AppConfig) {
    let payload = serde_json::to_value(config).unwrap_or(Value::Null);
    emit_event(CONFIG_CHANGED_EVENT, &payload);
}
