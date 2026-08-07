use rusqlite::{Connection, params};

use crate::domain::{LibraryFolderCursor, LibraryFolderPage, LibraryFolderView, ScanError};

use super::{database_error, load_catalog_revision, normalize_relative_folder, sqlite_unsigned};

pub(super) fn load_folder_page(
    connection: &mut Connection,
    root_id: &str,
    parent_relative_path: &str,
    max_items: u32,
    after: Option<&LibraryFolderCursor>,
) -> Result<LibraryFolderPage, ScanError> {
    validate_request(root_id, parent_relative_path, max_items)?;
    let parent_relative_path = normalize_relative_folder(parent_relative_path);
    let transaction = connection.transaction().map_err(database_error)?;
    let revision = load_catalog_revision(&transaction)?;
    if after.is_some_and(|cursor| {
        cursor.revision != revision
            || cursor.root_id != root_id
            || cursor.parent_relative_path != parent_relative_path
    }) {
        return Err(ScanError::new(
            "catalog_folder_cursor_stale",
            "The catalog or folder scope changed after this folder cursor was created",
        ));
    }

    let after_relative_path = after
        .map(|cursor| cursor.relative_path.as_str())
        .unwrap_or_default();
    let sql_limit = i64::from(max_items).saturating_add(1);
    let mut statement = transaction
        .prepare(
            "WITH candidates AS (
               SELECT locations.parent_relative_path AS asset_parent,
                      CASE
                        WHEN ?1 = '' THEN
                          CASE
                            WHEN instr(locations.parent_relative_path, '/') = 0
                              THEN locations.parent_relative_path
                            ELSE substr(
                              locations.parent_relative_path,
                              1,
                              instr(locations.parent_relative_path, '/') - 1
                            )
                          END
                        ELSE ?1 || '/' ||
                          CASE
                            WHEN instr(
                              substr(locations.parent_relative_path, length(?1) + 2),
                              '/'
                            ) = 0 THEN substr(
                              locations.parent_relative_path,
                              length(?1) + 2
                            )
                            ELSE substr(
                              substr(locations.parent_relative_path, length(?1) + 2),
                              1,
                              instr(
                                substr(locations.parent_relative_path, length(?1) + 2),
                                '/'
                              ) - 1
                            )
                          END
                      END AS child_path
               FROM library_roots AS roots
               JOIN asset_locations AS locations
                 ON locations.scan_id = roots.active_scan_id
               WHERE locations.root_id = ?2
                 AND locations.parent_relative_path <> ?1
                 AND (
                   ?1 = '' OR substr(
                     locations.parent_relative_path,
                     1,
                     length(?1) + 1
                   ) = ?1 || '/'
                 )
             ), grouped AS (
               SELECT child_path,
                      SUM(CASE WHEN asset_parent = child_path THEN 1 ELSE 0 END)
                        AS direct_asset_count,
                      COUNT(*) AS descendant_asset_count
               FROM candidates
               WHERE child_path <> ''
               GROUP BY child_path
             )
             SELECT child_path, direct_asset_count, descendant_asset_count
             FROM grouped
             WHERE ?3 = '' OR child_path > ?3
             ORDER BY child_path
             LIMIT ?4",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                parent_relative_path,
                root_id,
                after_relative_path,
                sql_limit
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    let requested = usize::try_from(max_items).map_err(|_| {
        ScanError::new(
            "catalog_folder_page_limit_invalid",
            "The folder page limit is outside the supported range",
        )
    })?;
    let mut folders = Vec::new();
    for row in rows {
        let (relative_path, direct_asset_count, descendant_asset_count) =
            row.map_err(database_error)?;
        let name = relative_path
            .rsplit('/')
            .next()
            .unwrap_or(&relative_path)
            .to_owned();
        folders.push(LibraryFolderView {
            root_id: root_id.to_owned(),
            relative_path,
            name,
            direct_asset_count: sqlite_unsigned(direct_asset_count, "direct folder asset count")?,
            descendant_asset_count: sqlite_unsigned(
                descendant_asset_count,
                "descendant folder asset count",
            )?,
        });
    }
    drop(statement);

    let has_more = folders.len() > requested;
    folders.truncate(requested);
    let next_cursor = if has_more {
        folders.last().map(|folder| LibraryFolderCursor {
            revision,
            root_id: root_id.to_owned(),
            parent_relative_path: parent_relative_path.clone(),
            relative_path: folder.relative_path.clone(),
        })
    } else {
        None
    };
    transaction.commit().map_err(database_error)?;

    Ok(LibraryFolderPage {
        revision,
        root_id: root_id.to_owned(),
        parent_relative_path,
        folders,
        next_cursor,
    })
}

fn validate_request(
    root_id: &str,
    parent_relative_path: &str,
    max_items: u32,
) -> Result<(), ScanError> {
    if root_id.trim().is_empty() {
        return Err(ScanError::new(
            "catalog_root_id_invalid",
            "A library root identifier is required",
        ));
    }
    if max_items == 0 || max_items > 500 {
        return Err(ScanError::new(
            "catalog_folder_page_limit_invalid",
            "A folder page must contain between 1 and 500 items",
        ));
    }
    let parent_relative_path = normalize_relative_folder(parent_relative_path);
    if parent_relative_path
        .split('/')
        .any(|component| component == "..")
    {
        return Err(ScanError::new(
            "catalog_source_scope_invalid",
            "A folder scope must stay inside its library root",
        ));
    }
    Ok(())
}
