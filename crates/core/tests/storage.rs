use work_review_core::{config::StorageConfig, database::Store, model::*};

fn directory() -> tempfile::TempDir {
    let root = work_review_core::config::temporary_directory();
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}
fn activity(device: &str, start: i64, duration: i64) -> Activity {
    Activity {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device.into(),
        timestamp: start,
        duration,
        app_name: "Code".into(),
        window_title: "project".into(),
        browser_url: None,
        category: "development".into(),
        ocr_text: None,
        screenshot: false,
        note: String::new(),
    }
}
fn query(from: i64, to: i64) -> RangeQuery {
    RangeQuery {
        from,
        to,
        device: None,
        q: None,
        before_time: None,
        before_id: None,
        before_device: None,
        limit: None,
    }
}
#[test]
fn union_time_clips_days_hours_and_device_filters() {
    let dir = directory();
    let mut store = Store::open(&dir.path().join("test.db")).unwrap();
    let d1 = uuid::Uuid::new_v4().to_string();
    let d2 = uuid::Uuid::new_v4().to_string();
    let events = vec![
        activity(&d1, 3500, 200),
        activity(&d2, 3550, 200),
        activity(&d1, 3740, 200),
    ];
    let batch = Batch {
        device: Device {
            id: d1.clone(),
            name: "Windows".into(),
            platform: "windows".into(),
        },
        status: AgentStatus::default(),
        activities: events
            .iter()
            .filter(|a| a.device_id == d1)
            .map(|a| Upload {
                activity: a.clone(),
                screenshot_base64: None,
            })
            .collect(),
    };
    store.ingest(&batch, &[None, None], 1).unwrap();
    store.enqueue(&events[1], None).unwrap();
    let view = store.overview(&query(3600, 3900)).unwrap();
    assert_eq!(view.active_seconds, 300);
    assert_eq!(view.device_seconds, 410);
    assert_eq!(view.count, 3);
    let mut filtered = query(3600, 3900);
    filtered.device = Some(d2);
    assert_eq!(store.overview(&filtered).unwrap().active_seconds, 150);
    let day = store.overview(&query(0, 7200)).unwrap();
    let usage = day.devices.iter().find(|d| d.device_id == d1).unwrap();
    assert_eq!(usage.hours, vec![100, 300]);
}
#[test]
fn retries_preserve_hub_notes_and_pagination_has_no_duplicates() {
    let dir = directory();
    let mut store = Store::open(&dir.path().join("test.db")).unwrap();
    let device = uuid::Uuid::new_v4().to_string();
    let batch = Batch {
        device: Device {
            id: device.clone(),
            name: "same-name".into(),
            platform: "macos".into(),
        },
        status: AgentStatus::default(),
        activities: (0..5)
            .map(|_| Upload {
                activity: activity(&device, 100, 10),
                screenshot_base64: None,
            })
            .collect(),
    };
    store.ingest(&batch, &vec![None; 5], 1).unwrap();
    let id = &batch.activities[0].activity.id;
    store.set_note(&device, id, "special task").unwrap();
    store.ingest(&batch, &vec![None; 5], 2).unwrap();
    let mut range = query(0, 1000);
    range.limit = Some(2);
    let mut ids = std::collections::HashSet::new();
    loop {
        let page = store.activities(&range).unwrap();
        for a in page.items {
            assert!(ids.insert(a.id));
        }
        if let Some(next) = page.next {
            range.before_id = Some(next.id);
            range.before_device = Some(next.device_id);
            range.before_time = Some(next.timestamp);
        } else {
            break;
        }
    }
    assert_eq!(ids.len(), 5);
    let mut search = query(0, 1000);
    search.q = Some("special task".into());
    assert_eq!(store.overview(&search).unwrap().count, 1);
    assert_eq!(
        store.activities(&search).unwrap().items[0].note,
        "special task"
    );
}
#[test]
fn cleanup_preserves_all_unacknowledged_data() {
    let dir = directory();
    let mut store = Store::open(&dir.path().join("test.db")).unwrap();
    let device = uuid::Uuid::new_v4().to_string();
    let pending = activity(&device, 10, 10);
    let acknowledged = activity(&device, 20, 10);
    let pending_image = dir.path().join("pending.jpg");
    let ack_image = dir.path().join("synced.jpg");
    std::fs::write(&pending_image, b"pending").unwrap();
    std::fs::write(&ack_image, b"synced").unwrap();
    store.enqueue(&pending, Some(&pending_image)).unwrap();
    store.enqueue(&acknowledged, Some(&ack_image)).unwrap();
    store.acknowledge(&device, &[acknowledged.id]).unwrap();
    store
        .cleanup(100 * 86400, &StorageConfig::default())
        .unwrap();
    assert!(pending_image.exists());
    assert!(!ack_image.exists());
    assert_eq!(store.pending_count().unwrap(), 1);
    assert_eq!(store.overview(&query(0, 1000)).unwrap().count, 1);
}

#[test]
fn pagination_keeps_same_ids_from_distinct_devices_and_search_ignores_schema_keys() {
    let dir = directory();
    let store = Store::open(&dir.path().join("test.db")).unwrap();
    let mut a = activity(&uuid::Uuid::new_v4().to_string(), 100, 10);
    store.enqueue(&a, None).unwrap();
    a.device_id = uuid::Uuid::new_v4().to_string();
    store.enqueue(&a, None).unwrap();
    let mut range = query(0, 1000);
    range.limit = Some(1);
    let first = store.activities(&range).unwrap();
    let cursor = first.next.unwrap();
    range.before_time = Some(cursor.timestamp);
    range.before_id = Some(cursor.id);
    range.before_device = Some(cursor.device_id);
    let second = store.activities(&range).unwrap();
    assert_eq!(second.items.len(), 1);
    assert_ne!(first.items[0].device_id, second.items[0].device_id);
    let mut search = query(0, 1000);
    search.q = Some("device_id".into());
    assert_eq!(store.overview(&search).unwrap().count, 0);
}
