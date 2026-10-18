use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, migrate_v29_ambiguous_gap_fixture, publication_identity,
    seed_current_explicit_recovery_claim,
};

use tempfile::tempdir;

use crate::domain::LibraryChangeQueueHealth;
use crate::ports::LibraryChangeQueue;

use crate::adapters::sqlite_catalog::*;

fn seed_pending_journal_claim(catalog: &SqliteCatalog, root_id: &str) {
    catalog
        .connection
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET protocol_version = 5, capability_state = 'supported',
                 continuity_state = 'current', last_failure_code = NULL,
                 last_failure_message = NULL, updated_unix_ms = 50
             WHERE root_id = ?1 AND root_generation = 1",
            [root_id],
        )
        .expect("make pending-journal root current");
    catalog
        .connection
        .execute(
            "INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state,
               updated_unix_ms
             ) VALUES (
               ?1, 1, 'volume-guid', '77', 3, ?2, '44', '55', '55', 1,
               5, 1, 'current', 50
             )",
            params![root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal checkpoint");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'live_notification',
               50, 50, '2', '2', 1, 'retry_wait', 50, 0, 60,
               'live_gap_waiting_for_journal_range',
               'The P0 live gap is waiting for its durable journal range',
               1, 50, 50
             )",
            [root_id],
        )
        .expect("insert pending-journal gap");
    let gap_change_id = catalog.connection.last_insert_rowid();
    catalog
        .connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               opening_volume_guid, opening_volume_serial,
               opening_root_reference_version, opening_root_file_reference,
               opening_journal_id, opening_next_usn, protocol_version,
               contract_version, created_unix_ms
             ) VALUES (
               ?1, ?2, 1, 'pending_journal', 'volume-guid', '77', 3, ?3,
               '44', '55', 5, 1, 50
             )",
            params![gap_change_id, root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal claim");
}

#[test]
fn explicit_recovery_claim_is_consumed_only_by_its_foreground_scan_publication() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-publication-root";
    let root_path = "C:\\ExplicitPublication";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("explicit-publication-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('a'),
        )
        .expect("begin explicit foreground recovery");
    let active: (String, String, String, Option<i64>) = catalog
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.next_retry_unix_ms
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("active explicit foreground claim");
    assert_eq!(
        active,
        (
            "foreground_scan".to_owned(),
            request.scan_id.clone(),
            "leased".to_owned(),
            None,
        )
    );

    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove explicit recovery first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish explicit foreground recovery");
    drop(catalog);
    let reopened = SqliteCatalog::open(catalog_path).expect("reopen published explicit recovery");
    let consumed: (String, String, i64, String, Option<String>, i64) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id,
                    claim.consumed_unix_ms IS NOT NULL, gap.status,
                    gap.last_failure_code, gap.catalog_revision_at_success
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("consumed explicit foreground claim");
    assert_eq!(
        consumed,
        (
            "foreground_scan".to_owned(),
            request.scan_id,
            1,
            "completed".to_owned(),
            None,
            1,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load consumed explicit recovery claim metrics");
    assert_eq!(metrics.explicit_recovery_required_count, 0);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Idle);
}

#[test]
fn abandoned_explicit_recovery_scan_restores_the_original_typed_claim() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-abandon-root";
    let root_path = "C:\\ExplicitAbandon";
    let mut catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("published recovery catalog");
    let baseline_request = fixture_request("explicit-abandon-baseline", root_path);
    catalog
        .begin_scan(&baseline_request, root_id, root_path)
        .expect("begin published recovery baseline");
    catalog
        .prove_live_only_first_import_handoff_for_test(&baseline_request.scan_id)
        .expect("prove recovery baseline handoff");
    catalog
        .stage_location(
            &baseline_request.scan_id,
            root_id,
            &AssetLocationView {
                asset_id: "explicit-abandon-asset".to_owned(),
                location_id: "explicit-abandon-location".to_owned(),
                root_id: root_id.to_owned(),
                scan_id: baseline_request.scan_id.clone(),
                absolute_path: format!("{root_path}\\one.png"),
                display_path: format!("{root_path}\\one.png"),
                relative_path: "one.png".to_owned(),
                preview_path: String::new(),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 0,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Pending,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                capture_time: None,
            },
        )
        .expect("stage recovery baseline with allocated source generation");
    catalog
        .publish_scan(&baseline_request.scan_id, root_id, 1, 0)
        .expect("publish recovery baseline");
    drop(catalog);
    let mut catalog =
        SqliteCatalog::open(catalog_path.clone()).expect("reopen valid v31 recovery baseline");
    seed_current_explicit_recovery_claim(&catalog, root_id, 41);
    let request = fixture_request("explicit-abandon-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('b'),
        )
        .expect("begin explicit foreground recovery");
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("abandon explicit foreground recovery");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("reopen abandoned explicit recovery");
    let baseline: (String, i64, i64, i64) = reopened
        .connection
        .query_row(
            "SELECT root.active_scan_id, state.is_active, catalog.revision,
                (SELECT COUNT(*) FROM asset_locations AS location
                 WHERE location.root_id = root.id AND location.scan_id = root.active_scan_id)
         FROM library_roots AS root
         JOIN library_change_root_state AS state ON state.root_id = root.id
         CROSS JOIN catalog_state AS catalog WHERE root.id = ?1",
            [root_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("published baseline survives abandoned replacement");
    assert_eq!(baseline, ("explicit-abandon-baseline".to_owned(), 1, 1, 1));
    let restored: (
        String,
        Option<String>,
        String,
        Option<i64>,
        String,
        Option<String>,
    ) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.next_retry_unix_ms, gap.last_failure_code,
                    gap.authoritative_scan_id
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             WHERE claim.root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("restored explicit claim");
    assert_eq!(
        restored,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            None,
            "live_gap_v30_explicit_recovery_required".to_owned(),
            None,
        )
    );
    let metrics = reopened
        .load_library_change_root_queue_metrics(
            root_id,
            LibraryRootGeneration::initial(),
            unix_time_ms(),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("load explicit recovery claim metrics");
    assert_eq!(metrics.explicit_recovery_required_count, 1);
    assert_eq!(metrics.health, LibraryChangeQueueHealth::Degraded);
}

#[test]
fn reopening_an_interrupted_explicit_recovery_scan_restores_the_claim_atomically() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "explicit-crash-root";
    let root_path = "C:\\ExplicitCrash";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("explicit-crash-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('c'),
        )
        .expect("begin explicit foreground recovery");
    drop(catalog);

    let reopened = SqliteCatalog::open(catalog_path).expect("recover interrupted explicit scan");
    let restored: (String, Option<String>, String, String) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status, scans.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = ?2
             WHERE claim.root_id = ?1",
            params![root_id, request.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("recovered interrupted explicit claim");
    assert_eq!(
        restored,
        (
            "explicit_recovery_required".to_owned(),
            None,
            "retry_wait".to_owned(),
            "failed".to_owned(),
        )
    );
}

#[test]
fn foreground_scan_does_not_capture_a_pending_journal_claim() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "pending-journal-scan-root";
    let root_path = "C:\\PendingJournalScan";
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("catalog");
    let initial = fixture_request("pending-journal-initial", root_path);
    catalog
        .begin_scan(&initial, root_id, root_path)
        .expect("begin initial scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&initial.scan_id)
        .expect("prove pending-journal fixture first-import handoff");
    catalog
        .publish_scan(&initial.scan_id, root_id, 0, 0)
        .expect("publish initial scan");
    seed_pending_journal_claim(&catalog, root_id);

    let foreground = fixture_request("pending-journal-foreground", root_path);
    catalog
        .begin_scan(&foreground, root_id, root_path)
        .expect("begin unrelated foreground scan");
    drop(catalog);
    let mut reopened = SqliteCatalog::open(catalog_path).expect("reopen pending-journal scan");
    let retained: (String, String, Option<String>, Option<i64>) = reopened
        .connection
        .query_row(
            "SELECT claim.consumer_kind, gap.status, gap.authoritative_scan_id,
                    scans.change_queue_high_watermark
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = ?2
             WHERE claim.root_id = ?1",
            params![root_id, foreground.scan_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("retained pending-journal owner");
    assert_eq!(
        retained,
        (
            "pending_journal".to_owned(),
            "retry_wait".to_owned(),
            None,
            None,
        )
    );
    reopened
        .abandon_scan(&foreground.scan_id, "cancelled", 0)
        .expect("clean up foreground scan");
}
