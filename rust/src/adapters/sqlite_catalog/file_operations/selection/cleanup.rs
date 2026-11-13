use rusqlite::Connection;

use crate::domain::AssetLocationView;
use crate::domain::ScanError;
use crate::domain::file_operation_journal::FILE_OPERATION_PAGE_SIZE;
use crate::domain::file_operation_selection::{
    DuplicateCleanupGroup, DuplicateCleanupKind, DuplicateCleanupSummary,
};
use crate::domain::organization::{stale_selection, validate_identifier};
use crate::ports::CatalogRepository;

use super::super::super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

mod gallery;
mod queries;
pub(super) use gallery::kind as gallery_kind;

pub(super) fn retained_page(
    catalog: &SqliteCatalog,
    after_location: &str,
) -> Result<Vec<AssetLocationView>, ScanError> {
    let mut statement = catalog
        .connection
        .prepare(
            "SELECT DISTINCT kept.location_id, kept.asset_id, kept.source_generation
         FROM temp.cleanup_members AS kept
         WHERE kept.keep = 1 AND kept.location_id > ?1
           AND EXISTS(SELECT 1 FROM temp.cleanup_members AS member
             JOIN temp.file_operation_selection AS selected USING(location_id)
             WHERE member.group_id = kept.group_id)
         ORDER BY kept.location_id LIMIT ?2",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            rusqlite::params![after_location, FILE_OPERATION_PAGE_SIZE as i64],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    rows.map(|row| {
        let (id, asset, generation) = row.map_err(database_error)?;
        let location = catalog
            .load_active_location(&id)?
            .ok_or_else(stale_selection)?;
        if location.asset_id != asset
            || location.source_generation != sqlite_unsigned(generation, "kept generation")?
        {
            return Err(stale_selection());
        }
        Ok(location)
    })
    .collect()
}

pub(super) fn initialize(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE cleanup_requested(group_id TEXT PRIMARY KEY) WITHOUT ROWID;
         CREATE TEMP TABLE cleanup_members(
           group_id TEXT NOT NULL, content_key TEXT NOT NULL, asset_id TEXT NOT NULL,
           location_id TEXT NOT NULL, source_generation INTEGER NOT NULL,
           root_generation INTEGER NOT NULL, file_size INTEGER NOT NULL, keep INTEGER NOT NULL,
           PRIMARY KEY(group_id, location_id)) WITHOUT ROWID;
         CREATE TEMP TABLE cleanup_groups(
           group_id TEXT PRIMARY KEY, keeps INTEGER NOT NULL) WITHOUT ROWID;
         CREATE INDEX temp.cleanup_kept_source ON cleanup_members(keep, source_generation);",
        )
        .map_err(database_error)
}

pub(super) fn append(
    connection: &Connection,
    kind: &DuplicateCleanupKind,
    groups: &[DuplicateCleanupGroup],
) -> Result<(), ScanError> {
    if groups.len() > 256 {
        return Err(stale_selection());
    }
    let transaction = connection.unchecked_transaction().map_err(database_error)?;
    for group in groups {
        let id = match (kind, group) {
            (DuplicateCleanupKind::Exact, DuplicateCleanupGroup::Exact { key }) => {
                if key.digest.len() != 64
                    || !key.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(stale_selection());
                }
                sqlite_integer(key.byte_length, "content length")?;
                format!("{}:{}", key.digest, key.byte_length)
            }
            (DuplicateCleanupKind::Similar { .. }, DuplicateCleanupGroup::Similar { group_id }) => {
                validate_identifier(group_id)?;
                group_id.clone()
            }
            _ => return Err(stale_selection()),
        };
        transaction
            .execute(
                "INSERT OR IGNORE INTO temp.cleanup_requested VALUES (?1)",
                [id],
            )
            .map_err(database_error)?;
    }
    transaction.commit().map_err(database_error)
}

pub(super) fn prepare(
    connection: &Connection,
    kind: &DuplicateCleanupKind,
    all_groups: bool,
    gallery_query: Option<&crate::domain::GalleryQuery>,
) -> Result<DuplicateCleanupSummary, ScanError> {
    let members = if let Some(query) = gallery_query {
        gallery::require_ready(connection, query, all_groups)?;
        gallery::members(query, all_groups, false)
    } else {
        queries::members(kind, all_groups, false)
    };
    connection
        .execute(
            &format!("INSERT INTO temp.cleanup_members {}", members.sql),
            rusqlite::params_from_iter(members.parameters),
        )
        .map_err(database_error)?;
    let count = match kind {
        DuplicateCleanupKind::Exact => "COUNT(*)",
        DuplicateCleanupKind::Similar { .. } => "COUNT(DISTINCT content_key)",
    };
    connection.execute_batch(&format!(
        "INSERT INTO temp.cleanup_groups
         SELECT group_id, SUM(keep) FROM temp.cleanup_members
         GROUP BY group_id HAVING {count} > 1;
         INSERT INTO temp.file_operation_selection(asset_id, location_id, source_generation, root_generation, file_size)
         SELECT DISTINCT member.asset_id, member.location_id, member.source_generation,
                         member.root_generation, member.file_size
         FROM temp.cleanup_members AS member
         JOIN temp.cleanup_groups AS groups ON groups.group_id = member.group_id AND groups.keeps > 0
         WHERE member.keep = 0 AND NOT EXISTS (
           SELECT 1 FROM temp.cleanup_members AS retained
           WHERE retained.keep = 1 AND retained.source_generation = member.source_generation)
         ORDER BY member.location_id;"
    )).map_err(database_error)?;
    let (groups, skipped, kept): (i64, i64, i64) = connection
        .query_row(
            "WITH affected_groups AS (
             SELECT DISTINCT member.group_id FROM temp.cleanup_members AS member
             JOIN temp.file_operation_selection AS selected USING(location_id))
           SELECT
           (SELECT COUNT(*) FROM affected_groups),
           (SELECT COUNT(*) FROM temp.cleanup_groups WHERE keeps = 0),
           (SELECT COUNT(DISTINCT location_id) FROM temp.cleanup_members
             JOIN affected_groups USING(group_id) WHERE keep = 1)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(database_error)?;
    Ok(DuplicateCleanupSummary {
        group_count: sqlite_unsigned(groups, "cleanup groups")?,
        skipped_groups: sqlite_unsigned(skipped, "cleanup skipped groups")?,
        kept_files: sqlite_unsigned(kept, "cleanup kept files")?,
    })
}

pub(super) fn validate(
    connection: &Connection,
    kind: &DuplicateCleanupKind,
    gallery_query: Option<&crate::domain::GalleryQuery>,
) -> Result<(), ScanError> {
    // Additions cannot enlarge the confirmed selection; removals or changed keep decisions retire it.
    let current = if let Some(query) = gallery_query {
        gallery::members(query, false, true)
    } else {
        queries::members(kind, false, true)
    };
    let changed: bool = connection
        .query_row(
            &format!(
                "WITH current_members AS ({}) SELECT EXISTS(
               SELECT member.* FROM temp.cleanup_members AS member
               WHERE EXISTS(SELECT 1 FROM temp.cleanup_members AS affected
                 JOIN temp.file_operation_selection AS selected USING(location_id)
                 WHERE affected.group_id = member.group_id)
               EXCEPT SELECT * FROM current_members)",
                current.sql
            ),
            rusqlite::params_from_iter(current.parameters),
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if changed {
        Err(stale_selection())
    } else {
        Ok(())
    }
}
