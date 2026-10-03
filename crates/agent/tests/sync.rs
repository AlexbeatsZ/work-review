use std::sync::{Arc, Mutex};
use work_review_core::{config::AgentConfig, database::Store, model::*};
use work_review_server::{Hub, ServerConfig};
#[tokio::test]
async fn offline_queue_survives_and_drains_after_reconnect() {
    let root = work_review_core::config::temporary_directory();
    std::fs::create_dir_all(&root).unwrap();
    let dir = tempfile::tempdir_in(root).unwrap();
    let mut config = AgentConfig::default();
    let token = ServerConfig::default();
    config.token = Some(token.agent_token.clone());
    let mut store = Store::open(&dir.path().join("agent.db")).unwrap();
    let activity = Activity {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: config.device.id.clone(),
        timestamp: 100,
        duration: 10,
        app_name: "Code".into(),
        window_title: "Task".into(),
        browser_url: None,
        category: "development".into(),
        ocr_text: None,
        screenshot: false,
        note: String::new(),
    };
    store.enqueue(&activity, None).unwrap();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = unavailable.local_addr().unwrap();
    drop(unavailable);
    config.server_url = Some(format!("http://{address}"));
    assert!(work_review_agent::sync::synchronize(
        &client,
        &mut store,
        &config,
        AgentStatus::default()
    )
    .await
    .is_err());
    assert_eq!(store.pending_count().unwrap(), 1);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.server_url = Some(format!("http://{}", listener.local_addr().unwrap()));
    let hub = Arc::new(Hub {
        store: Mutex::new(Store::open(&dir.path().join("hub.db")).unwrap()),
        directory: dir.path().into(),
        config: token,
    });
    let verify = hub.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, work_review_server::router(hub))
            .await
            .unwrap();
    });
    assert_eq!(
        work_review_agent::sync::synchronize(&client, &mut store, &config, AgentStatus::default())
            .await
            .unwrap(),
        1
    );
    assert_eq!(store.pending_count().unwrap(), 0);
    assert_eq!(
        work_review_agent::sync::synchronize(&client, &mut store, &config, AgentStatus::default())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        verify
            .store
            .lock()
            .unwrap()
            .connection
            .query_row("SELECT count(*) FROM events", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    server.abort();
    let _ = server.await;
}
