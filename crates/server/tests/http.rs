use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use work_review_core::{database::Store, model::*};
use work_review_server::{router, Hub, ServerConfig};

fn setup() -> (tempfile::TempDir, axum::Router, ServerConfig) {
    let root = work_review_core::config::temporary_directory();
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    let config = ServerConfig::default();
    let hub = Arc::new(Hub {
        store: Mutex::new(Store::open(&dir.path().join("test.db")).unwrap()),
        directory: dir.path().into(),
        config: config.clone(),
    });
    (dir, router(hub), config)
}
async fn call(app: &axum::Router, path: &str, token: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(path)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}
fn batch() -> Batch {
    let id = uuid::Uuid::new_v4().to_string();
    Batch {
        device: Device {
            id: id.clone(),
            name: "MacBook".into(),
            platform: "macos".into(),
        },
        status: AgentStatus {
            state: "recording".into(),
            ..Default::default()
        },
        activities: vec![Upload {
            activity: Activity {
                id: uuid::Uuid::new_v4().to_string(),
                device_id: id,
                timestamp: 100,
                duration: 10,
                app_name: "Safari".into(),
                window_title: "Reference".into(),
                browser_url: Some("https://example.com".into()),
                category: "browser".into(),
                ocr_text: None,
                screenshot: false,
                note: String::new(),
            },
            screenshot_base64: None,
        }],
    }
}
async fn upload(app: &axum::Router, token: &str, batch: &Batch) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ingest")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(batch).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
}
#[tokio::test]
async fn data_endpoints_require_distinct_credentials() {
    let (_dir, app, config) = setup();
    for path in [
        "/api/devices",
        "/api/activities?from=0&to=1000",
        "/api/overview?from=0&to=1000",
        "/api/report?from=0&to=1000",
        "/api/screenshots/invalid/invalid",
    ] {
        assert_eq!(
            call(&app, path, "").await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&app, path, &config.agent_token).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        upload(&app, &config.view_token, &batch()).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(call(&app, "/", "").await.status(), StatusCode::OK);
    assert_eq!(
        call(&app, "/api/does-not-exist", "").await.status(),
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn ingest_retries_are_idempotent_and_bad_batches_are_atomic() {
    let (_dir, app, config) = setup();
    let mut batch = batch();
    assert_eq!(
        upload(&app, &config.agent_token, &batch).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        upload(&app, &config.agent_token, &batch).await.status(),
        StatusCode::OK
    );
    let response = call(&app, "/api/overview?from=0&to=1000", &config.view_token).await;
    let view: Overview =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(view.count, 1);
    batch.activities[0].activity.id = uuid::Uuid::new_v4().to_string();
    batch.activities.push(batch.activities[0].clone());
    batch.activities[1].activity.device_id = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        upload(&app, &config.agent_token, &batch).await.status(),
        StatusCode::BAD_REQUEST
    );
    let response = call(&app, "/api/activities?from=0&to=1000", &config.view_token).await;
    let page: ActivityPage =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        call(&app, "/api/overview?from=0&to=0", &config.view_token)
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
}
