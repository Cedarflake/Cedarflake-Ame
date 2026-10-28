use rusqlite::Transaction;

use crate::domain::{LibraryRootAvailability, LibraryRootView, ScanError};

use super::{database_error, sqlite_unsigned, user_visible_path};

pub(super) fn load_root_views(
    transaction: &Transaction<'_>,
) -> Result<Vec<LibraryRootView>, ScanError> {
    let mut statement = transaction
        .prepare(
            "SELECT roots.id, roots.path, roots.active_scan_id, roots.created_unix_ms,
                    COALESCE(scans.asset_count, 0), COALESCE(scans.issue_count, 0)
             FROM library_sidebar_root_order AS ordering
             JOIN library_roots AS roots ON roots.id = ordering.root_id
             LEFT JOIN scan_runs AS scans ON scans.id = roots.active_scan_id
             ORDER BY ordering.position, roots.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?;
    let mut roots = Vec::new();
    for row in rows {
        let (root_id, path, active_scan_id, created_unix_ms, asset_count, issue_count) =
            row.map_err(database_error)?;
        roots.push(LibraryRootView {
            root_id,
            display_path: user_visible_path(&path),
            path,
            active_scan_id,
            created_unix_ms,
            asset_count: sqlite_unsigned(asset_count, "asset count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
            availability: LibraryRootAvailability::Unknown,
            availability_message: None,
        });
    }
    Ok(roots)
}
