#![cfg(any(windows, target_os = "macos"))]

use std::{process::Command, time::Duration};

#[test]
fn first_run_initializes_storage_and_rejects_duplicate_collectors() {
    let root = work_review_core::config::temporary_directory();
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let binary = env!("CARGO_BIN_EXE_work-review-agent");
    let command = || {
        let mut command = Command::new(binary);
        command.arg("--data-dir").arg(directory.path());
        command
    };
    assert!(command().arg("init").output().unwrap().status.success());
    assert!(command()
        .args(["config", "--set", "enabled=false"])
        .output()
        .unwrap()
        .status
        .success());
    let mut running = command().args(["run", "--duration", "3"]).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    let duplicate = command().args(["run", "--duration", "1"]).output().unwrap();
    assert!(!duplicate.status.success());
    let status: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.path().join("status.json")).unwrap())
            .unwrap();
    assert_eq!(status["running"], true);
    assert_eq!(status["capture"]["state"], "paused");
    assert!(running.wait().unwrap().success());
    let status = command().arg("status").output().unwrap();
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["pending"], 0);
    assert_eq!(status["status"]["running"], false);
}
