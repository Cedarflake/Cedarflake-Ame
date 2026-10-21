use rusqlite::{OptionalExtension, params};

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::{
    FileIdentityEvidence, LibraryChangeId, LibraryChangeLeaseUpdateOutcome,
    MetadataInventoryRunStatus, ScanError,
};

use super::super::spool_retirement::{SpoolOwner, retire_owned_spools};
use super::super::{SqliteCatalog, database_error, sqlite_integer};
use super::{
    directory_snapshot, load_absence_candidates, load_recovery_authority,
    metadata_inventory_has_unresolved_candidates, recovery_authority_database_error, require_run,
};

pub(super) fn finish(
    catalog: &mut SqliteCatalog,
    change_id: LibraryChangeId,
    lease_generation: u64,
    catalog_revision_at_success: u64,
    completed_unix_ms: i64,
) -> Result<Option<LibraryChangeLeaseUpdateOutcome>, ScanError> {
    let (root_path, expected_root_identity) = catalog
        .connection
        .query_row(
            "SELECT root.path, spool.root_identity_scheme, spool.root_identity_value
             FROM library_recovery_authorities AS authority
             JOIN library_metadata_inventory_spools AS spool
               ON spool.run_id = authority.run_id
             JOIN library_roots AS root ON root.id = authority.root_id
             WHERE authority.change_id = ?1 AND authority.retired_unix_ms IS NULL",
            [sqlite_integer(
                change_id.value(),
                "recovery authority change id",
            )?],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    FileIdentityEvidence {
                        scheme: row.get(1)?,
                        value: row.get(2)?,
                    },
                ))
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_root_identity_missing",
                "Recovery finalization lacks its durable pinned-root identity",
            )
        })?;
    let _finalization_root =
        PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &root_path,
            &expected_root_identity,
        )?;
    let transaction = catalog.begin_write()?;
    let authority = load_recovery_authority(&transaction, change_id)?.ok_or_else(|| {
        ScanError::new(
            "metadata_inventory_recovery_authority_missing",
            "The durable recovery authority no longer exists",
        )
    })?;
    if completed_unix_ms < authority.authorized_unix_ms {
        return Err(ScanError::new(
            "metadata_inventory_recovery_authority_time_invalid",
            "Recovery authority cannot complete before it was authorized",
        ));
    }
    let run = require_run(&transaction, &authority.run_id)?;
    if !matches!(
        run.status,
        MetadataInventoryRunStatus::Comparing | MetadataInventoryRunStatus::Completed
    ) || run.request.root_id != authority.root_id
        || run.request.root_generation != authority.root_generation
    {
        return Err(ScanError::new(
            "metadata_inventory_recovery_incomplete",
            "Recovery authority cannot retire before its inventory completes",
        ));
    }
    if run.status == MetadataInventoryRunStatus::Comparing {
        if !run.enumeration_complete || !run.absence_authority {
            return Err(ScanError::new(
                "metadata_inventory_recovery_incomplete",
                "Recovery authority cannot retire before inventory publication completes",
            ));
        }
        let has_pending = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_entries
                   WHERE run_id = ?1 AND comparison_status = 'pending'
                 )",
                [&authority.run_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        let remaining_absence =
            load_absence_candidates(&transaction, &run, run.absence_cursor.as_deref(), 1)?;
        if has_pending || !remaining_absence.is_empty() {
            return Err(ScanError::new(
                "metadata_inventory_recovery_incomplete",
                "Recovery authority cannot retire before every inventory page is published",
            ));
        }
    }
    let is_baseline = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_persistent_journal_baselines
               WHERE change_id = ?1
             )",
            [sqlite_integer(
                change_id.value(),
                "recovery authority change id",
            )?],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if is_baseline {
        let phase = transaction
            .query_row(
                "SELECT phase FROM library_persistent_journal_baselines
                 WHERE change_id = ?1",
                [sqlite_integer(
                    change_id.value(),
                    "recovery authority change id",
                )?],
                |row| row.get::<_, String>(0),
            )
            .map_err(database_error)?;
        if phase != "absence"
            || !super::super::persistent_journal::baseline_closing_is_covered(
                &transaction,
                change_id,
            )?
        {
            return Err(ScanError::new(
                "persistent_journal_baseline_closing_incomplete",
                "The closing journal boundary must be durably covered before baseline completion",
            ));
        }
    }
    if metadata_inventory_has_unresolved_candidates(&transaction, &run)? {
        transaction.commit().map_err(database_error)?;
        return Ok(None);
    }
    let outcome = super::super::change_queue::classify_lease_update(
        &transaction,
        change_id,
        lease_generation,
        Some(catalog_revision_at_success),
    )?;
    if outcome != LibraryChangeLeaseUpdateOutcome::Applied {
        transaction.commit().map_err(database_error)?;
        return Ok(Some(outcome));
    }
    directory_snapshot::publish(&transaction, &run)?;
    let completed = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', lease_expires_unix_ms = NULL,
                 catalog_revision_at_success = ?1, updated_unix_ms = ?2
             WHERE id = ?3 AND status = 'leased' AND lease_generation = ?4",
            params![
                sqlite_integer(catalog_revision_at_success, "catalog revision")?,
                completed_unix_ms,
                sqlite_integer(change_id.value(), "change ID")?,
                sqlite_integer(lease_generation, "lease generation")?,
            ],
        )
        .map_err(database_error)?;
    if completed != 1 {
        return Err(ScanError::new(
            "metadata_inventory_recovery_completion_raced",
            "The leased recovery changed during atomic completion",
        ));
    }
    if run.status == MetadataInventoryRunStatus::Comparing {
        let completed_run = transaction
            .execute(
                "UPDATE library_metadata_inventory_runs
                 SET status = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE id = ?1 AND status = 'comparing'
                   AND enumeration_complete = 1 AND absence_authority = 1",
                params![authority.run_id, completed_unix_ms],
            )
            .map_err(database_error)?;
        if completed_run != 1 {
            return Err(ScanError::new(
                "metadata_inventory_recovery_completion_raced",
                "The inventory publication changed during atomic completion",
            ));
        }
    }
    super::super::establish_root_publication_namespace(
        &transaction,
        &authority.root_id,
        sqlite_integer(authority.root_generation.value(), "root generation")?,
        &expected_root_identity,
        "metadata_inventory",
        catalog_revision_at_success,
        completed_unix_ms,
    )?;
    if is_baseline {
        let baseline_identity = transaction
            .query_row(
                "SELECT root_id, root_generation
                 FROM library_persistent_journal_baselines
                 WHERE change_id = ?1 AND phase = 'absence'",
                [sqlite_integer(change_id.value(), "baseline change ID")?],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE library_persistent_journal_baselines
                 SET phase = 'completed', completed_unix_ms = ?2, updated_unix_ms = ?2
                 WHERE change_id = ?1 AND phase = 'absence'
                   AND completed_unix_ms IS NULL",
                params![
                    sqlite_integer(change_id.value(), "baseline change ID")?,
                    completed_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_completion_raced",
                "The one-time baseline changed during atomic completion",
            ));
        }
        let checkpoint_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_checkpoints
                 SET continuity_state = 'current', updated_unix_ms = ?1
                 WHERE root_id = ?2 AND root_generation = ?3
                   AND continuity_state = 'catching_up'
                   AND next_unread_usn = captured_exclusive_end
                   AND last_failure_code IS NULL",
                params![completed_unix_ms, baseline_identity.0, baseline_identity.1],
            )
            .map_err(database_error)?;
        if checkpoint_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_completion_raced",
                "The baseline checkpoint changed during atomic completion",
            ));
        }
        let root_state_updated = transaction
            .execute(
                "UPDATE library_persistent_journal_root_state
                 SET continuity_state = 'current', updated_unix_ms = ?1
                 WHERE root_id = ?2 AND root_generation = ?3
                   AND capability_state = 'supported'
                   AND continuity_state = 'catching_up'
                   AND last_failure_code IS NULL
                   AND EXISTS (
                     SELECT 1 FROM library_change_root_state AS active
                     WHERE active.root_id = ?2 AND active.generation = ?3
                       AND active.is_active = 1
                   )",
                params![completed_unix_ms, baseline_identity.0, baseline_identity.1],
            )
            .map_err(database_error)?;
        if root_state_updated != 1 {
            return Err(ScanError::new(
                "persistent_journal_baseline_completion_raced",
                "The baseline root authority changed during atomic completion",
            ));
        }
    }
    retire_owned_spools(&transaction, SpoolOwner::Run(&authority.run_id))?;
    let retired = transaction
        .execute(
            "UPDATE library_recovery_authorities
             SET retired_unix_ms = ?2
             WHERE change_id = ?1 AND retired_unix_ms IS NULL",
            params![
                sqlite_integer(change_id.value(), "recovery authority change id")?,
                completed_unix_ms,
            ],
        )
        .map_err(recovery_authority_database_error)?;
    if retired != 1 {
        return Err(ScanError::new(
            "metadata_inventory_recovery_completion_raced",
            "Recovery authority changed during atomic completion",
        ));
    }
    super::super::change_queue::wake_metadata_inventory_capacity_deferrals(
        &transaction,
        &authority.root_id,
        authority.root_generation,
        completed_unix_ms,
    )?;
    super::super::change_queue::completion::release_handoffs(&transaction, change_id)?;
    transaction.commit().map_err(database_error)?;
    Ok(Some(outcome))
}
