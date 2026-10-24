use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    (
        "table",
        "organization_admissions",
        "CREATE TABLE organization_admissions (
           admission_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(admission_id > 0),
           source_generation INTEGER NOT NULL UNIQUE CHECK(source_generation > 0)
         )",
    ),
    (
        "table",
        "organization_admission_roots",
        "CREATE TABLE organization_admission_roots (
           root_key INTEGER PRIMARY KEY CHECK(root_key > 0),
           root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE
         )",
    ),
    (
        "table",
        "organization_admission_sources",
        "CREATE TABLE organization_admission_sources (
           admission_id INTEGER NOT NULL REFERENCES organization_admissions(admission_id) ON DELETE CASCADE,
           root_key INTEGER NOT NULL REFERENCES organization_admission_roots(root_key) ON DELETE CASCADE,
           PRIMARY KEY(admission_id, root_key)
         ) WITHOUT ROWID",
    ),
    (
        "index",
        "organization_admission_sources_root",
        "CREATE INDEX organization_admission_sources_root
           ON organization_admission_sources(root_key, admission_id)",
    ),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 50)?;
    for (_, _, ddl) in OBJECTS {
        transaction.execute_batch(ddl).map_err(database_error)?;
    }
    super::super::organization::admission::seed_current_catalog(transaction)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 51; PRAGMA user_version = 51;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 51)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, ddl) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    for (_, name, _) in OBJECTS.iter().filter(|(kind, _, _)| *kind == "table") {
        let orphaned: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check(?1))",
                [name],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if orphaned {
            return Err(invalid());
        }
    }
    let unowned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_admissions AS admitted
             WHERE NOT EXISTS (
               SELECT 1 FROM organization_admission_sources AS owner
               WHERE owner.admission_id = admitted.admission_id
             ))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if unowned {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_organization_admission_invalid",
        "Organization admission requires ordered versions with configured source ownership",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v50_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 51 {
        for (kind, name, _) in OBJECTS.iter().rev() {
            connection
                .execute_batch(&format!("DROP {kind} {name}"))
                .unwrap();
        }
        connection
            .execute_batch("UPDATE schema_info SET version = 50; PRAGMA user_version = 50;")
            .unwrap();
    }
}

#[cfg(test)]
mod tests;
