use crate::adapters::sqlite_catalog::tests::support::fixture_request;

use tempfile::tempdir;

use crate::domain::{
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeScope,
};
use crate::ports::LibraryChangeQueue;

use crate::adapters::sqlite_catalog::*;

#[test]
fn legacy_automatic_scans_are_retired_while_foreground_checkpoint_remains_recoverable() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let first_authoritative = fixture_request("sync-recovery-a", "C:\\RecoveryA");
    let second_authoritative = fixture_request("sync-recovery-b", "C:\\RecoveryB");
    let foreground = fixture_request("foreground-c", "C:\\ForegroundC");
    catalog
        .begin_authoritative_scan(
            &first_authoritative,
            "root-recovery-a",
            &first_authoritative.root_path,
        )
        .expect("first authoritative scan");
    catalog
        .begin_authoritative_scan(
            &second_authoritative,
            "root-recovery-b",
            &second_authoritative.root_path,
        )
        .expect("second authoritative scan");
    catalog
        .begin_scan(&foreground, "root-foreground-c", &foreground.root_path)
        .expect("foreground scan");
    let first_recovery = catalog
        .load_authoritative_recoverable_scan_after(None)
        .expect("first authoritative recovery")
        .expect("first authoritative scan");
    let second_recovery = catalog
        .load_authoritative_recoverable_scan_after(Some(&first_recovery.scan_id))
        .expect("second authoritative recovery")
        .expect("second authoritative scan");
    let end_of_page = catalog
        .load_authoritative_recoverable_scan_after(Some(&second_recovery.scan_id))
        .expect("authoritative recovery end");
    let ownership_error = catalog
        .resume_scan(
            &first_authoritative,
            "root-recovery-a",
            &first_authoritative.root_path,
        )
        .expect_err("foreground lifecycle cannot claim authoritative scan");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET status = 'paused' WHERE id = ?1",
            [&second_authoritative.scan_id],
        )
        .expect("pause second authoritative scan");
    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy automatic scans");
    let foreground_recovery = catalog
        .load_recoverable_scan()
        .expect("foreground recovery")
        .expect("foreground scan is recoverable");
    let remaining_authoritative = catalog
        .load_authoritative_recoverable_scan_after(None)
        .expect("remaining authoritative recovery");
    let retired_statuses: Vec<String> = catalog
        .connection
        .prepare("SELECT status FROM scan_runs WHERE id IN (?1, ?2) ORDER BY id")
        .expect("retired status statement")
        .query_map(
            rusqlite::params![first_authoritative.scan_id, second_authoritative.scan_id],
            |row| row.get(0),
        )
        .expect("retired status rows")
        .collect::<Result<_, _>>()
        .expect("retired statuses");

    assert_eq!(first_recovery.scan_id, first_authoritative.scan_id);
    assert_eq!(second_recovery.scan_id, second_authoritative.scan_id);
    assert!(end_of_page.is_none());
    assert_eq!(ownership_error.code, "catalog_scan_resume_mismatch");
    assert_eq!(retired, 2);
    assert_eq!(foreground_recovery.scan_id, foreground.scan_id);
    assert!(remaining_authoritative.is_none());
    assert_eq!(
        retired_statuses,
        vec!["superseded".to_owned(), "superseded".to_owned()]
    );
}

#[test]
fn legacy_root_audit_and_its_exclusive_recovery_scan_are_retired() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "legacy-audit-root";
    let root_path = "C:\\LegacyAudit";
    let initial = fixture_request("legacy-audit-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove legacy audit first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .expect("enqueue legacy audit");
    let recovery = fixture_request("legacy-audit-recovery", root_path);
    catalog
        .begin_authoritative_scan(&recovery, root_id, root_path)
        .expect("begin legacy audit recovery scan");

    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy audit");
    let scan_status: String = catalog
        .connection
        .query_row(
            "SELECT status FROM scan_runs WHERE id = ?1",
            [&recovery.scan_id],
            |row| row.get(0),
        )
        .expect("retired scan status");
    let metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 2_000, policy)
        .expect("retired queue metrics");

    assert_eq!(retired, 1);
    assert_eq!(scan_status, "superseded");
    assert!(
        catalog
            .load_authoritative_recoverable_scan_after(None)
            .expect("recoverable scans")
            .is_none()
    );
    assert_eq!(metrics.pending_count, 0);
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.superseded_count, 1);

    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "retry.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 3_000,
                most_recent_observed_unix_ms: 3_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            3_000,
            policy,
        )
        .expect("enqueue historical path retry");

    assert_eq!(
        catalog
            .retire_legacy_automatic_full_scans(4_000)
            .expect("preserve historical path retry"),
        0
    );
    let retained = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 4_000, policy)
        .expect("retained path metrics");
    assert_eq!(retained.pending_count, 1);
}

#[test]
fn legacy_automatic_full_scan_releases_live_gap_to_metadata_inventory() {
    let directory = tempdir().expect("catalog directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    let root_id = "legacy-live-gap-root";
    let root_path = "C:\\LegacyLiveGap";
    let initial = fixture_request("legacy-live-gap-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove legacy live-gap first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    let generation = LibraryRootGeneration::initial();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root_id.to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .expect("enqueue live evidence gap");
    let recovery = fixture_request("legacy-live-gap-recovery", root_path);
    catalog
        .begin_authoritative_scan(&recovery, root_id, root_path)
        .expect("begin legacy automatic full scan");
    let leased_metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, 1_000, policy)
        .expect("leased queue metrics");

    let retired = catalog
        .retire_legacy_automatic_full_scans(2_000)
        .expect("retire legacy automatic full scan");
    let released_unix_ms = unix_time_ms();
    let released_metrics = catalog
        .load_library_change_root_queue_metrics(root_id, generation, released_unix_ms, policy)
        .expect("released queue metrics");
    let active_scan_id: String = catalog
        .connection
        .query_row(
            "SELECT active_scan_id FROM library_roots WHERE id = ?1",
            [root_id],
            |row| row.get(0),
        )
        .expect("active published scan");
    let leased = catalog
        .lease_authoritative_library_change(root_id, generation, released_unix_ms, policy)
        .expect("lease released inventory authority")
        .expect("inventory authority remains queued");

    assert_eq!(leased_metrics.leased_count, 1);
    assert_eq!(retired, 1);
    assert_eq!(released_metrics.pending_count, 1);
    assert_eq!(released_metrics.leased_count, 0);
    assert_eq!(released_metrics.freshness_unknown_count, 1);
    assert_eq!(active_scan_id, initial.scan_id);
    assert_eq!(
        leased.change.intent.kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(leased.change.intent.scope, LibraryChangeScope::Root);
    assert!(
        catalog
            .load_authoritative_recoverable_scan_after(None)
            .expect("legacy recoverable scans")
            .is_none()
    );
}
