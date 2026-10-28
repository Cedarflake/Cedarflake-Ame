use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::sidebar_order::{SidebarOrderKind, SidebarOrderPlacement};
use crate::ports::sidebar_order::SidebarOrderRepository;

use super::{SqliteCatalog, database_error};

impl SidebarOrderRepository for SqliteCatalog {
    fn reorder_sidebar_entry(
        &mut self,
        kind: SidebarOrderKind,
        entry_id: &str,
        anchor_id: &str,
        placement: SidebarOrderPlacement,
    ) -> Result<(), ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let current = position(&transaction, kind, entry_id)?;
        let anchor = position(&transaction, kind, anchor_id)?;
        if entry_id == anchor_id {
            return transaction.commit().map_err(database_error);
        }
        let target = match placement {
            SidebarOrderPlacement::Before if current < anchor => anchor - 1,
            SidebarOrderPlacement::Before => anchor,
            SidebarOrderPlacement::After if current > anchor => anchor + 1,
            SidebarOrderPlacement::After => anchor,
        };
        if target != current {
            let (table, key) = storage(kind);
            let sql = format!(
                "UPDATE {table} SET position = CASE
                   WHEN {key} = ?1 THEN ?3
                   WHEN ?2 < ?3 THEN position - 1 ELSE position + 1 END
                 WHERE {key} = ?1 OR position BETWEEN MIN(?2, ?3) AND MAX(?2, ?3)"
            );
            transaction
                .execute(&sql, params![entry_id, current, target])
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }
}

fn storage(kind: SidebarOrderKind) -> (&'static str, &'static str) {
    match kind {
        SidebarOrderKind::Albums => ("organization_sidebar_album_order", "album_id"),
        SidebarOrderKind::SourceFolders => ("library_sidebar_root_order", "root_id"),
    }
}

fn position(connection: &Connection, kind: SidebarOrderKind, id: &str) -> Result<i64, ScanError> {
    let sql = match kind {
        SidebarOrderKind::Albums => {
            "SELECT position FROM organization_sidebar_album_order WHERE album_id = ?1
             AND NOT EXISTS(SELECT 1 FROM organization_album_retirements WHERE album_id = ?1)"
        }
        SidebarOrderKind::SourceFolders => {
            "SELECT position FROM library_sidebar_root_order WHERE root_id = ?1"
        }
    };
    connection
        .query_row(sql, [id], |row| row.get(0))
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "sidebar_order_entry_unavailable",
                "A reordered item is no longer in the sidebar",
            )
        })
}

#[cfg(test)]
mod tests;
