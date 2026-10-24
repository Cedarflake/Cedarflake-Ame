use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::database_error;

const CURRENT_VERSIONS: &str = "asset_locations AS location
    JOIN library_roots AS root ON root.id = location.root_id
      AND root.active_scan_id = location.scan_id";

pub(in crate::adapters::sqlite_catalog) fn publish_root(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    prepare_batch(connection)?;
    connection
        .execute(
            &format!(
                "INSERT INTO temp.organization_admission_batch
                 SELECT DISTINCT location.source_generation FROM {CURRENT_VERSIONS}
                 WHERE location.root_id = ?1"
            ),
            [root_id],
        )
        .map_err(database_error)?;
    publish_batch(connection)
}

pub(in crate::adapters::sqlite_catalog) fn publish_generation(
    connection: &Connection,
    generation: i64,
) -> Result<(), ScanError> {
    prepare_batch(connection)?;
    connection
        .execute(
            &format!(
                "INSERT INTO temp.organization_admission_batch
                 SELECT DISTINCT location.source_generation FROM {CURRENT_VERSIONS}
                 WHERE location.source_generation = ?1"
            ),
            [generation],
        )
        .map_err(database_error)?;
    publish_batch(connection)
}

pub(in crate::adapters::sqlite_catalog) fn seed_current_catalog(
    connection: &Connection,
) -> Result<(), ScanError> {
    prepare_batch(connection)?;
    connection
        .execute(
            &format!(
                "INSERT INTO temp.organization_admission_batch
                 SELECT DISTINCT location.source_generation FROM {CURRENT_VERSIONS}"
            ),
            [],
        )
        .map_err(database_error)?;
    publish_batch(connection)
}

fn prepare_batch(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_admission_batch (
               source_generation INTEGER PRIMARY KEY NOT NULL
             );
             DELETE FROM temp.organization_admission_batch;",
        )
        .map_err(database_error)
}

fn publish_batch(connection: &Connection) -> Result<(), ScanError> {
    // Exclude existing versions before inserting: an ignored AUTOINCREMENT insert would still
    // advance sqlite_sequence on every unchanged scan. The temporary batch also bounds history joins.
    connection
        .execute_batch(
            "INSERT INTO organization_admissions(source_generation)
             SELECT source_generation FROM temp.organization_admission_batch AS incoming
             WHERE NOT EXISTS (
               SELECT 1 FROM organization_admissions AS admitted
               WHERE admitted.source_generation = incoming.source_generation
             ) ORDER BY source_generation;

             INSERT INTO organization_admission_roots(root_id)
             SELECT DISTINCT location.root_id FROM temp.organization_admission_batch AS incoming
             CROSS JOIN asset_locations AS location
               ON location.source_generation = incoming.source_generation
             JOIN library_roots AS root
               ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             WHERE NOT EXISTS (
               SELECT 1 FROM organization_admission_roots AS known
               WHERE known.root_id = location.root_id
             );

             INSERT OR IGNORE INTO organization_admission_sources(admission_id, root_key)
             SELECT admitted.admission_id, owner.root_key
             FROM temp.organization_admission_batch AS incoming
             CROSS JOIN organization_admissions AS admitted
               ON admitted.source_generation = incoming.source_generation
             CROSS JOIN asset_locations AS location
               ON location.source_generation = incoming.source_generation
             JOIN library_roots AS root
               ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             JOIN organization_admission_roots AS owner ON owner.root_id = root.id;",
        )
        .map_err(database_error)
}

pub(super) fn forget_source(connection: &Connection, root_id: &str) -> Result<(), ScanError> {
    connection
        .execute(
            "DELETE FROM organization_admissions
             WHERE admission_id IN (
               SELECT source.admission_id FROM organization_admission_roots AS root
               JOIN organization_admission_sources AS source ON source.root_key = root.root_key
               WHERE root.root_id = ?1
             ) AND NOT EXISTS (
               SELECT 1 FROM organization_admission_sources AS surviving
               JOIN organization_admission_roots AS root ON root.root_key = surviving.root_key
               WHERE surviving.admission_id = organization_admissions.admission_id
                 AND root.root_id <> ?1
             )",
            [root_id],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "DELETE FROM organization_admission_roots WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn transfer_source(
    connection: &Connection,
    previous_root: &str,
    destination_root: &str,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT OR IGNORE INTO organization_admission_roots(root_id) VALUES (?1)",
            [destination_root],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "INSERT OR IGNORE INTO organization_admission_sources(admission_id, root_key)
             SELECT source.admission_id, destination.root_key
             FROM organization_admission_roots AS previous
             JOIN organization_admission_sources AS source ON source.root_key = previous.root_key
             JOIN organization_admission_roots AS destination ON destination.root_id = ?2
             WHERE previous.root_id = ?1",
            [previous_root, destination_root],
        )
        .map_err(database_error)?;
    Ok(())
}
