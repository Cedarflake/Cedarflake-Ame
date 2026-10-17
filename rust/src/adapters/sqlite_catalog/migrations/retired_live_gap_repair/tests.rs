use rusqlite::Connection;
use tempfile::NamedTempFile;

use crate::adapters::SqliteCatalog;

use super::super::retired_live_gap_repair::test_catalog::{
    advance_live_gap_root_generation, assert_retired_live_gap_claim_repaired, retire_live_gap_root,
    seed_current_explicit_live_gap_claim, seed_current_pending_journal_live_gap_claim,
};
use super::super::{SCHEMA_VERSION, migrate_schema};

#[test]
fn current_schema_repairs_removed_root_explicit_claim_idempotently() {
    let catalog = NamedTempFile::new().expect("retired explicit catalog");
    let mut connection = Connection::open(catalog.path()).expect("open retired catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_explicit_live_gap_claim(&connection, "removed-explicit-root", 4_001);
    retire_live_gap_root(&connection, "removed-explicit-root", 4_001);
    drop(connection);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf())
            .expect("repair removed-root explicit claim"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect repaired explicit claim");
    assert_retired_live_gap_claim_repaired(&evidence, 4_001);
    let lifecycle: (i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT generation FROM library_change_root_state
                WHERE root_id = 'removed-explicit-root'),
               (SELECT is_active FROM library_change_root_state
                WHERE root_id = 'removed-explicit-root'),
               (SELECT COUNT(*) FROM library_roots
                WHERE id = 'removed-explicit-root')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("removed-root lifecycle evidence");
    assert_eq!(lifecycle, (1, 0, 0));
    drop(evidence);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf())
            .expect("idempotently reopen repaired catalog"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect idempotent repair");
    assert_retired_live_gap_claim_repaired(&evidence, 4_001);
}

#[test]
fn current_schema_repairs_explicit_claim_from_an_older_generation() {
    let catalog = NamedTempFile::new().expect("advanced explicit catalog");
    let mut connection = Connection::open(catalog.path()).expect("open advanced catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_explicit_live_gap_claim(&connection, "advanced-explicit-root", 4_002);
    advance_live_gap_root_generation(&connection, "advanced-explicit-root", 4_002);
    drop(connection);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf())
            .expect("repair previous-generation claim"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect advanced repair");
    assert_retired_live_gap_claim_repaired(&evidence, 4_002);
    let lifecycle: (i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT generation FROM library_change_root_state
                WHERE root_id = 'advanced-explicit-root'),
               (SELECT is_active FROM library_change_root_state
                WHERE root_id = 'advanced-explicit-root'),
               (SELECT COUNT(*) FROM library_roots
                WHERE id = 'advanced-explicit-root')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("advanced-root lifecycle evidence");
    assert_eq!(lifecycle, (2, 1, 1));
}

#[test]
fn current_schema_repairs_the_retained_two_removed_one_advanced_shape() {
    let catalog = NamedTempFile::new().expect("retained live-gap catalog");
    let mut connection = Connection::open(catalog.path()).expect("open retained catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    for (root_id, gap_change_id) in [("removed-explicit-a", 4_011), ("removed-explicit-b", 4_012)] {
        seed_current_explicit_live_gap_claim(&connection, root_id, gap_change_id);
        retire_live_gap_root(&connection, root_id, gap_change_id);
    }
    seed_current_explicit_live_gap_claim(&connection, "advanced-explicit", 4_013);
    advance_live_gap_root_generation(&connection, "advanced-explicit", 4_013);
    drop(connection);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf()).expect("repair retained live-gap shape"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect retained live-gap repair");
    for gap_change_id in [4_011, 4_012, 4_013] {
        assert_retired_live_gap_claim_repaired(&evidence, gap_change_id);
    }
    let lifecycle: (i64, i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_change_root_state
                WHERE is_active = 0 AND generation = 1
                  AND root_id IN ('removed-explicit-a', 'removed-explicit-b')),
               (SELECT COUNT(*) FROM library_roots
                WHERE id IN ('removed-explicit-a', 'removed-explicit-b')),
               (SELECT generation FROM library_change_root_state
                WHERE root_id = 'advanced-explicit'),
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("retained lifecycle projection");
    assert_eq!(lifecycle, (2, 0, 2, 0));
}

#[test]
fn current_schema_repairs_retired_pending_journal_claim() {
    let catalog = NamedTempFile::new().expect("retired pending-journal catalog");
    let mut connection = Connection::open(catalog.path()).expect("open pending catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_pending_journal_live_gap_claim(&connection, "removed-pending-root", 4_021);
    retire_live_gap_root(&connection, "removed-pending-root", 4_021);
    drop(connection);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf())
            .expect("repair retired pending-journal claim"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect pending-journal repair");
    assert_retired_live_gap_claim_repaired(&evidence, 4_021);
}

#[test]
fn v30_to_v31_repairs_retired_live_gap_claim_before_validation() {
    let catalog = NamedTempFile::new().expect("v30 retired claim catalog");
    let mut connection = Connection::open(catalog.path()).expect("open v30 claim catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_explicit_live_gap_claim(&connection, "v30-removed-root", 4_041);
    retire_live_gap_root(&connection, "v30-removed-root", 4_041);
    super::super::downgrade_source_revision_contract_to_v30_for_test(&connection);
    drop(connection);

    drop(SqliteCatalog::open(catalog.path().to_path_buf()).expect("migrate repaired v30 claim"));
    let evidence = Connection::open(catalog.path()).expect("inspect migrated claim repair");
    assert_eq!(
        evidence
            .query_row("SELECT version FROM schema_info", [], |row| row
                .get::<_, i64>(0))
            .expect("migrated schema version"),
        SCHEMA_VERSION,
    );
    assert_retired_live_gap_claim_repaired(&evidence, 4_041);
}

#[test]
fn current_schema_preserves_legal_explicit_and_pending_journal_claims() {
    let catalog = NamedTempFile::new().expect("legal live-gap catalog");
    let mut connection = Connection::open(catalog.path()).expect("open legal claim catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_explicit_live_gap_claim(&connection, "legal-explicit-root", 4_051);
    seed_current_pending_journal_live_gap_claim(&connection, "legal-pending-root", 4_052);
    drop(connection);

    drop(
        SqliteCatalog::open(catalog.path().to_path_buf()).expect("validate legal live-gap claims"),
    );
    let evidence = Connection::open(catalog.path()).expect("inspect legal live-gap claims");
    let retained: Vec<(String, String, String)> = evidence
        .prepare(
            "SELECT claim.root_id, claim.consumer_kind, gap.status
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             ORDER BY claim.root_id",
        )
        .expect("legal claim statement")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("legal claim rows")
        .collect::<Result<_, _>>()
        .expect("collect legal claims");
    assert_eq!(
        retained,
        vec![
            (
                "legal-explicit-root".to_owned(),
                "explicit_recovery_required".to_owned(),
                "retry_wait".to_owned(),
            ),
            (
                "legal-pending-root".to_owned(),
                "pending_journal".to_owned(),
                "retry_wait".to_owned(),
            ),
        ],
    );
}

#[test]
fn current_schema_live_gap_repair_rolls_back_when_another_claim_is_invalid() {
    let catalog = NamedTempFile::new().expect("mixed valid-invalid claim catalog");
    let mut connection = Connection::open(catalog.path()).expect("open rollback catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    seed_current_explicit_live_gap_claim(&connection, "repairable-removed-root", 4_061);
    retire_live_gap_root(&connection, "repairable-removed-root", 4_061);
    seed_current_explicit_live_gap_claim(&connection, "invalid-active-root", 4_062);
    connection
        .execute(
            "UPDATE library_change_queue SET status = 'superseded' WHERE id = 4062",
            [],
        )
        .expect("make active-generation claim invalid");
    drop(connection);

    let error = match SqliteCatalog::open(catalog.path().to_path_buf()) {
        Ok(_) => panic!("invalid peer claim must roll back safe repair"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
    let evidence = Connection::open(catalog.path()).expect("inspect rolled-back live-gap repair");
    let retained: (i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                WHERE gap_change_id = 4061),
               (SELECT COUNT(*) FROM library_live_gap_recovery_claims
                WHERE gap_change_id = 4062)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("rolled-back claim evidence");
    assert_eq!(retained, (1, 1));
}

#[test]
fn current_schema_live_gap_repair_keeps_malformed_states_fail_closed() {
    for case in 0..6 {
        let catalog = NamedTempFile::new().expect("malformed live-gap catalog");
        let mut connection =
            Connection::open(catalog.path()).expect("open malformed live-gap catalog");
        migrate_schema(&mut connection).expect("create current catalog");
        let gap_change_id = 4_100 + case;
        let root_id = format!("malformed-live-gap-{case}");
        if case == 5 {
            seed_current_pending_journal_live_gap_claim(&connection, &root_id, gap_change_id);
        } else {
            seed_current_explicit_live_gap_claim(&connection, &root_id, gap_change_id);
        }
        match case {
            0 => {
                connection
                    .execute(
                        "UPDATE library_change_queue
                         SET status = 'superseded' WHERE id = ?1",
                        [gap_change_id],
                    )
                    .expect("make active same-generation claim invalid");
            }
            1 => {
                advance_live_gap_root_generation(&connection, &root_id, gap_change_id);
                let successor_id = gap_change_id + 100;
                connection
                    .execute(
                        "INSERT INTO library_change_queue(
                           id, root_id, root_generation, intent_kind, scope, relative_path,
                           origin, first_observed_unix_ms, most_recent_observed_unix_ms,
                           first_sequence, most_recent_sequence,
                           coalesced_observation_count, status, ready_unix_ms,
                           catalog_revision_at_enqueue, catalog_revision_at_success,
                           created_unix_ms, updated_unix_ms
                         ) VALUES (
                           ?1, ?2, 2, 'reconcile', 'path', 'successor.jpg',
                           'user_refresh', 4, 4, ?3, ?3, 1, 'completed', 4,
                           0, 0, 4, 4
                         )",
                        rusqlite::params![successor_id, root_id, successor_id.to_string()],
                    )
                    .expect("insert explicit successor evidence");
                connection
                    .execute(
                        "UPDATE library_change_queue
                         SET superseded_by_change_id = ?1 WHERE id = ?2",
                        rusqlite::params![successor_id, gap_change_id],
                    )
                    .expect("attach forbidden explicit successor");
            }
            2 => {
                retire_live_gap_root(&connection, &root_id, gap_change_id);
                connection
                    .execute(
                        "UPDATE library_change_queue
                         SET last_failure_code = 'wrong_explicit_failure',
                             last_failure_message = 'Wrong explicit failure'
                         WHERE id = ?1",
                        [gap_change_id],
                    )
                    .expect("corrupt explicit failure evidence");
            }
            3 => {
                retire_live_gap_root(&connection, &root_id, gap_change_id);
                connection
                    .execute("DELETE FROM library_live_gap_recovery_contract", [])
                    .expect("remove live-gap marker");
            }
            4 => {
                retire_live_gap_root(&connection, &root_id, gap_change_id);
                let original = connection
                    .query_row(
                        "SELECT sql FROM sqlite_master
                         WHERE type = 'table'
                           AND name = 'library_live_gap_recovery_claims'",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .expect("canonical live-gap claim DDL");
                let modified = original.replacen("'foreground_scan'", "'foreground_scan_other'", 1);
                assert_ne!(modified, original);
                connection
                    .execute_batch("PRAGMA writable_schema = ON")
                    .expect("enable controlled live-gap DDL mutation");
                connection
                    .execute(
                        "UPDATE sqlite_master SET sql = ?1
                         WHERE type = 'table'
                           AND name = 'library_live_gap_recovery_claims'",
                        [modified],
                    )
                    .expect("mutate live-gap claim DDL");
                connection
                    .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1410")
                    .expect("publish controlled live-gap DDL mutation");
            }
            5 => {
                retire_live_gap_root(&connection, &root_id, gap_change_id);
                connection
                    .execute(
                        "UPDATE library_change_queue
                         SET last_failure_code = 'wrong_pending_journal_failure',
                             last_failure_message = 'Wrong pending-journal failure'
                         WHERE id = ?1",
                        [gap_change_id],
                    )
                    .expect("corrupt pending-journal failure evidence");
            }
            _ => unreachable!(),
        }
        drop(connection);

        let error = match SqliteCatalog::open(catalog.path().to_path_buf()) {
            Ok(_) => panic!("malformed live-gap case {case} must fail closed"),
            Err(error) => error,
        };
        assert_eq!(
            error.code, "catalog_live_gap_recovery_contract_unverifiable",
            "unexpected error for malformed live-gap case {case}",
        );
        let evidence = Connection::open(catalog.path()).expect("inspect malformed live-gap row");
        let retained = evidence
            .query_row(
                "SELECT COUNT(*) FROM library_live_gap_recovery_claims
                 WHERE gap_change_id = ?1",
                [gap_change_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("retained malformed claim");
        assert_eq!(retained, 1, "case {case} must remain unchanged");
    }
}
