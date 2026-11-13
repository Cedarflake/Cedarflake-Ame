use rusqlite::Connection;

use super::super::migrate_schema;

#[test]
fn clean_current_schema_validates_once_with_full_authority_audit() {
    let mut connection = Connection::open_in_memory().expect("clean catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("PRAGMA query_only = ON")
        .expect("make clean current catalog read only");

    super::reset_current_schema_validation_count();
    super::super::source_revision_contract::reset_row_audit_count();
    super::reset_current_schema_row_audit_count();
    migrate_schema(&mut connection).expect("validate clean current catalog");

    assert_eq!(super::current_schema_validation_count(), 1);
    assert_eq!(super::super::source_revision_contract::row_audit_count(), 1);
    assert!(super::current_schema_row_audit_count() > 0);
}

#[test]
fn current_v31_change_catch_up_ddl_damage_still_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("DROP INDEX scan_run_catch_up_lineage_evidence;")
        .expect("damage catch-up DDL");

    super::reset_current_schema_row_audit_count();
    let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

    assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    assert_eq!(super::current_schema_row_audit_count(), 0);
}

#[test]
fn shared_relationship_audit_rejects_classification_and_work_orphans() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();

    for mutation in [
        "UPDATE classification_state SET active_rules = printf('%064d', 0)",
        "INSERT INTO classification_stopped_runs(run_id) VALUES ('missing-run')",
    ] {
        let transaction = connection.transaction().unwrap();
        transaction.execute_batch(mutation).unwrap();
        let error = super::validate_current_schema_contract(&transaction)
            .expect_err("reject an orphan before accepting classification content");
        assert_eq!(
            error.code,
            "catalog_persistent_journal_contract_unverifiable"
        );
        transaction.rollback().unwrap();
    }
    super::validate_current_schema_contract(&connection).unwrap();
}
