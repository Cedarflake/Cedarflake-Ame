use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::domain::{
    LibraryChangeId, LibraryChangeLane, LibraryRecoveryOpeningBoundary, LibraryRootGeneration,
    PersistentJournalContinuityState, ScanError,
};

use super::super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};
use super::{
    attach_recovery_window_if_available, invalid_batch, load_checkpoint,
    load_root_authority_for_key, require_active_root_generation,
};

struct OpeningAttachment {
    change_id: LibraryChangeId,
    has_window: bool,
}

pub(super) fn attach(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    boundary: &LibraryRecoveryOpeningBoundary,
    captured_unix_ms: i64,
) -> Result<Option<LibraryChangeId>, ScanError> {
    boundary.validate()?;
    if captured_unix_ms < 0 {
        return Err(invalid_batch(
            "The recovery opening capture time is invalid",
        ));
    }
    // Ordinary journal reads also supply opening boundaries. A no-op must not interrupt
    // a lower-priority publication; release this snapshot before requesting a writer.
    let candidate = {
        let read = catalog
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(database_error)?;
        let candidate = select_attachment(&read, root_id, root_generation, boundary)?;
        read.commit().map_err(database_error)?;
        candidate
    };
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    if candidate.has_window {
        return Ok(Some(candidate.change_id));
    }

    let transaction = catalog.begin_write_in_lane(LibraryChangeLane::Journal)?;
    let candidate = select_attachment(&transaction, root_id, root_generation, boundary)?;
    let Some(candidate) = candidate else {
        transaction.commit().map_err(database_error)?;
        return Ok(None);
    };
    if !candidate.has_window {
        attach_recovery_window_if_available(
            &transaction,
            candidate.change_id,
            root_id,
            root_generation,
            Some(boundary),
            captured_unix_ms,
        )?;
    }
    transaction.commit().map_err(database_error)?;
    Ok(Some(candidate.change_id))
}

fn select_attachment(
    connection: &Connection,
    root_id: &str,
    root_generation: LibraryRootGeneration,
    boundary: &LibraryRecoveryOpeningBoundary,
) -> Result<Option<OpeningAttachment>, ScanError> {
    require_active_root_generation(connection, root_id, root_generation)?;
    let root = load_root_authority_for_key(connection, root_id, root_generation)?;
    if root.capability_state != "supported" || root.continuity_state != "recovery_required" {
        return Ok(None);
    }
    let checkpoint = load_checkpoint(connection, root_id, root_generation)?.ok_or_else(|| {
        ScanError::new(
            "persistent_journal_recovery_checkpoint_missing",
            "A pending journal recovery has no durable checkpoint",
        )
    })?;
    if checkpoint.continuity != PersistentJournalContinuityState::RecoveryRequired
        || checkpoint.volume != boundary.volume
        || checkpoint.root_file_reference != boundary.root_file_reference
        || checkpoint.protocol_version != boundary.protocol_version
        || checkpoint.contract_version != boundary.contract_version
    {
        return Err(ScanError::new(
            "persistent_journal_recovery_opening_identity_drift",
            "The recovered journal opening boundary no longer matches the root authority",
        ));
    }
    let active = connection
        .query_row(
            "SELECT authority.change_id,
                    window.change_id IS NOT NULL,
                    COALESCE(window.volume_guid = ?3
                      AND window.volume_serial = ?4
                      AND window.root_reference_version = ?5
                      AND window.root_file_reference = ?6
                      AND window.journal_id = ?7
                      AND window.opening_next_usn = ?8
                      AND window.protocol_version = ?9
                      AND window.contract_version = ?10
                      AND window.phase <> 'completed', 0)
             FROM library_recovery_authorities AS authority
             JOIN library_change_queue AS queue ON queue.id = authority.change_id
             JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
             LEFT JOIN library_persistent_journal_baselines AS window
               ON window.change_id = authority.change_id
             WHERE authority.root_id = ?1 AND authority.root_generation = ?2
               AND authority.retired_unix_ms IS NULL
               AND queue.status IN ('pending', 'leased', 'retry_wait')
               AND lane.lane = 'p2_recovery'
             ORDER BY authority.change_id LIMIT 1",
            params![
                root_id,
                sqlite_integer(root_generation.value(), "root generation")?,
                boundary.volume.volume_guid,
                boundary.volume.canonical_serial(),
                i64::from(boundary.root_file_reference.record_version()),
                boundary.root_file_reference.as_bytes(),
                boundary.journal_id.to_canonical_text(),
                boundary.next_usn.to_canonical_text(),
                i64::from(boundary.protocol_version),
                i64::from(boundary.contract_version),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some((change_id, has_window, exact_window)) = active else {
        return Ok(None);
    };
    let change_id = LibraryChangeId::new(sqlite_unsigned(change_id, "recovery change ID")?)
        .ok_or_else(|| invalid_batch("The recovery change ID is invalid"))?;
    if has_window && !exact_window {
        return Err(ScanError::new(
            "persistent_journal_recovery_opening_conflict",
            "The recovery authority already owns a different opening boundary",
        ));
    }
    Ok(Some(OpeningAttachment {
        change_id,
        has_window,
    }))
}
