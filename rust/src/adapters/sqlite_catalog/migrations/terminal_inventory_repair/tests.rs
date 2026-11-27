use rusqlite::Connection;

use crate::adapters::SqliteCatalog;

use super::super::terminal_inventory_repair::test_catalog::{
    assert_current_terminal_inventory_repaired, current_terminal_inventory_authority_catalog,
};

#[test]
fn current_schema_repairs_orphaned_terminal_inventory_authority_before_validation() {
    let catalog = current_terminal_inventory_authority_catalog();
    let catalog_path = catalog.path().to_path_buf();

    let reopened = SqliteCatalog::open(catalog_path.clone())
        .expect("reopen current terminal inventory catalog");
    drop(reopened);
    let evidence = Connection::open(&catalog_path).expect("open repaired current evidence");
    assert_current_terminal_inventory_repaired(&evidence);
    drop(evidence);

    let reopened = SqliteCatalog::open(catalog_path.clone())
        .expect("idempotently reopen current terminal inventory catalog");
    drop(reopened);
    let evidence = Connection::open(catalog_path).expect("reopen repaired current evidence");
    assert_current_terminal_inventory_repaired(&evidence);
}

#[test]
fn current_schema_repair_rolls_back_when_active_inventory_has_no_authority() {
    let catalog = current_terminal_inventory_authority_catalog();
    let catalog_path = catalog.path().to_path_buf();
    let connection = Connection::open(&catalog_path).expect("open invalid active fixture");
    connection
        .execute(
            "DELETE FROM library_recovery_authorities WHERE change_id = 301",
            [],
        )
        .expect("remove active inventory authority");
    drop(connection);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(_) => panic!("unowned active inventory must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
    let evidence = Connection::open(catalog_path).expect("open rolled-back repair evidence");
    let retained: (i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools
                WHERE run_id = 'terminal-spool-inventory'),
               (SELECT COUNT(*) FROM library_recovery_authorities
                WHERE run_id = 'active-inventory' AND retired_unix_ms IS NULL)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load rolled-back repair evidence");
    assert_eq!(retained, (1, 1, 0));
}

#[test]
fn current_schema_repair_does_not_write_through_malformed_inventory_ddl() {
    let catalog = current_terminal_inventory_authority_catalog();
    let catalog_path = catalog.path().to_path_buf();
    let connection = Connection::open(&catalog_path).expect("open malformed inventory fixture");
    let original = connection
        .query_row(
            "SELECT sql FROM sqlite_master
             WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("canonical inventory run DDL");
    let modified = original.replacen(
        "CHECK(enumeration_complete = 1 OR absence_authority = 0)",
        "CHECK(1)",
        1,
    );
    assert_ne!(modified, original);
    connection
        .execute_batch("PRAGMA writable_schema = ON")
        .expect("enable controlled inventory DDL mutation");
    connection
        .execute(
            "UPDATE sqlite_master SET sql = ?1
             WHERE type = 'table' AND name = 'library_metadata_inventory_runs'",
            [modified],
        )
        .expect("weaken inventory run DDL");
    connection
        .execute_batch("PRAGMA writable_schema = OFF; PRAGMA schema_version = 1300")
        .expect("publish controlled inventory DDL mutation");
    drop(connection);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(_) => panic!("malformed inventory DDL must not be normalized through"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_persistent_journal_contract_unverifiable"
    );
    let evidence = Connection::open(catalog_path).expect("open malformed repair evidence");
    let retained: (i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT absence_authority FROM library_metadata_inventory_runs
                WHERE id = 'terminal-inventory'),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools
                WHERE run_id = 'terminal-spool-inventory')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load malformed repair evidence");
    assert_eq!(retained, (1, 1));
}
