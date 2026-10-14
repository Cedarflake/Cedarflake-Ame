use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::domain::{
    FileIdentityEvidence, LibraryRootGeneration, PersistentJournalBaseline,
    PersistentJournalBaselinePhase, ScanCheckpoint, ScanError, ScanRequest, ScanResumeCoverage,
};

use super::scan_lifecycle::discard_first_import_boundary;
use super::scan_resume_inventory::retain_locations;
use super::{
    ScanOwner, SqliteCatalog, database_error, optional_sqlite_u32, sqlite_u32, sqlite_unsigned,
    validate_root_publication_identity,
};

#[cfg(test)]
mod tests;

enum ScanResumePlan {
    RetainCheckpoint,
    RebuildUnverifiedFirstImport,
}

fn resume_checkpoint(
    transaction: &Transaction<'_>,
    scan_id: &str,
    owner: ScanOwner,
    checkpoint: ScanCheckpoint,
    coverage: Option<&ScanResumeCoverage>,
) -> Result<ScanCheckpoint, ScanError> {
    let has_published_baseline = transaction
        .query_row(
            "SELECT root.active_scan_id IS NOT NULL FROM scan_runs AS scan
         JOIN library_roots AS root ON root.id = scan.root_id WHERE scan.id = ?1",
            [scan_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let plan = if let Some(coverage) = coverage {
        let expected = coverage.baseline();
        let current = load_first_import_baseline(transaction, scan_id, &expected.root_id)?;
        if owner != ScanOwner::Foreground
            || has_published_baseline
            || current.as_ref() != Some(expected)
        {
            return Err(ScanError::new(
                "catalog_scan_resume_coverage_stale",
                "The first-import boundary changed after its journal coverage was checked",
            ));
        }
        ScanResumePlan::RetainCheckpoint
    } else if owner == ScanOwner::Foreground && !has_published_baseline {
        ScanResumePlan::RebuildUnverifiedFirstImport
    } else {
        ScanResumePlan::RetainCheckpoint
    };
    match plan {
        ScanResumePlan::RetainCheckpoint => Ok(checkpoint),
        ScanResumePlan::RebuildUnverifiedFirstImport => {
            // A retained opening record cannot prove that its journal still covers the
            // detached interval. Only an explicit resume reaches this rebuilding path.
            discard_first_import_boundary(transaction, scan_id)?;
            retain_locations(transaction, scan_id)?;
            for sql in [
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                "DELETE FROM scan_directory_identities WHERE scan_id = ?1",
                "DELETE FROM scan_issues WHERE scan_id = ?1",
            ] {
                transaction
                    .execute(sql, [scan_id])
                    .map_err(database_error)?;
            }
            transaction
                .execute(
                    "UPDATE scan_runs SET last_visited_relative_path = NULL,
                   visited_entries = 0, accepted_items = 0, issue_count = 0,
                   requires_previous_snapshot = 0, current_directory_relative_path = NULL,
                   current_directory_enumerated = 0
                 WHERE id = ?1 AND status IN ('running', 'paused')",
                    [scan_id],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                    [scan_id],
                )
                .map_err(database_error)?;
            transaction.execute(
                "UPDATE library_persistent_journal_root_state
                 SET protocol_version = 0, capability_state = 'unknown', continuity_state = 'baseline_required',
                   last_failure_code = NULL, last_failure_message = NULL
                 WHERE (root_id, root_generation) = (
                   SELECT root_id, root_generation_at_start FROM scan_runs WHERE id = ?1
                 )",
                params![scan_id],
            ).map_err(database_error)?;
            Ok(ScanCheckpoint::default())
        }
    }
}

impl SqliteCatalog {
    pub(crate) fn load_first_import_resume_baseline(
        &self,
        scan_id: &str,
        root_id: &str,
    ) -> Result<Option<PersistentJournalBaseline>, ScanError> {
        load_first_import_baseline(&self.connection, scan_id, root_id)
    }

    pub(super) fn resume_scan_owned(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        owner: ScanOwner,
        publication_identity: Option<&FileIdentityEvidence>,
        coverage: Option<&ScanResumeCoverage>,
    ) -> Result<ScanCheckpoint, ScanError> {
        if let Some(identity) = publication_identity {
            validate_root_publication_identity(identity)?;
        }
        let transaction = self.begin_write()?;
        let stored = transaction
            .query_row(
                "SELECT scans.root_id, roots.path, scans.status,
                        scans.max_items, scans.max_entries, scans.preview_edge,
                        scans.last_visited_relative_path, scans.visited_entries,
                        scans.accepted_items, scans.issue_count,
                        scans.root_generation_at_start,
                        scans.requires_previous_snapshot, scans.scan_owner,
                        state.generation, state.is_active
                 FROM scan_runs AS scans
                 JOIN library_roots AS roots ON roots.id = scans.root_id
                 JOIN library_change_root_state AS state ON state.root_id = scans.root_id
                 WHERE scans.id = ?1",
                [&request.scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, Option<i64>>(10)?,
                        row.get::<_, bool>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, i64>(13)?,
                        row.get::<_, bool>(14)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_resume_missing",
                    "The requested scan checkpoint no longer exists",
                )
            })?;
        let (
            stored_root_id,
            stored_root_path,
            status,
            max_items,
            max_entries,
            preview_edge,
            last_visited_relative_path,
            visited_entries,
            accepted_items,
            issue_count,
            stored_root_generation,
            requires_previous_snapshot,
            stored_owner,
            active_root_generation,
            root_is_active,
        ) = stored;
        let stored_max_items = optional_sqlite_u32(max_items, "item limit")?;
        let stored_max_entries = optional_sqlite_u32(max_entries, "entry limit")?;
        let stored_preview_edge = sqlite_u32(preview_edge, "preview edge")?;
        let is_paused = status == "paused";
        if status != "running" && !is_paused
            || stored_root_id != root_id
            || stored_root_path != root_path
            || stored_max_items != request.max_items
            || stored_max_entries != request.max_entries
            || stored_preview_edge != request.preview_edge
            || stored_root_generation != Some(active_root_generation)
            || !root_is_active
            || stored_owner != owner.as_str()
        {
            return Err(ScanError::new(
                "catalog_scan_resume_mismatch",
                "The stored scan cannot be resumed with different identity, ownership, or parameters",
            ));
        }
        if let Some(identity) = publication_identity {
            let binding = transaction
                .query_row(
                    "SELECT root_id, root_generation, identity_scheme, identity_value
                     FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
                    [&request.scan_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?;
            if binding
                .as_ref()
                .is_none_or(|(binding_root, generation, scheme, value)| {
                    binding_root != root_id
                        || Some(*generation) != stored_root_generation
                        || scheme != &identity.scheme
                        || value != &identity.value
                })
            {
                return Err(ScanError::new(
                    "catalog_scan_publication_namespace_mismatch",
                    "The stored scan lacks its original configured-root namespace binding",
                ));
            }
        }
        let checkpoint = ScanCheckpoint {
            last_visited_relative_path,
            visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
            accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
            requires_previous_snapshot,
        };
        let has_conflicting_scan = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused') AND id <> ?2
                 )",
                params![root_id, request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if has_conflicting_scan {
            return Err(ScanError::new(
                "catalog_root_scan_in_progress",
                "Another authoritative scan already owns this library root",
            ));
        }
        if is_paused {
            let updated = transaction
                .execute(
                    "UPDATE scan_runs SET status = 'running'
                     WHERE id = ?1 AND status = 'paused'",
                    [&request.scan_id],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_scan_resume_raced",
                    "The scan checkpoint changed before it could be resumed",
                ));
            }
        }
        let checkpoint =
            resume_checkpoint(&transaction, &request.scan_id, owner, checkpoint, coverage)?;
        transaction.commit().map_err(database_error)?;
        Ok(checkpoint)
    }
}

fn load_first_import_baseline(
    connection: &Connection,
    scan_id: &str,
    root_id: &str,
) -> Result<Option<PersistentJournalBaseline>, ScanError> {
    let generation = connection
        .query_row(
            "SELECT active.generation
             FROM scan_runs AS scan
             JOIN library_roots AS root ON root.id = scan.root_id
             JOIN library_change_root_state AS active ON active.root_id = root.id
             JOIN library_recovery_authorities AS authority
               ON authority.run_id = scan.id AND authority.root_id = root.id
              AND authority.root_generation = active.generation
             JOIN library_persistent_journal_root_state AS journal
               ON journal.root_id = root.id AND journal.root_generation = active.generation
             WHERE scan.id = ?1 AND scan.root_id = ?2
               AND scan.status IN ('running', 'paused') AND scan.scan_owner = 'foreground'
               AND scan.root_generation_at_start = active.generation AND active.is_active = 1
               AND root.active_scan_id IS NULL
               AND authority.reason = 'first_import_boundary' AND authority.retired_unix_ms IS NULL
               AND journal.capability_state = 'supported'
               AND journal.continuity_state = 'baseline_required'
               AND journal.last_failure_code IS NULL",
            params![scan_id, root_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(generation) = generation else {
        return Ok(None);
    };
    let generation = LibraryRootGeneration::new(sqlite_unsigned(generation, "root generation")?)
        .ok_or_else(|| {
            ScanError::new("catalog_scan_generation_invalid", "Invalid scan generation")
        })?;
    let baseline =
        super::persistent_journal::load_baseline_for_root(connection, root_id, generation)?;
    Ok(baseline.filter(|baseline| {
        baseline.run_id == scan_id && baseline.phase == PersistentJournalBaselinePhase::Inventory
    }))
}
