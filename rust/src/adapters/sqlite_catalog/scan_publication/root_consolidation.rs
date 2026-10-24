use std::collections::BTreeSet;
use std::path::Path;

use rusqlite::{Transaction, params};

use crate::domain::{ConsolidatedLibraryRoot, LibraryRootGeneration, ScanError};
use crate::ports::{RootConsolidationPlan, RootConsolidationRegistration};

use super::super::{
    SqliteCatalog, database_error, root_unregistration, sqlite_integer, sqlite_unsigned,
    stored_file_identity,
};
use super::PublicationInterruption;

impl SqliteCatalog {
    pub(crate) fn load_root_consolidation_registrations(
        &self,
        parent_root_id: &str,
    ) -> Result<Vec<RootConsolidationRegistration>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT root.id, root.path, root.active_scan_id, state.generation, state.is_active,
                    EXISTS(SELECT 1 FROM scan_runs AS scan
                           WHERE scan.root_id = root.id AND scan.status IN ('running', 'paused')),
                    namespace.identity_scheme, namespace.identity_value
             FROM library_roots AS root
             JOIN library_change_root_state AS state ON state.root_id = root.id
             LEFT JOIN library_root_publication_namespaces AS namespace
               ON namespace.root_id = root.id AND namespace.root_generation = state.generation
             WHERE root.id <> ?1 ORDER BY root.id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([parent_root_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(database_error)?;
        let mut roots = Vec::new();
        for row in rows {
            let (
                root_id,
                root_path,
                active_scan_id,
                generation,
                is_active,
                has_running_scan,
                scheme,
                value,
            ) = row.map_err(database_error)?;
            let root_generation =
                LibraryRootGeneration::new(sqlite_unsigned(generation, "covered root generation")?)
                    .ok_or_else(changed)?;
            roots.push(RootConsolidationRegistration {
                root_id,
                root_path,
                active_scan_id,
                root_generation,
                is_active,
                has_running_scan,
                publication_root_identity: stored_file_identity(scheme, value)?,
            });
        }
        Ok(roots)
    }
}

pub(super) fn require_current(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    plan: &RootConsolidationPlan,
) -> Result<(), ScanError> {
    if plan.scan_id != scan_id || plan.destination_root_id != root_id {
        return Err(changed());
    }
    let parent_matches = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_roots
               WHERE id = ?1 AND path = ?2 AND active_scan_id IS NULL
             )",
            params![root_id, plan.destination_path],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !parent_matches {
        return Err(changed());
    }

    let mut statement = transaction
        .prepare("SELECT id, path FROM library_roots WHERE id <> ?1 ORDER BY id")
        .map_err(database_error)?;
    let rows = statement
        .query_map([root_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    let mut current_ids = BTreeSet::new();
    for row in rows {
        let (id, path) = row.map_err(database_error)?;
        if let Ok(relative) = Path::new(&path).strip_prefix(&plan.destination_path)
            && !relative.as_os_str().is_empty()
        {
            current_ids.insert(id);
        }
    }
    let expected_ids: BTreeSet<_> = plan
        .candidates
        .iter()
        .map(|candidate| candidate.expected.root_id.clone())
        .collect();
    if current_ids != expected_ids || expected_ids.len() != plan.candidates.len() {
        return Err(changed());
    }
    for candidate in &plan.candidates {
        let root = &candidate.expected;
        let identity = root.publication_root_identity.as_ref();
        let relative = Path::new(&root.root_path)
            .strip_prefix(&plan.destination_path)
            .map_err(|_| changed())?;
        if relative.as_os_str().is_empty()
            || relative.to_string_lossy().replace('\\', "/") != candidate.relative_path
            || root.has_running_scan
        {
            return Err(changed());
        }
        let matches = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_roots AS root
                   JOIN library_change_root_state AS state ON state.root_id = root.id
                   LEFT JOIN library_root_publication_namespaces AS namespace
                     ON namespace.root_id = root.id AND namespace.root_generation = state.generation
                   WHERE root.id = ?1 AND root.path = ?2 AND root.active_scan_id IS ?3
                     AND state.generation = ?4 AND state.is_active = ?5
                     AND namespace.identity_scheme IS ?6 AND namespace.identity_value IS ?7
                     AND NOT EXISTS(
                       SELECT 1 FROM scan_runs
                       WHERE root_id = root.id AND status IN ('running', 'paused')
                     )
                 )",
                params![
                    root.root_id,
                    root.root_path,
                    root.active_scan_id,
                    sqlite_integer(root.root_generation.value(), "covered root generation")?,
                    root.is_active,
                    identity.map(|identity| &identity.scheme),
                    identity.map(|identity| &identity.value),
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !matches {
            return Err(changed());
        }
    }
    Ok(())
}

pub(super) fn retire(
    transaction: &Transaction<'_>,
    plan: &RootConsolidationPlan,
    interruption: &PublicationInterruption,
) -> Result<Vec<ConsolidatedLibraryRoot>, ScanError> {
    let mut consolidated = Vec::with_capacity(plan.candidates.len());
    for candidate in &plan.candidates {
        interruption.ensure_running()?;
        super::super::organization::lifecycle::transfer_source(
            transaction,
            &candidate.expected.root_id,
            &plan.destination_root_id,
        )?;
        if !root_unregistration::unregister_root(transaction, &candidate.expected.root_id)? {
            return Err(changed());
        }
        consolidated.push(ConsolidatedLibraryRoot {
            previous_root_id: candidate.expected.root_id.clone(),
            relative_path: candidate.relative_path.clone(),
        });
    }
    Ok(consolidated)
}

fn changed() -> ScanError {
    ScanError::new(
        "catalog_root_consolidation_changed",
        "The covered source registrations changed before the parent import committed",
    )
}

#[cfg(test)]
mod tests;
