// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(feature = "ffi"))]
fn main() {
    work_review_engine::run_tauri_app();
}

#[cfg(feature = "ffi")]
fn main() {}
