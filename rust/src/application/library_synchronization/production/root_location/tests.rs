use super::*;

#[test]
fn stop_joins_an_in_flight_discovery_without_admitting_another_worker() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        while !worker_cancelled.load(Ordering::Acquire) {
            thread::yield_now();
        }
        sender.send(Ok(())).unwrap();
    });
    let mut owner = RootLocationRecoveryOwner {
        task: Some(LocationTask {
            root: test_root(),
            cancelled: Arc::clone(&cancelled),
            receiver,
            worker: Some(worker),
        }),
        ..RootLocationRecoveryOwner::default()
    };
    owner
        .finish_stopping_until(Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(cancelled.load(Ordering::Acquire));
    assert!(owner.stopping);
    assert!(owner.task.is_none());
}

#[test]
fn stale_binding_does_not_inherit_failure_or_retry_admission() {
    let root = test_root();
    let mut newer = root.clone();
    newer.root_generation = newer.root_generation.next().unwrap();
    assert!(!same_binding(&root, &newer));
    newer = root.clone();
    newer.root_path.push_str("-moved");
    assert!(!same_binding(&root, &newer));
}

fn test_root() -> IncrementalCatalogRoot {
    IncrementalCatalogRoot {
        root_id: "fixture".into(),
        root_path: r"C:\fixture".into(),
        root_generation: crate::domain::LibraryRootGeneration::initial(),
        active_scan_id: Some("published".into()),
        has_running_scan: false,
        catalog_revision: 1,
        last_consistency_audit_unix_ms: None,
        publication_root_identity: None,
    }
}
