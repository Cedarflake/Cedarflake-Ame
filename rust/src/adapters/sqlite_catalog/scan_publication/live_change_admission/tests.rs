use rusqlite::Connection;

use super::*;

#[test]
fn only_uncovered_current_live_debt_blocks_replacement_publication() {
    let mut connection = fixture();
    let transaction = connection.transaction().unwrap();
    let authority = authority();
    let check = || ensure_publishable(&transaction, "replacement", "root", &authority);
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
    for statement in [
        "UPDATE library_change_queue SET root_id = 'peer'",
        "UPDATE library_change_queue SET root_generation = 2",
        "UPDATE library_change_queue_lanes SET lane = 'p1_journal'",
        "UPDATE library_change_queue SET scope = 'path'",
        "UPDATE library_change_queue SET status = 'completed'",
    ] {
        transaction.execute_batch("SAVEPOINT boundary").unwrap();
        transaction.execute_batch(statement).unwrap();
        check().expect("unrelated or nonblocking work stays outside this publication");
        transaction
            .execute_batch("ROLLBACK TO boundary; RELEASE boundary")
            .unwrap();
    }
    transaction
        .execute(
            "INSERT INTO library_live_gap_recovery_claims VALUES (1, ?1, 'replacement', NULL)",
            [LiveGapRecoveryConsumer::ForegroundScan.as_str()],
        )
        .unwrap();
    check().expect("the current scan's verified claim covers the debt");
    transaction
        .execute_batch(
            "UPDATE library_live_gap_recovery_claims SET foreground_scan_id = 'other-scan'",
        )
        .unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
    transaction.execute_batch("UPDATE library_live_gap_recovery_claims SET foreground_scan_id = 'replacement', consumed_unix_ms = 123").unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
}

#[test]
fn retryable_work_waits_but_cannot_mask_an_exhausted_blocker() {
    let mut connection = fixture();
    let transaction = connection.transaction().unwrap();
    let authority = authority();
    let check = || ensure_publishable(&transaction, "replacement", "root", &authority);
    for statement in [
        "UPDATE library_change_queue SET attempt_count = 1, next_retry_unix_ms = 500",
        "UPDATE library_change_queue SET status = 'leased'",
        "UPDATE library_change_queue SET status = 'pending'",
    ] {
        transaction.execute_batch("SAVEPOINT boundary").unwrap();
        transaction.execute_batch(statement).unwrap();
        assert_eq!(
            check().unwrap_err().code,
            "catalog_scan_live_changes_pending"
        );
        transaction
            .execute_batch("ROLLBACK TO boundary; RELEASE boundary")
            .unwrap();
    }
    transaction
        .execute_batch(
            "INSERT INTO library_change_queue (id, root_id, root_generation, status, attempt_count, scope, intent_kind)
         SELECT 2, root_id, root_generation, 'pending', 0, scope, intent_kind FROM library_change_queue;
         INSERT INTO library_change_queue_lanes VALUES (2, 'p0_live');",
        )
        .unwrap();
    let error = check().unwrap_err();
    assert_eq!(error.code, "catalog_scan_live_changes_exhausted");
    assert!(error.message.contains("fixture_failure"));
    let first_import = PublicationAuthority {
        previous_active_scan: None,
        ..authority
    };
    ensure_publishable(&transaction, "first-import", "root", &first_import).unwrap();
}

#[test]
fn retained_recovery_cannot_wait_for_capacity_owned_by_a_foreground_blocked_lane() {
    let mut connection = fixture();
    let transaction = connection.transaction().unwrap();
    let authority = authority();
    let check = || ensure_publishable(&transaction, "replacement", "root", &authority);
    for failure in ["metadata_inventory_required", "change_lease_expired"] {
        transaction
            .execute(
                "UPDATE library_change_queue SET last_failure_code = ?1",
                [failure],
            )
            .unwrap();
        assert_eq!(
            check().unwrap_err().code,
            "catalog_scan_live_changes_pending"
        );
    }
    transaction.execute_batch(
        "WITH RECURSIVE slots(id) AS (SELECT 2 UNION ALL SELECT id + 1 FROM slots WHERE id < 3073)
         INSERT INTO library_change_queue (id, root_id, root_generation, status, attempt_count, scope, intent_kind)
         SELECT id, 'root', 1, 'pending', 0, 'subtree', 'reconcile' FROM slots;
         INSERT INTO library_change_queue_lanes SELECT id, 'p2_recovery' FROM library_change_queue WHERE id > 1;",
    ).unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
    transaction
        .execute_batch(
            "UPDATE library_change_queue SET status = 'retry_wait', attempt_count = 8 WHERE id > 1",
        )
        .unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
    transaction
        .execute_batch("UPDATE library_change_queue SET status = 'completed' WHERE id > 1")
        .unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_pending"
    );
    transaction
        .execute_batch(
            "UPDATE library_change_queue SET previous_relative_path = 'old-album' WHERE id = 1",
        )
        .unwrap();
    assert_eq!(
        check().unwrap_err().code,
        "catalog_scan_live_changes_exhausted"
    );
}

fn authority() -> PublicationAuthority {
    PublicationAuthority {
        previous_active_scan: Some("baseline".into()),
        root_generation: 1,
        change_queue_high_watermark: None,
        scan_owner: "foreground".into(),
        namespace_identity: None,
    }
}

fn fixture() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE library_change_queue (
           id INTEGER PRIMARY KEY, root_id TEXT, root_generation INTEGER, status TEXT,
           attempt_count INTEGER, next_retry_unix_ms INTEGER, last_failure_code TEXT,
           scope TEXT, intent_kind TEXT, lease_expires_unix_ms INTEGER,
           origin TEXT DEFAULT 'live_notification', relative_path TEXT DEFAULT 'album',
           previous_relative_path TEXT, authoritative_scan_id TEXT, superseded_by_change_id INTEGER
         );
         CREATE TABLE library_change_queue_lanes (change_id INTEGER PRIMARY KEY, lane TEXT);
         CREATE TABLE library_live_gap_recovery_claims (
           gap_change_id INTEGER, consumer_kind TEXT, foreground_scan_id TEXT, consumed_unix_ms INTEGER
         );
         CREATE TABLE library_change_root_state (root_id TEXT, generation INTEGER, is_active INTEGER);
         CREATE TABLE library_persistent_journal_root_state (
           root_id TEXT, root_generation INTEGER, capability_state TEXT, continuity_state TEXT
         );
         CREATE TABLE library_roots (id TEXT, active_scan_id TEXT);
         CREATE TABLE library_root_publication_namespaces (root_id TEXT, root_generation INTEGER);
         CREATE TABLE library_recovery_authorities (change_id INTEGER);
         CREATE TABLE library_metadata_inventory_candidate_owners (change_id INTEGER);
         INSERT INTO library_change_root_state VALUES ('root', 1, 1);
         INSERT INTO library_persistent_journal_root_state VALUES ('root', 1, 'live_only', 'live_only');
         INSERT INTO library_roots VALUES ('root', 'baseline');
         INSERT INTO library_root_publication_namespaces VALUES ('root', 1);
         INSERT INTO library_change_queue (id, root_id, root_generation, status, attempt_count,
           next_retry_unix_ms, last_failure_code, scope, intent_kind) VALUES (
           1, 'root', 1, 'retry_wait', 8, NULL, 'fixture_failure', 'subtree', 'reconcile'
         );
         INSERT INTO library_change_queue_lanes VALUES (1, 'p0_live');",
    ).unwrap();
    connection
}
