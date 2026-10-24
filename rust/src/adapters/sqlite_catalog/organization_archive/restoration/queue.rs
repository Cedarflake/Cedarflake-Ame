use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::{database_error, error};

mod bookmark;
mod ordering;

pub(super) fn apply(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO organization_recovery_versions
         SELECT location.retained_id, asset.retained_id, COALESCE(root.current_root_id, root.retained_root_id),
           generation.local_generation, version.byte_length, version.file_identity_scheme,
           version.file_identity_value, version.source_revision_token, version.fingerprint_scheme, version.digest,
           version.native_copy_count
         FROM recovery.archive_version_evidence AS version
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?1 AND generation.source_generation = version.source_generation
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = version.root_id
         JOIN temp.organization_restore_identifiers AS location
           ON location.kind = 'location' AND location.original_id = version.location_id
         JOIN temp.organization_restore_identifiers AS asset
           ON asset.kind = 'asset' AND asset.original_id = version.asset_id
         WHERE 1 ON CONFLICT(location_id) DO NOTHING", [archive_id],
    ).map_err(database_error)?;
    ordering::restore(connection, archive_id)?;
    connection.execute(
        "INSERT INTO organization_recovery_admission_sources
         SELECT generation.local_generation, COALESCE(root.current_root_id, root.retained_root_id)
         FROM recovery.archive_admission_source AS source
         JOIN recovery.archive_admission AS admission USING(admission_id)
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?1 AND generation.source_generation = admission.source_generation
         JOIN temp.organization_restore_roots AS root ON root.original_root_id = source.root_id
         WHERE 1 ON CONFLICT(source_generation, root_id) DO NOTHING", [archive_id],
    ).map_err(database_error)?;
    bookmark::restore(connection, archive_id)?;
    associate_current(connection)?;
    bookmark::refresh(connection)
}

fn associate_current(connection: &Connection) -> Result<(), ScanError> {
    let mut after = String::new();
    loop {
        let roots = connection.prepare(
            "SELECT id FROM library_roots WHERE id > ?1 AND active_scan_id IS NOT NULL ORDER BY id LIMIT 256",
        ).map_err(database_error)?.query_map([&after], |row| row.get::<_, String>(0))
            .map_err(database_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(database_error)?;
        let Some(last) = roots.last() else { break };
        after = last.clone();
        for root in roots {
            crate::adapters::sqlite_catalog::organization::publication::root(connection, &root)?;
        }
    }
    Ok(())
}

fn capacity_error() -> ScanError {
    error(
        "queue_capacity",
        "The catalog cannot allocate more organization queue positions",
    )
}
