use super::*;

#[test]
fn cancelled_identity_recovery_reopens_at_the_new_path_without_another_root() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    fs::rename(&fixture.original, &fixture.replacement).unwrap();
    let mut cancelled = false;
    run_scan_with_storage(
        request("cancelled-recovery", &fixture.replacement),
        |event| {
            if let ScanEvent::Started { scan_id, .. } = &event {
                assert!(cancel_scan(scan_id));
            }
            cancelled |= matches!(event, ScanEvent::Cancelled { .. });
            true
        },
        fixture.storage.clone(),
    )
    .expect("cancel the admitted recovery");
    assert!(cancelled);
    let cancelled_snapshot = fixture.snapshot();
    assert_eq!(cancelled_snapshot.roots.len(), 1);
    assert_eq!(
        cancelled_snapshot.roots[0].active_scan_id,
        before.roots[0].active_scan_id
    );
    assert_eq!(
        cancelled_snapshot.assets[0].absolute_path,
        before.assets[0].absolute_path
    );

    run_scan_with_storage(
        request("retry-recovery", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("ordinary retry after catalog reopen");
    let after = fixture.snapshot();
    assert_eq!(after.roots.len(), 1);
    assert_eq!(after.roots[0].root_id, fixture.root.root_id);
    assert_eq!(after.assets[0].asset_id, before.assets[0].asset_id);
    assert_eq!(after.assets[0].scan_id, "retry-recovery");
}

#[test]
fn identity_recovery_cannot_change_binding_without_a_held_namespace_guard() {
    let fixture = Fixture::new();
    fs::rename(&fixture.original, &fixture.replacement).unwrap();
    let _failure = crate::adapters::force_publication_namespace_guard_failure_for_test();
    let error = run_scan_with_storage(
        request("unprotected-recovery", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect_err("a discovered identity does not replace the namespace capability");
    assert_eq!(error.code, "root_publication_namespace_guard_unsupported");
    let root = fixture.current_root();
    assert_eq!(root.root_path, fixture.root.root_path);
    assert_eq!(root.root_generation, fixture.root.root_generation);
    assert_eq!(root.active_scan_id, fixture.root.active_scan_id);
    assert!(!root.has_running_scan);
}

#[test]
fn identity_change_after_candidate_selection_rejects_the_admission_transaction() {
    let fixture = Fixture::new();
    fs::rename(&fixture.original, &fixture.replacement).unwrap();
    let identity = fixture.root.publication_root_identity.as_ref().unwrap();
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    assert_eq!(
        catalog
            .load_published_root_id_by_identity(identity)
            .unwrap()
            .as_deref(),
        Some(fixture.root.root_id.as_str())
    );
    let mut replacement_identity = identity.clone();
    let last = replacement_identity.value.pop().unwrap();
    replacement_identity
        .value
        .push(if last == '0' { '1' } else { '0' });
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .execute(
            "UPDATE library_root_publication_namespaces SET identity_value = ?1 WHERE root_id = ?2",
            rusqlite::params![replacement_identity.value, fixture.root.root_id],
        )
        .unwrap();
    drop(connection);
    let destination = fixture.replacement.canonicalize().unwrap();
    let error = catalog
        .begin_identity_recovered_scan(
            &request("stale-identity", &fixture.replacement),
            &fixture.root,
            &destination.to_string_lossy(),
            identity,
        )
        .expect_err("a once-matching identity cannot authorize a changed namespace");
    assert_eq!(error.code, "catalog_root_identity_changed");
    let after = fixture.current_root();
    assert_eq!(after.root_path, fixture.root.root_path);
    assert_eq!(after.root_generation, fixture.root.root_generation);
    assert_eq!(after.active_scan_id, fixture.root.active_scan_id);
    assert!(!after.has_running_scan);
}

#[test]
fn ambiguous_identity_added_after_lookup_cannot_retire_the_selected_root() {
    let fixture = Fixture::new();
    let peer = fixture._source.path().join("late-peer");
    fs::create_dir(&peer).unwrap();
    run_scan_with_storage(request("peer", &peer), |_| true, fixture.storage.clone()).unwrap();
    let identity = fixture.root.publication_root_identity.as_ref().unwrap();
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).unwrap();
    assert_eq!(
        catalog
            .load_published_root_id_by_identity(identity)
            .unwrap()
            .as_deref(),
        Some(fixture.root.root_id.as_str())
    );
    let before = catalog.load_incremental_catalog_roots().unwrap();
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .execute(
            "UPDATE library_root_publication_namespaces SET identity_value = ?1",
            [&identity.value],
        )
        .unwrap();
    drop(connection);
    fs::rename(&fixture.original, &fixture.replacement).unwrap();
    let destination = fixture.replacement.canonicalize().unwrap();
    let error = catalog
        .begin_identity_recovered_scan(
            &request("late-ambiguity", &fixture.replacement),
            &fixture.root,
            &destination.to_string_lossy(),
            identity,
        )
        .expect_err("transaction must repeat the uniqueness check");
    assert_eq!(error.code, "catalog_root_identity_ambiguous");
    for root in catalog.load_incremental_catalog_roots().unwrap() {
        let old = before
            .iter()
            .find(|old| old.root_id == root.root_id)
            .unwrap();
        assert_eq!(root.root_path, old.root_path);
        assert_eq!(root.root_generation, old.root_generation);
        assert_eq!(root.active_scan_id, old.active_scan_id);
        assert!(!root.has_running_scan);
    }
}

#[test]
fn registered_path_precedes_another_roots_ambiguous_identity_claim() {
    let fixture = Fixture::new();
    let peer = fixture._source.path().join("stale-peer");
    fs::create_dir(&peer).unwrap();
    run_scan_with_storage(request("peer", &peer), |_| true, fixture.storage.clone()).unwrap();
    let identity = fixture.root.publication_root_identity.as_ref().unwrap();
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .execute(
            "UPDATE library_root_publication_namespaces SET identity_value = ?1",
            [&identity.value],
        )
        .unwrap();
    drop(connection);
    run_scan_with_storage(
        request("known-path", &fixture.original),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("an ordinary update does not need identity-based root selection");
    assert_eq!(fixture.snapshot().roots.len(), 2);
    assert_eq!(
        fixture.current_root().root_generation,
        fixture.root.root_generation
    );
    assert_eq!(
        fixture.current_root().active_scan_id.as_deref(),
        Some("known-path")
    );
}
