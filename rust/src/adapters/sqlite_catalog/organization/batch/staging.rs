use rusqlite::{Connection, params, params_from_iter};

use super::super::super::exact_duplicates::read_unsigned;
use super::super::super::{database_error, gallery, sqlite_integer};
use super::super::selection;
use crate::domain::organization::{
    OrganizationGroupKey, OrganizationGroupTarget, OrganizationSelection, stale_selection,
};
use crate::domain::{GalleryQuery, ScanError};

pub(super) const GROUP_WINDOW: usize = 64;

pub(super) struct StagedGroup {
    pub sequence: i64,
    pub key: OrganizationGroupKey,
    pub target: Option<OrganizationGroupTarget>,
}

pub(super) fn initialize(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "PRAGMA temp_store = FILE;
         PRAGMA temp.cache_size = -2048;
         CREATE TEMP TABLE organization_batch_ids(asset_id TEXT PRIMARY KEY) WITHOUT ROWID;
         CREATE TEMP TABLE organization_batch_locations(
           location_id TEXT PRIMARY KEY, asset_id TEXT NOT NULL, source_generation INTEGER NOT NULL
         ) WITHOUT ROWID;
         CREATE INDEX temp.organization_batch_location_assets ON organization_batch_locations(asset_id);
         CREATE TEMP TABLE organization_batch_groups(
           sequence INTEGER PRIMARY KEY, digest TEXT NOT NULL, byte_length INTEGER NOT NULL,
           asset_id TEXT NOT NULL, source_generation INTEGER NOT NULL,
           representative_asset TEXT, representative_location TEXT, representative_generation INTEGER,
           location_count INTEGER, membership_authority TEXT,
           UNIQUE(digest, byte_length, asset_id, source_generation)
         );",
        )
        .map_err(database_error)
}

pub(super) fn materialize(
    connection: &Connection,
    query: &GalleryQuery,
    all_matching: bool,
) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "DELETE FROM temp.organization_batch_groups;
         DELETE FROM temp.organization_batch_locations;",
        )
        .map_err(database_error)?;
    let selected = gallery::selection::selected_locations(query, all_matching)?;
    connection
        .execute(
            &format!(
                "INSERT INTO temp.organization_batch_locations(asset_id, location_id, source_generation) {}",
                selected.sql,
            ),
            params_from_iter(selected.parameters),
        )
        .map_err(database_error)?;
    if !all_matching {
        let missing: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM temp.organization_batch_ids AS selected
             WHERE NOT EXISTS(SELECT 1 FROM temp.organization_batch_locations AS location
                              WHERE location.asset_id = selected.asset_id))",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if missing {
            return Err(stale_selection());
        }
    }
    let mut after = String::new();
    loop {
        let window = {
            let mut statement = connection
                .prepare_cached(
                    "SELECT asset_id, location_id, source_generation
                 FROM temp.organization_batch_locations WHERE location_id > ?1
                 ORDER BY location_id LIMIT 256",
                )
                .map_err(database_error)?;
            statement
                .query_map([&after], |row| {
                    Ok(OrganizationSelection {
                        asset_id: row.get(0)?,
                        location_id: row.get(1)?,
                        source_generation: read_unsigned(row, 2)?,
                    })
                })
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?
        };
        let Some(last) = window.last() else {
            break;
        };
        after.clone_from(&last.location_id);
        for key in selection::selected_keys(connection, &window)? {
            let (digest, byte_length, asset_id, generation) = match key {
                OrganizationGroupKey::Confirmed {
                    digest,
                    byte_length,
                } => (digest, byte_length, String::new(), 0),
                OrganizationGroupKey::Provisional {
                    asset_id,
                    source_generation,
                } => (String::new(), 0, asset_id, source_generation),
            };
            connection
                .prepare_cached(
                    "INSERT OR IGNORE INTO temp.organization_batch_groups
                       (digest, byte_length, asset_id, source_generation)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(database_error)?
                .execute(params![
                    digest,
                    sqlite_integer(byte_length, "byte length")?,
                    asset_id,
                    sqlite_integer(generation, "source generation")?,
                ])
                .map_err(database_error)?;
        }
    }
    Ok(())
}

pub(super) fn groups(connection: &Connection, after: i64) -> Result<Vec<StagedGroup>, ScanError> {
    let mut statement = connection
        .prepare_cached(
            "SELECT sequence, digest, byte_length, asset_id, source_generation,
                representative_asset, representative_location, representative_generation,
                location_count, membership_authority
         FROM temp.organization_batch_groups WHERE sequence > ?1 ORDER BY sequence LIMIT ?2",
        )
        .map_err(database_error)?;
    statement
        .query_map(params![after, GROUP_WINDOW as i64], |row| {
            let digest: String = row.get(1)?;
            let key = if digest.is_empty() {
                OrganizationGroupKey::Provisional {
                    asset_id: row.get(3)?,
                    source_generation: read_unsigned(row, 4)?,
                }
            } else {
                OrganizationGroupKey::Confirmed {
                    digest,
                    byte_length: read_unsigned(row, 2)?,
                }
            };
            let target = match row.get::<_, Option<String>>(9)? {
                Some(membership_authority) => Some(OrganizationGroupTarget {
                    key: key.clone(),
                    representative: OrganizationSelection {
                        asset_id: row.get(5)?,
                        location_id: row.get(6)?,
                        source_generation: read_unsigned(row, 7)?,
                    },
                    location_count: read_unsigned(row, 8)?,
                    membership_authority,
                }),
                None => None,
            };
            Ok(StagedGroup {
                sequence: row.get(0)?,
                key,
                target,
            })
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)
}

pub(super) fn retain_target(
    connection: &Connection,
    sequence: i64,
    target: &OrganizationGroupTarget,
) -> Result<(), ScanError> {
    connection
        .prepare_cached(
            "UPDATE temp.organization_batch_groups
             SET representative_asset = ?1, representative_location = ?2, representative_generation = ?3,
                 location_count = ?4, membership_authority = ?5
             WHERE sequence = ?6",
        )
        .map_err(database_error)?
        .execute(
            params![
                target.representative.asset_id,
                target.representative.location_id,
                sqlite_integer(target.representative.source_generation, "source generation")?,
                sqlite_integer(target.location_count, "group size")?,
                target.membership_authority,
                sequence,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}
