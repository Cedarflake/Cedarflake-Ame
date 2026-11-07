use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;

use super::super::{current_schema, database_error};

pub(super) const DECISIONS: &str = "CREATE TABLE \"similarity_decisions\" (
    group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision >= 0)
) STRICT, WITHOUT ROWID";

pub(super) const PREFERENCES: &str = "CREATE TABLE similarity_preferences (
    group_id TEXT NOT NULL REFERENCES similarity_groups(group_id),
    fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
    PRIMARY KEY(group_id, fingerprint_scheme, digest, byte_length)
) STRICT, WITHOUT ROWID";

pub(in crate::adapters::sqlite_catalog::migrations) fn migrate(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog::migrations) fn migrate_transaction(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 75)?;
    transaction
        .execute_batch(PREFERENCES)
        .map_err(database_error)?;
    transaction
        .execute_batch(&DECISIONS.replacen(
            "similarity_decisions",
            "similarity_decisions_multiple",
            1,
        ))
        .map_err(database_error)?;
    transaction
        .execute_batch(
            "INSERT INTO similarity_preferences
           SELECT group_id, preferred_scheme, preferred_digest, preferred_byte_length
           FROM similarity_decisions WHERE preferred_scheme IS NOT NULL;
         INSERT INTO similarity_decisions_multiple
           SELECT group_id, ignored, revision FROM similarity_decisions;
         DROP TABLE similarity_decisions;
         ALTER TABLE similarity_decisions_multiple RENAME TO similarity_decisions;
         UPDATE schema_info SET version = 76; PRAGMA user_version = 76;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 76)
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    super::super::multiple_metadata_formats::downgrade_for_test(connection);
    if super::super::schema_shape::schema_version(connection).unwrap() != 76 {
        return;
    }
    let legacy = super::schema::OBJECTS
        .iter()
        .find(|(_, name, _)| *name == "similarity_decisions")
        .unwrap()
        .2;
    connection
        .execute_batch("DROP TABLE similarity_preferences; DROP TABLE similarity_decisions;")
        .unwrap();
    connection.execute_batch(legacy).unwrap();
    connection
        .execute_batch("UPDATE schema_info SET version = 75; PRAGMA user_version = 75;")
        .unwrap();
}

#[cfg(test)]
mod tests;
