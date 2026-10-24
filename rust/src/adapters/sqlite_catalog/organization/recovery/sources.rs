use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::super::database_error;

pub(super) fn associate(connection: &Connection, root_id: &str) -> Result<(), ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_recovery_root_matches (root_id TEXT PRIMARY KEY NOT NULL) WITHOUT ROWID;
         DELETE FROM temp.organization_recovery_root_matches;",
    ).map_err(database_error)?;
    connection.execute(
        "INSERT INTO temp.organization_recovery_root_matches
         SELECT retained.root_id FROM organization_recovery_roots AS retained
         JOIN library_root_publication_namespaces AS namespace
           ON namespace.identity_scheme = retained.identity_scheme AND namespace.identity_value = retained.identity_value
         JOIN library_change_root_state AS state ON state.root_id = namespace.root_id
           AND state.generation = namespace.root_generation AND state.is_active = 1
         WHERE retained.identity_scheme = 'windows-file-id-128-v1'
         GROUP BY retained.root_id HAVING COUNT(*) = 1 AND MIN(namespace.root_id) = ?1", [root_id],
    ).map_err(database_error)?;
    for (table, key) in [
        ("organization_sources", "record_id"),
        (
            "organization_recovery_admission_sources",
            "source_generation",
        ),
        (
            "organization_recovery_duplicate_sources",
            "scheme, digest, byte_length",
        ),
    ] {
        connection.execute(&format!(
            "INSERT INTO {table}(root_id, {key})
             SELECT ?1, {key} FROM {table} WHERE root_id IN (SELECT root_id FROM temp.organization_recovery_root_matches)
             ON CONFLICT DO NOTHING"
        ), [root_id]).map_err(database_error)?;
        connection.execute(&format!(
            "DELETE FROM {table} WHERE root_id IN (SELECT root_id FROM temp.organization_recovery_root_matches)"
        ), []).map_err(database_error)?;
    }
    for table in [
        "organization_recovery_versions",
        "organization_review_scopes",
        "organization_review_bookmark",
    ] {
        connection.execute(&format!(
            "UPDATE {table} SET root_id = ?1 WHERE root_id IN (SELECT root_id FROM temp.organization_recovery_root_matches)"
        ), [root_id]).map_err(database_error)?;
    }
    Ok(())
}

pub(in super::super) fn forget_source(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection.execute(
        "DELETE FROM organization_recovery_duplicate_decisions AS choice
         WHERE EXISTS(SELECT 1 FROM organization_recovery_duplicate_sources AS owned
           WHERE owned.scheme = choice.scheme AND owned.digest = choice.digest AND owned.byte_length = choice.byte_length AND owned.root_id = ?1)
           AND NOT EXISTS(SELECT 1 FROM organization_recovery_duplicate_sources AS surviving
             JOIN library_roots AS root ON root.id = surviving.root_id
             WHERE surviving.scheme = choice.scheme AND surviving.digest = choice.digest
               AND surviving.byte_length = choice.byte_length AND surviving.root_id <> ?1)", [root_id],
    ).map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM organization_recovery_admissions AS history
         WHERE history.source_generation IN (
           SELECT source_generation FROM organization_recovery_admission_sources WHERE root_id = ?1)
           AND NOT EXISTS(SELECT 1 FROM organization_recovery_admission_sources AS source
             JOIN library_roots AS root ON root.id = source.root_id
             WHERE source.source_generation = history.source_generation AND source.root_id <> ?1)",
            [root_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM organization_recovery_versions WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM organization_recovery_admission_sources WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM organization_recovery_duplicate_sources WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(in super::super) fn transfer_source(
    connection: &Connection,
    previous: &str,
    destination: &str,
) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_recovery_duplicate_sources
         SELECT scheme, digest, byte_length, ?2 FROM organization_recovery_duplicate_sources WHERE root_id = ?1
         ON CONFLICT(scheme, digest, byte_length, root_id) DO NOTHING", params![previous, destination],
    ).map_err(database_error)?;
    connection.execute(
        "INSERT INTO organization_recovery_admission_sources
         SELECT source_generation, ?2 FROM organization_recovery_admission_sources WHERE root_id = ?1
         ON CONFLICT(source_generation, root_id) DO NOTHING", params![previous, destination],
    ).map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_recovery_versions SET root_id = ?2 WHERE root_id = ?1",
            params![previous, destination],
        )
        .map_err(database_error)?;
    Ok(())
}
