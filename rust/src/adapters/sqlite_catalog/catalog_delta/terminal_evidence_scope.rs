use std::collections::{HashMap, HashSet};

use rusqlite::{Transaction, params, params_from_iter};

use crate::domain::{CatalogDeltaBatch, LibraryChangeId, LibraryChangeScope, ScanError};

use super::{super::database_error, MAX_DELTA_MUTATIONS};

pub(super) struct CompletedLeasePaths {
    scope: LibraryChangeScope,
    current: String,
    previous: Option<String>,
}

impl CompletedLeasePaths {
    pub(super) fn new(
        scope: &str,
        intent_kind: &str,
        current: String,
        previous: Option<String>,
    ) -> Result<Self, ScanError> {
        if scope == "subtree" && previous.is_some() != (intent_kind == "rename_candidate") {
            return Err(ScanError::new(
                "catalog_delta_rename_scope_invalid",
                "Only a paired subtree rename can own a previous subtree",
            ));
        }
        let scope = match scope {
            "path" => LibraryChangeScope::Path,
            "subtree" => LibraryChangeScope::Subtree,
            "root" => LibraryChangeScope::Root,
            _ => {
                return Err(ScanError::new(
                    "catalog_delta_scope_invalid",
                    "The completed lease has an unknown path scope",
                ));
            }
        };
        Ok(Self {
            scope,
            current,
            previous,
        })
    }

    pub(super) fn accepts_terminal_path(&self, path: &str) -> bool {
        if !is_relative_file_path(path) {
            return false;
        }
        match self.scope {
            LibraryChangeScope::Path => path == self.current,
            LibraryChangeScope::Subtree => {
                is_in_subtree(path, &self.current)
                    || self
                        .previous
                        .as_deref()
                        .is_some_and(|previous| is_in_subtree(path, previous))
            }
            LibraryChangeScope::Root => self.current.is_empty() && self.previous.is_none(),
        }
    }

    fn exact_paths(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.current.as_str()).chain(self.previous.as_deref())
    }

    fn retained_terminal_paths(
        &self,
        transaction: &Transaction<'_>,
        root_id: &str,
    ) -> Result<Vec<String>, ScanError> {
        if self.scope == LibraryChangeScope::Path {
            return Ok(self.exact_paths().map(str::to_owned).collect());
        }
        let mut paths = Vec::new();
        for scope in self.exact_paths() {
            let mut statement = transaction
                .prepare(
                    "SELECT relative_path FROM library_terminal_media_evidence
                     WHERE root_id = ?1 AND (
                       ?2 = '' OR relative_path = ?2
                       OR (relative_path >= ?3 AND relative_path < ?4)
                     )
                     ORDER BY relative_path LIMIT ?5",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map(
                    params![
                        root_id,
                        scope,
                        format!("{scope}/"),
                        format!("{scope}0"),
                        (MAX_DELTA_MUTATIONS + 1) as i64,
                    ],
                    |row| row.get::<_, String>(0),
                )
                .map_err(database_error)?;
            for row in rows {
                paths.push(row.map_err(database_error)?);
                if paths.len() > MAX_DELTA_MUTATIONS {
                    return Err(ScanError::new(
                        "metadata_inventory_required",
                        "The completed scope has more terminal evidence than one bounded delta can retire",
                    ));
                }
            }
        }
        Ok(paths)
    }
}

fn is_relative_file_path(path: &str) -> bool {
    !path.contains(['\0', '\\', ':'])
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

fn is_in_subtree(path: &str, subtree: &str) -> bool {
    is_relative_file_path(subtree)
        && (path == subtree
            || path
                .strip_prefix(subtree)
                .is_some_and(|suffix| suffix.starts_with('/')))
}

pub(super) fn invalidate_changed_paths(
    transaction: &Transaction<'_>,
    batch: &CatalogDeltaBatch,
    active_scan_id: &str,
    completed_paths: &HashMap<LibraryChangeId, CompletedLeasePaths>,
    affected_location_ids: &HashSet<String>,
) -> Result<(), ScanError> {
    let mut exact_paths = HashSet::new();
    for paths in completed_paths.values() {
        exact_paths.extend(paths.retained_terminal_paths(transaction, &batch.root_id)?);
        if exact_paths.len() > MAX_DELTA_MUTATIONS {
            return Err(ScanError::new(
                "metadata_inventory_required",
                "Terminal evidence retirement exceeds one bounded delta",
            ));
        }
    }
    exact_paths.extend(
        batch
            .mutations
            .iter()
            .filter_map(|mutation| mutation.upsert_location.as_ref())
            .map(|location| location.relative_path.clone()),
    );
    let placeholders = |count: usize| {
        std::iter::repeat_n("?", count)
            .collect::<Vec<_>>()
            .join(", ")
    };
    // Both sets come from the bounded delta. Resolve removed paths before deleting their locations.
    let statement = format!(
        "DELETE FROM library_terminal_media_evidence
         WHERE root_id = ? AND (
           relative_path IN ({})
           OR relative_path IN (
             SELECT relative_path FROM asset_locations
             WHERE root_id = ? AND scan_id = ? AND location_id IN ({})
           )
         )",
        placeholders(exact_paths.len()),
        placeholders(affected_location_ids.len()),
    );
    let parameters = std::iter::once(batch.root_id.as_str())
        .chain(exact_paths.iter().map(String::as_str))
        .chain([batch.root_id.as_str(), active_scan_id])
        .chain(affected_location_ids.iter().map(String::as_str));
    transaction
        .execute(&statement, params_from_iter(parameters))
        .map_err(database_error)?;
    Ok(())
}

#[cfg(test)]
mod tests;
