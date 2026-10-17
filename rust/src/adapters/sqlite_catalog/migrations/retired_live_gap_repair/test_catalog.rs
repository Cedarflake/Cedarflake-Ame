use rusqlite::Connection;

pub(in super::super) fn seed_current_explicit_live_gap_claim(
    connection: &Connection,
    root_id: &str,
    gap_change_id: i64,
) {
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("enable explicit live-gap foreign keys");
    let root_path = format!("C:/retired-live-gap/{root_id}");
    connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
            rusqlite::params![root_id, root_path],
        )
        .expect("insert explicit live-gap root");
    connection
        .execute(
            "INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES (?1, 1, 1, 1)",
            [root_id],
        )
        .expect("insert explicit live-gap generation");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (?1, 1, 0, 1, 'unknown', 'baseline_required', 1)",
            [root_id],
        )
        .expect("insert explicit live-gap journal root");
    connection
        .execute(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, ?2, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               2, 2, ?3, ?3, 1, 'retry_wait', 2, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The retained gap requires an explicit library update',
               0, 2, 2
             )",
            rusqlite::params![gap_change_id, root_id, gap_change_id.to_string()],
        )
        .expect("insert explicit live-gap row");
    connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               created_unix_ms
             ) VALUES (?1, ?2, 1, 'explicit_recovery_required', 2)",
            rusqlite::params![gap_change_id, root_id],
        )
        .expect("insert explicit live-gap claim");
}

pub(in super::super) fn seed_current_pending_journal_live_gap_claim(
    connection: &Connection,
    root_id: &str,
    gap_change_id: i64,
) {
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("enable pending-journal foreign keys");
    let root_path = format!("C:/retired-live-gap/{root_id}");
    connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
            rusqlite::params![root_id, root_path],
        )
        .expect("insert pending-journal root");
    connection
        .execute(
            "INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES (?1, 1, 1, 1)",
            [root_id],
        )
        .expect("insert pending-journal generation");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (?1, 1, 5, 1, 'supported', 'current', 1)",
            [root_id],
        )
        .expect("insert pending-journal root authority");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_checkpoints(
               root_id, root_generation, volume_guid, volume_serial,
               root_reference_version, root_file_reference, journal_id,
               next_unread_usn, captured_exclusive_end, covered_catalog_revision,
               protocol_version, contract_version, continuity_state, updated_unix_ms
             ) VALUES (
               ?1, 1, 'volume-guid', '77', 3, ?2, '44', '55', '55', 0,
               5, 1, 'current', 1
             )",
            rusqlite::params![root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal checkpoint");
    connection
        .execute(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, ?2, 1, 'freshness_unknown', 'root', '', 'live_notification',
               2, 2, ?3, ?3, 1, 'retry_wait', 2, 0, 3,
               'live_gap_waiting_for_journal_range',
               'The live gap is waiting for its durable journal range',
               0, 2, 2
             )",
            rusqlite::params![gap_change_id, root_id, gap_change_id.to_string()],
        )
        .expect("insert pending-journal gap");
    connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               opening_volume_guid, opening_volume_serial,
               opening_root_reference_version, opening_root_file_reference,
               opening_journal_id, opening_next_usn, protocol_version,
               contract_version, created_unix_ms
             ) VALUES (
               ?1, ?2, 1, 'pending_journal', 'volume-guid', '77', 3, ?3,
               '44', '55', 5, 1, 2
             )",
            rusqlite::params![gap_change_id, root_id, vec![1_u8; 16]],
        )
        .expect("insert pending-journal claim");
}

pub(in super::super) fn retire_live_gap_root(
    connection: &Connection,
    root_id: &str,
    gap_change_id: i64,
) {
    connection
        .execute(
            "UPDATE library_change_root_state
             SET is_active = 0, updated_unix_ms = 3 WHERE root_id = ?1",
            [root_id],
        )
        .expect("retire live-gap generation");
    connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = 3
             WHERE id = ?1",
            [gap_change_id],
        )
        .expect("supersede retired live gap");
    connection
        .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
        .expect("remove retired live-gap root");
}

pub(in super::super) fn advance_live_gap_root_generation(
    connection: &Connection,
    root_id: &str,
    gap_change_id: i64,
) {
    connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'superseded', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, superseded_by_change_id = NULL,
                 updated_unix_ms = 3
             WHERE id = ?1",
            [gap_change_id],
        )
        .expect("supersede previous-generation live gap");
    connection
        .execute(
            "UPDATE library_persistent_journal_root_state
             SET capability_state = CASE
                   WHEN protocol_version = 0 THEN 'unknown' ELSE 'live_only' END,
                 continuity_state = 'unavailable',
                 last_failure_code = 'root_generation_retired',
                 last_failure_message = 'The root generation was retired',
                 updated_unix_ms = 3
             WHERE root_id = ?1 AND root_generation = 1",
            [root_id],
        )
        .expect("retire previous journal generation");
    connection
        .execute(
            "UPDATE library_change_root_state
             SET generation = 2, is_active = 1, updated_unix_ms = 3
             WHERE root_id = ?1",
            [root_id],
        )
        .expect("advance live-gap root generation");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (?1, 2, 0, 1, 'unknown', 'baseline_required', 3)",
            [root_id],
        )
        .expect("insert current journal generation");
}

pub(in super::super) fn assert_retired_live_gap_claim_repaired(
    connection: &Connection,
    gap_change_id: i64,
) {
    let evidence = connection
        .query_row(
            "SELECT gap.status, gap.superseded_by_change_id,
                    gap.last_failure_code,
                    (SELECT COUNT(*) FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = gap.id)
             FROM library_change_queue AS gap WHERE gap.id = ?1",
            [gap_change_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .expect("retained terminal live-gap evidence");
    assert_eq!(evidence.0, "superseded");
    assert_eq!(evidence.1, None);
    assert!(matches!(
        evidence.2.as_str(),
        "live_gap_v30_explicit_recovery_required" | "live_gap_waiting_for_journal_range"
    ));
    assert_eq!(evidence.3, 0);
}
