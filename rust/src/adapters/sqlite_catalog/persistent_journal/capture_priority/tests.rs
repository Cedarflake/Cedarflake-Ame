use super::*;

fn fixture() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(
        "CREATE TABLE library_roots(id TEXT PRIMARY KEY, active_scan_id TEXT);
         CREATE TABLE library_change_root_state(root_id TEXT, generation INTEGER, is_active INTEGER);
         CREATE TABLE library_change_queue(
           id INTEGER PRIMARY KEY, root_id TEXT, root_generation INTEGER, origin TEXT,
           status TEXT, intent_kind TEXT, attempt_count INTEGER, ready_unix_ms INTEGER,
           next_retry_unix_ms INTEGER);
         CREATE TABLE library_change_queue_lanes(change_id INTEGER, lane TEXT);
         CREATE TABLE library_change_queue_catch_up_lineage(change_id INTEGER, catch_up_source TEXT);
         CREATE TABLE library_persistent_journal_baselines(
           change_id INTEGER, root_id TEXT, root_generation INTEGER, phase TEXT);
         CREATE TABLE library_recovery_authorities(
           change_id INTEGER, run_id TEXT, root_id TEXT, root_generation INTEGER,
           reason TEXT, retired_unix_ms INTEGER);
         CREATE TABLE scan_runs(
           id TEXT, root_id TEXT, root_generation_at_start INTEGER, scan_owner TEXT, status TEXT);
         INSERT INTO library_roots VALUES ('peer-root', 'published-scan');
         INSERT INTO library_change_root_state VALUES ('peer-root', 1, 1);
         INSERT INTO library_change_queue VALUES
           (1, 'peer-root', 1, 'live_notification', 'pending', 'reconcile', 0, 200, NULL);
         INSERT INTO library_change_queue_lanes VALUES (1, 'p0_live');"
    ).unwrap();
    connection
}

#[test]
fn due_foreground_work_and_active_leases_preempt_capture_across_roots() {
    let connection = fixture();
    let policy = LibraryChangeQueuePolicy::default();
    assert!(!should_yield(&connection, 100, policy).unwrap());
    assert!(should_yield(&connection, 200, policy).unwrap());
    connection
        .execute("UPDATE library_change_queue SET status = 'leased'", [])
        .unwrap();
    assert!(should_yield(&connection, 100, policy).unwrap());
    connection
        .execute("UPDATE library_change_queue SET status = 'completed'", [])
        .unwrap();
    assert!(!should_yield(&connection, 200, policy).unwrap());
}

#[test]
fn retry_wait_requires_a_due_timestamp_and_remaining_attempts() {
    let connection = fixture();
    let policy = LibraryChangeQueuePolicy::default();
    connection
        .execute("UPDATE library_change_queue SET status = 'retry_wait'", [])
        .unwrap();
    assert!(!should_yield(&connection, 200, policy).unwrap());
    connection
        .execute(
            "UPDATE library_change_queue SET next_retry_unix_ms = 201",
            [],
        )
        .unwrap();
    assert!(!should_yield(&connection, 200, policy).unwrap());
    assert!(should_yield(&connection, 201, policy).unwrap());
    connection
        .execute(
            "UPDATE library_change_queue SET attempt_count = ?1",
            [policy.max_attempts],
        )
        .unwrap();
    assert!(!should_yield(&connection, 201, policy).unwrap());
}

#[test]
fn stale_unpublished_and_non_foreground_work_does_not_preempt_capture() {
    for update in [
        "UPDATE library_change_root_state SET is_active = 0",
        "UPDATE library_change_queue SET root_generation = 2",
        "UPDATE library_roots SET active_scan_id = NULL",
        "UPDATE library_change_queue SET origin = 'startup_catch_up'",
        "UPDATE library_change_queue_lanes SET lane = 'p1_journal'",
        "UPDATE library_change_queue SET intent_kind = 'freshness_unknown'",
    ] {
        let connection = fixture();
        connection.execute_batch(update).unwrap();
        assert!(!should_yield(&connection, 200, LibraryChangeQueuePolicy::default()).unwrap());
    }
}
