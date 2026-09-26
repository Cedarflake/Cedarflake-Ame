use std::fs::OpenOptions;
use std::os::windows::fs::OpenOptionsExt;

use super::*;

#[test]
fn locked_discovery_retries_images_independently_of_suffix_and_published_baseline() {
    for has_baseline in [false, true] {
        for (name, code) in [
            ("locked.png", "image_open_failed"),
            ("locked.data", "media_signature_unreadable"),
            ("locked", "media_signature_unreadable"),
        ] {
            verify_locked_input(has_baseline, name, code, true);
        }
    }
}

#[test]
fn locked_discovery_completes_non_images_once_after_unlock_without_false_locations() {
    for has_baseline in [false, true] {
        for name in ["document.data", "document"] {
            verify_locked_input(has_baseline, name, "media_signature_unreadable", false);
        }
    }
}

#[test]
fn locked_discovery_preserves_published_location_while_exact_retry_remains_locked() {
    let mut fixture = MediaInputCatalog::new();
    let name = "retained.data";
    fixture.write(
        name,
        encode_rgb_quadrants(MediaFixtureFormat::Png, WIDTH, HEIGHT).expect("pixels"),
    );
    let first = fixture.scan();
    let previous = fixture
        .preview(&location_named(&first.snapshot, name), false)
        .expect("baseline preview");
    let modified = fixture.modified(name);
    let mut lock = Some(
        OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(fixture.source.path().join(name))
            .expect("exclusive read lock"),
    );
    let mut failed_read_observed = false;
    let updated = fixture.scan_with_events(|event| match event {
        ScanEvent::Issue { issue, .. } if issue.code == "media_signature_unreadable" => {
            failed_read_observed = true;
            assert!(lock.is_some());
        }
        ScanEvent::Completed { .. } => {
            assert!(failed_read_observed);
            drop(lock.take());
        }
        _ => {}
    });
    assert!(
        lock.is_none(),
        "publication must finish with precise pending retry ownership"
    );
    fixture.assert_path_queue_count(1);
    let retained = location_named(&updated.snapshot, name);
    assert_eq!(retained.location_id, previous.location_id);
    assert_eq!(retained.asset_id, previous.asset_id);
    assert_eq!(retained.source_generation, previous.source_generation);
    assert_eq!(retained.preview_path, previous.preview_path);
    let queue: (String, i64) = Connection::open_with_flags(
        &fixture.storage.catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY,
    ).expect("read-only queue evidence").query_row(
        "SELECT status, attempt_count FROM library_change_queue WHERE relative_path = ?1 AND scope = 'path'",
        [name], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("durable deferred path");
    assert_eq!(queue.0, "retry_wait");
    assert!(
        queue.1 > 0,
        "production P0 must actually observe the held content lock"
    );
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("reopen");
    let root = catalog
        .load_incremental_catalog_root(&retained.root_id)
        .expect("root query")
        .expect("root");
    let processed = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        &retained.root_id,
        root.root_generation,
        LibraryChangeLane::Live,
        super::super::current_unix_ms()
            .expect("clock")
            .saturating_add(60_000),
        LibraryChangeQueuePolicy::default(),
    )
    .expect("unlock without a source event must finish the durable retry");
    assert_eq!(processed.completed_count, 1);
    assert_eq!(processed.retried_count, 0);
    drop(catalog);
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.assets.len(), 1);
    let ready = fixture
        .preview(&location_named(&snapshot, name), false)
        .expect("warm preview");
    fixture.assert_pixels(&ready, QUADRANT_COLORS);
    assert_eq!(fixture.modified(name), modified);
    fixture.assert_sources_unchanged();
}

fn verify_locked_input(has_baseline: bool, name: &str, code: &str, is_image: bool) {
    let mut fixture = MediaInputCatalog::new();
    fixture.write(
        "keeper.png",
        encode_rgb_quadrants(MediaFixtureFormat::Png, WIDTH, HEIGHT).expect("keeper pixels"),
    );
    if has_baseline {
        fixture.scan();
    }
    let bytes = if is_image {
        encode_rgb_quadrants(MediaFixtureFormat::Png, WIDTH, HEIGHT).expect("identical PNG pixels")
    } else {
        b"ordinary non-image content".to_vec()
    };
    fixture.write(name, bytes);
    let modified = fixture.modified(name);
    let mut lock = Some(
        OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(fixture.source.path().join(name))
            .expect("read-only exclusive lock"),
    );
    let observed = fixture.scan_with_events(|event| {
        if matches!(event, ScanEvent::Issue { issue, .. } if issue.code == code) {
            drop(lock.take());
        }
    });
    assert!(
        lock.is_none(),
        "the real discovery or decoder read must fail first"
    );
    assert_eq!(
        fixture.modified(name),
        modified,
        "unlock does not modify the source"
    );
    assert_eq!(observed.issues.len(), 1);
    assert_eq!(observed.issues[0].code, code);
    fixture.assert_path_queue_count(1);
    let root_id = &observed.snapshot.roots[0].root_id;
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("actual reconciliation repository");
    let root = catalog
        .load_incremental_catalog_root(root_id)
        .expect("root authority")
        .expect("published root");
    let now = super::super::current_unix_ms()
        .expect("clock")
        .saturating_add(1_000);
    let processed = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        root_id,
        root.root_generation,
        LibraryChangeLane::Live,
        now,
        LibraryChangeQueuePolicy::default(),
    )
    .expect("precise production live retry after unlock");
    assert_eq!(processed.completed_count, u32::from(!has_baseline));
    assert_eq!(processed.retried_count, 0);
    let idle = crate::application::process_ready_library_changes_in_lane(
        &mut catalog,
        root_id,
        root.root_generation,
        LibraryChangeLane::Live,
        now.saturating_add(10_000),
        LibraryChangeQueuePolicy::default(),
    )
    .expect("terminal work does not repeat");
    assert_eq!(idle.leased_count, 0);
    if !is_image {
        let evidence = catalog
            .load_terminal_media_evidence_by_relative_paths(root_id, &[name.to_owned()])
            .expect("ordinary documents retain silent negative observations");
        assert_eq!(evidence.len(), 1);
        let evidence = &evidence[0];
        assert!(evidence.file_identity.is_some());
        assert!(evidence.source_revision.is_some());
        assert!(evidence.source_generation > 0);
        assert_eq!(evidence.issue.code, "media_type_unsupported");
        crate::adapters::revalidate_file_state(&crate::domain::ExpectedFileState {
            absolute_path: fixture
                .source
                .path()
                .join(name)
                .to_string_lossy()
                .into_owned(),
            file_size: evidence.file_size,
            modified_unix_ms: evidence.modified_unix_ms,
            file_identity: evidence.file_identity.clone(),
            source_revision: evidence.source_revision.clone(),
        })
        .expect("negative evidence matches the unlocked, unchanged source");
        assert!(
            catalog
                .load_incremental_location_by_relative_path(root_id, name)
                .expect("ordinary document has no gallery location")
                .is_none()
        );
    }
    drop(catalog);
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.assets.len(), 1 + usize::from(is_image));
    if is_image {
        let ready = fixture
            .preview(&location_named(&snapshot, name), false)
            .expect("recovered image preview");
        fixture.assert_pixels(&ready, QUADRANT_COLORS);
    }
    let state: (i64, String, String) = Connection::open_with_flags(
        &fixture.storage.catalog_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .expect("read-only terminal evidence")
    .query_row(
        "SELECT (SELECT COUNT(*) FROM scan_runs), queue.status, lane.lane
         FROM library_change_queue AS queue
         JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
         WHERE queue.relative_path = ?1 AND queue.scope = 'path'",
        [name],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .expect("one durable retry owner");
    assert_eq!(
        state,
        (
            1 + i64::from(has_baseline),
            "completed".to_owned(),
            "p0_live".to_owned()
        )
    );
    fixture.assert_sources_unchanged();
}
