use std::path::Path;

use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::file_operation_journal::invalid;
use crate::domain::file_operation_publication::relative_output_path;
use crate::domain::{AssetLocationView, IncrementalCatalogRoot, ScanError};

use super::super::super::{database_error, sqlite_integer, sqlite_unsigned};

pub(super) struct PublicationRoot {
    pub id: String,
    pub scan_id: String,
    pub running_scan_id: Option<String>,
    path: String,
    generation: u64,
}

pub(super) fn load(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<Option<PublicationRoot>, ScanError> {
    let row: Option<(String, String, Option<String>, i64)> = transaction.query_row(
        "SELECT r.path, r.active_scan_id,
           (SELECT id FROM scan_runs WHERE root_id = r.id AND status IN ('running', 'paused') LIMIT 1), s.generation
         FROM library_roots AS r JOIN library_change_root_state AS s ON s.root_id = r.id
         JOIN scan_runs AS scan ON scan.id = r.active_scan_id AND scan.status = 'completed'
         WHERE r.id = ?1 AND s.is_active = 1",
        [id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(database_error)?;
    row.map(|(path, scan_id, running_scan_id, generation)| {
        Ok(PublicationRoot {
            id: id.to_owned(),
            path,
            scan_id,
            running_scan_id,
            generation: sqlite_unsigned(generation, "root generation")?,
        })
    })
    .transpose()
}

impl PublicationRoot {
    pub fn revalidate(&self, expected: &IncrementalCatalogRoot) -> Result<(), ScanError> {
        if self.id != expected.root_id
            || self.generation != expected.root_generation.value()
            || self.path != expected.root_path
        {
            return Err(invalid("The destination root changed before publication"));
        }
        Ok(())
    }

    pub fn update_count(&self, transaction: &Transaction<'_>, delta: i64) -> Result<(), ScanError> {
        let updated = transaction.execute(
            "UPDATE scan_runs SET asset_count = asset_count + ?2 WHERE id = ?1 AND status = 'completed' AND asset_count + ?2 >= 0",
            params![self.scan_id, delta],
        ).map_err(database_error)?;
        if updated != 1 {
            return Err(invalid(
                "The published location count cannot accept this change",
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_location(
    root: &PublicationRoot,
    location: &AssetLocationView,
) -> Result<(), ScanError> {
    let relative = Path::new(&location.relative_path);
    if location.root_id != root.id
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
        || relative_output_path(&root.path, &location.absolute_path).as_deref()
            != Some(location.relative_path.as_str())
        || location.source_generation != 0
    {
        return Err(invalid(
            "The destination location does not belong to its current root",
        ));
    }
    sqlite_integer(location.file_size, "destination size")?;
    Ok(())
}
