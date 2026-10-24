use rusqlite::{params, params_from_iter};

use crate::domain::ScanError;
use crate::domain::organization::{
    MAX_ORGANIZATION_SELECTION, OrganizationSelection, stale_selection, validate_identifier,
    validate_selections,
};
use crate::domain::organization_conflicts::{
    OrganizationConflictAlbum, OrganizationConflictAlbums, OrganizationConflictAnnotations,
    OrganizationConflictStatus,
};
use crate::ports::organization_conflicts::OrganizationConflictRepository;

use super::super::exact_duplicates::read_unsigned;
use super::super::{SqliteCatalog, database_error, load_catalog_revision};
use super::{inspection, selection};

impl OrganizationConflictRepository for SqliteCatalog {
    fn load_organization_conflict_albums(
        &self,
        selected: &OrganizationSelection,
        after: Option<&str>,
    ) -> Result<OrganizationConflictAlbums, ScanError> {
        validate_selections(std::slice::from_ref(selected))?;
        if let Some(after) = after {
            validate_identifier(after)?;
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let organization_revision = transaction
            .query_row(
                "SELECT revision FROM organization_state WHERE singleton = 1",
                [],
                |row| read_unsigned(row, 0),
            )
            .map_err(database_error)?;
        let keys = selection::selected_keys(&transaction, std::slice::from_ref(selected))?;
        let inspected =
            inspection::inspect_group(&transaction, keys.first().ok_or_else(stale_selection)?)?;
        let mut albums = if let Some(version) = inspected.version {
            let mut statement = transaction.prepare(
                "SELECT album.album_id, album.name, retired.album_id IS NOT NULL
                 FROM organization_conflicts AS conflict
                 JOIN organization_albums AS album ON album.album_id = conflict.album_id
                 LEFT JOIN organization_album_retirements AS retired ON retired.album_id = album.album_id
                 WHERE conflict.record_id = ?1 AND conflict.album_id > ?2
                 ORDER BY conflict.album_id LIMIT 65"
            ).map_err(database_error)?;
            statement
                .query_map(params![version.record_id, after.unwrap_or("")], |row| {
                    Ok(OrganizationConflictAlbum {
                        album_id: row.get(0)?,
                        name: row.get(1)?,
                        retired: row.get(2)?,
                    })
                })
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?
        } else {
            Vec::new()
        };
        let next_album_id = if albums.len() > 64 {
            albums.truncate(64);
            albums.last().map(|album| album.album_id.clone())
        } else {
            None
        };
        transaction.commit().map_err(database_error)?;
        Ok(OrganizationConflictAlbums {
            organization_revision,
            albums,
            next_album_id,
        })
    }

    fn load_organization_conflict_status(&self) -> Result<OrganizationConflictStatus, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let catalog_revision = load_catalog_revision(&transaction)?;
        let conflicting_groups = transaction.query_row(
            "SELECT COUNT(DISTINCT conflict.record_id) FROM organization_conflicts AS conflict
             WHERE EXISTS (
               SELECT 1 FROM organization_record_groups AS lineage
               JOIN organization_bindings AS binding ON binding.record_id = lineage.record_id
               JOIN asset_locations AS location
                 ON location.source_generation = binding.source_generation
                   AND location.asset_id = binding.asset_id AND location.file_size = binding.byte_length
                   AND location.file_identity_scheme IS binding.file_identity_scheme
                   AND location.file_identity_value IS binding.file_identity_value
                   AND location.source_revision_token IS binding.source_revision_token
               JOIN library_roots AS root ON root.active_scan_id = location.scan_id
               WHERE lineage.group_id = conflict.record_id
             )",
            [],
            |row| read_unsigned(row, 0),
        ).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(OrganizationConflictStatus {
            catalog_revision,
            conflicting_groups,
        })
    }

    fn load_organization_conflict_annotations(
        &self,
        revision: u64,
        location_ids: &[String],
    ) -> Result<OrganizationConflictAnnotations, ScanError> {
        if location_ids.len() > MAX_ORGANIZATION_SELECTION {
            return Err(ScanError::new(
                "organization_selection_invalid",
                "Conflict annotations accept at most 256 locations",
            ));
        }
        for id in location_ids {
            validate_identifier(id)?;
        }
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let catalog_revision = load_catalog_revision(&transaction)?;
        if catalog_revision < revision {
            return Err(ScanError::new(
                "catalog_revision_changed",
                "Conflict annotations cannot precede the requested catalog revision",
            ));
        }
        let location_ids = if location_ids.is_empty() {
            Vec::new()
        } else {
            let placeholders = vec!["?"; location_ids.len()].join(",");
            let record = super::super::gallery::organization::record_expression("location");
            let mut statement = transaction.prepare(&format!(
                "SELECT location.location_id FROM asset_locations AS location
                 JOIN library_roots AS root ON root.active_scan_id = location.scan_id
                 WHERE location.location_id IN ({placeholders}) AND EXISTS (
                   SELECT 1 FROM organization_conflicts AS conflict WHERE conflict.record_id = {record}
                 ) ORDER BY location.location_id"
            )).map_err(database_error)?;
            statement
                .query_map(params_from_iter(location_ids), |row| row.get(0))
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?
        };
        transaction.commit().map_err(database_error)?;
        Ok(OrganizationConflictAnnotations {
            catalog_revision,
            location_ids,
        })
    }
}
