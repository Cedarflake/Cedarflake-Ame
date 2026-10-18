use super::*;

#[test]
fn one_time_baseline_admission_is_atomic_and_idempotent() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog = catalog_with_roots(directory.path().join("catalog.sqlite3"), &["root-a"]);
    require_baseline_root(&mut catalog, "root-a");
    let request = baseline_request("root-a", 10);

    let first = catalog
        .begin_persistent_journal_baseline(&request, policy())
        .expect("admit baseline");
    let replay = catalog
        .begin_persistent_journal_baseline(&request, policy())
        .expect("replay baseline admission");
    let counts: (i64, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_queue),
               (SELECT COUNT(*) FROM library_recovery_authorities),
               (SELECT COUNT(*) FROM library_persistent_journal_baselines)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("baseline durable counts");

    assert_eq!(first, replay);
    assert_eq!(first.phase, PersistentJournalBaselinePhase::Inventory);
    assert_eq!(counts, (1, 1, 1));
    assert_eq!(
        catalog
            .load_persistent_journal_baselines()
            .expect("active baselines"),
        vec![first]
    );
}

#[test]
fn first_import_opening_boundary_survives_restart_before_enumeration() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("first-import-after-opening", "root-first-a");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .begin_scan(&request, "root-first-a", &request.root_path)
        .expect("begin first import");
    let baseline = first_import_baseline_request(&request, "root-first-a", 10);
    catalog
        .begin_persistent_journal_baseline(&baseline, policy())
        .expect("persist opening boundary before enumeration");
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopen after opening boundary");
    assert!(
        reopened
            .first_import_change_capture_is_ready(
                &request.scan_id,
                "root-first-a",
                LibraryRootGeneration::initial(),
            )
            .expect("reload first-import authority")
    );
    let resumed = reopened
        .resume_scan(&request, "root-first-a", &request.root_path)
        .expect("resume the same foreground scan");
    assert_eq!(resumed.visited_entries, 0);
    assert_eq!(
        reopened
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("metadata inventory count"),
        0
    );
}

#[test]
fn first_import_published_baseline_survives_restart_without_second_inventory() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("first-import-after-publish", "root-first-c");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .begin_scan(&request, "root-first-c", &request.root_path)
        .expect("begin first import");
    catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, "root-first-c", 10),
            policy(),
        )
        .expect("persist opening boundary");
    catalog
        .publish_scan(&request.scan_id, "root-first-c", 0, 0)
        .expect("publish first inventory as baseline");
    drop(catalog);

    let reopened = SqliteCatalog::open(path).expect("reopen after first publication");
    assert!(
        reopened
            .metadata_inventory_is_waiting_for_closing_boundary(&request.scan_id)
            .expect("first import awaits only the closing boundary")
    );
    assert_eq!(
        reopened
            .connection
            .query_row(
                "SELECT COUNT(*) FROM library_metadata_inventory_runs",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("metadata inventory count"),
        0
    );
}

#[test]
fn first_import_replay_restart_waits_for_the_replayed_live_queue() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let request = first_import_scan_request("first-import-replay", "root-first-replay");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .begin_scan(&request, "root-first-replay", &request.root_path)
        .expect("begin first import");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, "root-first-replay", 10),
            policy(),
        )
        .expect("persist opening boundary");
    catalog
        .publish_scan(&request.scan_id, "root-first-replay", 0, 0)
        .expect("publish first inventory");
    catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            10,
        ))
        .expect("enter replay phase with an empty journal range");
    let checkpoint = catalog
        .load_persistent_journal_checkpoint("root-first-replay", LibraryRootGeneration::initial())
        .expect("empty replay checkpoint")
        .expect("captured replay checkpoint");
    assert!(
        catalog
            .record_persistent_journal_empty_replay(&checkpoint, 2_001)
            .expect("record successful empty replay")
    );
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: "root-first-replay".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "during-scan.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 2_001,
                most_recent_observed_unix_ms: 2_001,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_001,
            policy(),
        )
        .expect("persist live work before the replay-phase restart");
    drop(catalog);

    let mut reopened = SqliteCatalog::open(path).expect("reopen during replay");
    assert!(
        !reopened
            .finalize_ready_first_import_journal_baseline(3_000)
            .expect("pending live work must block first-import completion")
    );
    let live = reopened
        .lease_path_library_changes_in_lane(
            "root-first-replay",
            LibraryRootGeneration::initial(),
            LibraryChangeLane::Live,
            3_001,
            policy(),
        )
        .expect("lease persisted live work after restart")
        .pop()
        .expect("persisted live work");
    let catalog_revision = reopened
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("catalog revision after first publication");
    reopened
        .complete_library_change(
            live.change.id,
            live.lease_generation,
            u64::try_from(catalog_revision).expect("non-negative catalog revision"),
            3_002,
        )
        .expect("complete replayed live work");
    assert!(
        reopened
            .finalize_ready_first_import_journal_baseline(3_003)
            .expect("finalize first-import replay")
    );
    assert!(
        reopened
            .load_persistent_journal_baselines()
            .expect("load active baselines after completion")
            .is_empty()
    );
    let (baseline_phase, authority_retired_unix_ms) = reopened
        .connection
        .query_row(
            "SELECT baseline.phase, authority.retired_unix_ms
             FROM library_persistent_journal_baselines AS baseline
             JOIN library_recovery_authorities AS authority
               ON authority.change_id = baseline.change_id
             WHERE baseline.change_id = ?1",
            [i64::try_from(baseline.change_id.value()).expect("stored change ID")],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .expect("load durable completed baseline");
    assert_eq!(baseline_phase, "completed");
    assert_eq!(authority_retired_unix_ms, Some(3_003));
    assert_eq!(
        reopened
            .load_persistent_journal_checkpoint(
                "root-first-replay",
                LibraryRootGeneration::initial(),
            )
            .expect("load current checkpoint")
            .expect("current checkpoint")
            .continuity,
        PersistentJournalContinuityState::Current
    );
}
