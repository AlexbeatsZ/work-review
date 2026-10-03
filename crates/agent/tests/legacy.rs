use work_review_core::{database::Store, model::RangeQuery};
#[test]
fn legacy_import_is_read_only_repeatable_and_keeps_interval_notes() {
    let root = work_review_core::config::temporary_directory();
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let original = directory.path().join("legacy.db");
    let old = rusqlite::Connection::open(&original).unwrap();
    old.execute_batch("CREATE TABLE activities(id INTEGER PRIMARY KEY,timestamp INTEGER,duration INTEGER,app_name TEXT,window_title TEXT,category TEXT,screenshot_path TEXT,browser_url TEXT,ocr_text TEXT,intent_purpose TEXT,intent_note TEXT);
    INSERT INTO activities VALUES(1,200,100,'Code','task','development','','https://example.com','content','purpose','note');").unwrap();
    drop(old);
    let before = std::fs::read(&original).unwrap();
    let queue = Store::open(&directory.path().join("agent.db")).unwrap();
    let device = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        work_review_agent::legacy::import(
            &queue,
            &original,
            directory.path(),
            directory.path(),
            &device
        )
        .unwrap(),
        1
    );
    assert_eq!(
        work_review_agent::legacy::import(
            &queue,
            &original,
            directory.path(),
            directory.path(),
            &device
        )
        .unwrap(),
        0
    );
    assert_eq!(std::fs::read(&original).unwrap(), before);
    let page = queue
        .activities(&RangeQuery {
            from: 0,
            to: 1000,
            device: None,
            q: None,
            before_time: None,
            before_id: None,
            before_device: None,
            limit: None,
        })
        .unwrap();
    let a = &page.items[0];
    assert_eq!(a.timestamp, 100);
    assert_eq!(a.duration, 100);
    assert_eq!(a.note, "purpose\nnote");
    assert_eq!(a.ocr_text.as_deref(), Some("content"));
}
