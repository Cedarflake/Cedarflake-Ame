use std::panic::resume_unwind;
use std::path::Path;

use super::*;
use crate::domain::{
    AssetLocationView, CatalogFreshnessState, GalleryQuery, LibraryRootAvailability,
};

#[test]
fn production_offline_root_recovery_preserves_catalog_while_peer_changes_publish() {
    let fixture = ProductionGapFixture::new_with_media_baseline("offline-peer-publication");
    let peer_source = fixture._directory.path().join("peer-source");
    std::fs::create_dir(&peer_source).expect("controlled peer source");
    write_peer_image(&peer_source.join("baseline.png"), [30, 60, 90]);
    let peer_path = crate::adapters::FileDiscovery::new(&peer_source.to_string_lossy())
        .expect("peer discovery")
        .canonical_root()
        .expect("canonical peer root")
        .to_string_lossy()
        .into_owned();
    let peer_id = crate::application::scan_library::stable_id("library-root-v1", &peer_path);
    let mut completed = false;
    crate::application::scan_library::run_scan_with_storage(
        ScanRequest {
            scan_id: "offline-peer-baseline".to_owned(),
            root_path: peer_path,
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if let crate::domain::ScanEvent::Completed {
                asset_count,
                issue_count,
                ..
            } = event
            {
                assert_eq!((asset_count, issue_count), (1, 0));
                completed = true;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("publish peer baseline through the production scan");
    assert!(completed);
    let mut catalog = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("populated two-root catalog");
    let root_path = catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root A")
        .expect("registered root A")
        .root_path;
    let cached_assets = visible_assets(&mut catalog, &fixture.root_id);
    assert_eq!(cached_assets.len(), 2);
    assert_eq!(visible_assets(&mut catalog, &peer_id).len(), 1);
    let source_before = source_snapshot(&fixture.source_root);
    let mut peer_expected = source_snapshot(&peer_source);
    let factory = QueuedSourceFactory::default();
    let mut production = fast_gap_runtime(factory.clone(), test_live_only_connection());

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("start both production observers");
            if snapshot.roots.len() == 2
                && snapshot.roots.iter().all(|root| {
                    root.source_health == crate::domain::LibraryChangeSourceHealth::Healthy
                })
            {
                break;
            }
            assert!(
                Instant::now() < ready_deadline,
                "both observers must become healthy"
            );
            thread::sleep(Duration::from_millis(2));
        }
        let offline_root = fixture.source_root.with_file_name("source-offline");
        std::fs::rename(&fixture.source_root, &offline_root)
            .expect("make only fixture A unavailable");
        let unavailable = poll_runtime_with_storage(&mut production, &fixture.storage)
            .expect("observe real root loss");
        assert_unavailable(&unavailable, &fixture.root_id);
        assert_eq!(
            visible_assets(&mut catalog, &fixture.root_id),
            cached_assets
        );

        let first_name = "while-a-offline.png";
        write_peer_image(&peer_source.join(first_name), [40, 80, 120]);
        peer_expected.insert(
            first_name.to_owned(),
            file_snapshot(&peer_source.join(first_name)),
        );
        push_peer_change(&factory, &peer_id, first_name, 1);
        let peer_published =
            wait_for_peer_publication(&mut production, &fixture, &catalog, &peer_id, first_name);
        assert_unavailable(&peer_published, &fixture.root_id);
        assert_eq!(
            visible_assets(&mut catalog, &fixture.root_id),
            cached_assets
        );

        crate::adapters::reset_source_enumeration_instrumentation(&root_path);
        let source_gate = crate::adapters::gate_source_enumeration(&root_path);
        std::fs::rename(&offline_root, &fixture.source_root)
            .expect("restore the original A directory identity");
        let gap = wait_for_durable_gap(&mut production, &fixture);
        assert_p0_live_gap(&gap);
        let enumeration_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("admit A recovery through its availability gap");
            if production.recovery.as_ref().is_some_and(|task| {
                task.root_id == fixture.root_id
                    && matches!(&task.kind, RecoveryTaskKind::MetadataInventory { .. })
            }) && source_gate.wait_until_blocked(Duration::from_millis(10))
            {
                break;
            }
            assert!(
                Instant::now() < enumeration_deadline,
                "A must own the blocked P2 source"
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        assert_eq!(
            visible_assets(&mut catalog, &fixture.root_id),
            cached_assets
        );

        let second_name = "while-a-recovers.png";
        write_peer_image(&peer_source.join(second_name), [50, 100, 150]);
        peer_expected.insert(
            second_name.to_owned(),
            file_snapshot(&peer_source.join(second_name)),
        );
        push_peer_change(&factory, &peer_id, second_name, 2);
        let peer_published =
            wait_for_peer_publication(&mut production, &fixture, &catalog, &peer_id, second_name);
        let recovering = peer_published
            .roots
            .iter()
            .find(|root| root.root_id == fixture.root_id)
            .expect("A remains registered during recovery");
        assert_eq!(recovering.availability, LibraryRootAvailability::Available);
        assert_ne!(recovering.freshness, CatalogFreshnessState::Synchronized);
        assert!(source_gate.wait_until_blocked(Duration::from_millis(10)));
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        assert_eq!(
            visible_assets(&mut catalog, &fixture.root_id),
            cached_assets
        );
        assert_eq!(visible_assets(&mut catalog, &peer_id).len(), 3);

        source_gate.release();
        let convergence_deadline = Instant::now() + Duration::from_secs(8);
        loop {
            let snapshot = poll_runtime_with_storage(&mut production, &fixture.storage)
                .expect("complete both production roots");
            if snapshot.roots.len() == 2
                && snapshot.roots.iter().all(|root| {
                    root.freshness == CatalogFreshnessState::Synchronized
                        && root.pending_change_count == 0
                        && root.retry_wait_count == 0
                })
            {
                break;
            }
            assert!(
                Instant::now() < convergence_deadline,
                "both populated roots must converge: {:?}",
                snapshot.roots
            );
            thread::sleep(Duration::from_millis(2));
        }
        assert_p2_gap_consumer(&fixture, gap.change_id);
        gap.change_id
    }));
    // The existing gate's Drop releases source reads before the owned stop joins workers.
    let stopped = production.finish_stopping_until(Instant::now() + Duration::from_secs(2));
    let gap_id = match outcome {
        Ok(gap_id) => gap_id,
        Err(panic) => {
            if let Err(error) = stopped {
                eprintln!("availability fixture stop also failed: {error:?}");
            }
            resume_unwind(panic);
        }
    };
    stopped.expect("all availability fixture workers retire within the existing stop budget");
    assert!(
        production.live.is_none() && production.journal.is_none() && production.recovery.is_none()
    );
    assert!(production.core_stopped && production.journal_closed);
    drop(catalog);
    let mut reopened = SqliteCatalog::open(fixture.storage.catalog_path.clone())
        .expect("FULL reopen after A recovery and B publications");
    assert_eq!(
        reopened
            .load_incremental_catalog_roots()
            .expect("current roots")
            .len(),
        2
    );
    let recovered_assets = visible_assets(&mut reopened, &fixture.root_id);
    assert_eq!(recovered_assets.len(), 2);
    for (before, after) in cached_assets.iter().zip(&recovered_assets) {
        assert_eq!(
            (&before.relative_path, &before.asset_id, &before.location_id),
            (&after.relative_path, &after.asset_id, &after.location_id)
        );
    }
    let peer_assets = visible_assets(&mut reopened, &peer_id);
    assert_eq!(
        peer_assets
            .iter()
            .map(|asset| asset.relative_path.as_str())
            .collect::<Vec<_>>(),
        [
            "baseline.png",
            "while-a-offline.png",
            "while-a-recovers.png"
        ]
    );
    assert_terminal_recovery(&fixture, gap_id);
    assert_eq!(source_snapshot(&fixture.source_root), source_before);
    assert_eq!(source_snapshot(&peer_source), peer_expected);
}

fn assert_unavailable(snapshot: &LibrarySynchronizationSnapshot, root_id: &str) {
    let root = snapshot
        .roots
        .iter()
        .find(|root| root.root_id == root_id)
        .expect("retained root");
    assert!(matches!(
        root.availability,
        LibraryRootAvailability::Offline | LibraryRootAvailability::Missing
    ));
    assert_eq!(root.freshness, CatalogFreshnessState::Unavailable);
}

fn visible_assets(catalog: &mut SqliteCatalog, root_id: &str) -> Vec<AssetLocationView> {
    let snapshot = catalog
        .load_snapshot(
            8,
            &GalleryQuery {
                root_id: Some(root_id.to_owned()),
                ..GalleryQuery::default()
            },
            "availability-visible-fixture",
            None,
            None,
            None,
        )
        .expect("read cached gallery page");
    assert!(snapshot.previous_cursor.is_none() && snapshot.next_cursor.is_none());
    let mut assets = snapshot.assets;
    assets.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    assets
}

fn push_peer_change(
    factory: &QueuedSourceFactory,
    root_id: &str,
    relative_path: &str,
    sequence: u64,
) {
    push_gap_batch(
        factory,
        root_id,
        crate::domain::LibraryChangeSourceBatch {
            observations: vec![crate::domain::LibraryChangeObservation {
                root_id: root_id.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                sequence,
                observed_unix_ms: now_unix_ms().expect("peer observation time"),
                kind: crate::domain::LibraryChangeObservationKind::Created,
                scope: crate::domain::LibraryChangeScope::Path,
                relative_path: relative_path.to_owned(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::LiveNotification,
            }],
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        },
    );
}

fn wait_for_peer_publication(
    production: &mut ProductionSynchronization,
    fixture: &ProductionGapFixture,
    catalog: &SqliteCatalog,
    root_id: &str,
    relative_path: &str,
) -> LibrarySynchronizationSnapshot {
    let connection =
        rusqlite::Connection::open(&fixture.storage.catalog_path).expect("peer P0 queue evidence");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = poll_runtime_with_storage(production, &fixture.storage)
            .expect("publish peer P0 change");
        let visible = catalog
            .load_incremental_location_by_relative_path(root_id, relative_path)
            .expect("query peer published media");
        let queue: Option<(String, String)> = connection
            .query_row(
                "SELECT queue.status, lanes.lane FROM library_change_queue AS queue
             JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
             WHERE queue.root_id = ?1 AND queue.relative_path = ?2 ORDER BY queue.id DESC LIMIT 1",
                rusqlite::params![root_id, relative_path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .expect("load exact peer P0 completion");
        if let Some(asset) = visible
            && queue == Some(("completed".to_owned(), "p0_live".to_owned()))
        {
            assert_eq!((asset.width, asset.height), (3, 2));
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "peer P0 publication did not complete: {queue:?}"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

fn assert_terminal_recovery(fixture: &ProductionGapFixture, gap_id: i64) {
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path)
        .expect("terminal recovery evidence");
    let recovery: (String, bool, i64, bool, bool, bool) = connection.query_row(
        "SELECT run.status, run.enumeration_complete, run.staged_entry_count, run.absence_authority,
                authority.retired_unix_ms IS NOT NULL, claim.consumed_unix_ms IS NOT NULL
         FROM library_live_gap_recovery_claims AS claim
         JOIN library_recovery_authorities AS authority ON authority.change_id = claim.recovery_change_id
         JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
         WHERE claim.gap_change_id = ?1",
        [gap_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).expect("exact completed A recovery consumer");
    assert_eq!(
        recovery,
        ("completed".to_owned(), true, 2, true, true, true)
    );
    let remaining: (i64, i64, i64, i64) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_change_queue WHERE status IN ('pending', 'leased', 'retry_wait')),
                (SELECT COUNT(*) FROM library_recovery_authorities WHERE retired_unix_ms IS NULL),
                (SELECT COUNT(*) FROM library_live_gap_recovery_claims WHERE consumed_unix_ms IS NULL),
                (SELECT COUNT(*) FROM scan_runs)",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).expect("both roots have no unresolved work or automatic full scans");
    assert_eq!(remaining, (0, 0, 0, fixture.scan_rows_before + 1));
}

fn write_peer_image(path: &Path, color: [u8; 3]) {
    image::RgbImage::from_pixel(3, 2, image::Rgb(color))
        .save_with_format(path, image::ImageFormat::Png)
        .expect("generated peer image");
}

fn file_snapshot(path: &Path) -> (Vec<u8>, SystemTime) {
    (
        std::fs::read(path).expect("fixture source bytes"),
        std::fs::metadata(path)
            .expect("fixture metadata")
            .modified()
            .expect("fixture mtime"),
    )
}

fn source_snapshot(root: &Path) -> BTreeMap<String, (Vec<u8>, SystemTime)> {
    std::fs::read_dir(root)
        .expect("owned flat fixture source")
        .map(|entry| {
            let entry = entry.expect("fixture entry");
            assert!(entry.file_type().expect("fixture entry type").is_file());
            (
                entry.file_name().into_string().expect("fixture filename"),
                file_snapshot(&entry.path()),
            )
        })
        .collect()
}
