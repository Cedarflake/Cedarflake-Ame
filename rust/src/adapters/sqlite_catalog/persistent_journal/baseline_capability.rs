use rusqlite::{Transaction, params};

use crate::domain::{PersistentJournalBaselineStartRequest, ScanError};

use super::super::{database_error, sqlite_integer};
use super::{load_checkpoint, load_root_authority_for_key};

pub(super) fn acquire(
    transaction: &Transaction<'_>,
    request: &PersistentJournalBaselineStartRequest,
) -> Result<(), ScanError> {
    if load_checkpoint(transaction, &request.root_id, request.root_generation)?.is_some() {
        return Err(ScanError::new(
            "persistent_journal_baseline_checkpoint_exists",
            "A root with a trustworthy checkpoint cannot start a migration baseline",
        ));
    }
    let updated = transaction
        .execute(
            "UPDATE library_persistent_journal_root_state
         SET protocol_version = ?1, contract_version = ?2,
             capability_state = 'supported', continuity_state = 'baseline_required',
             last_failure_code = NULL, last_failure_message = NULL, updated_unix_ms = ?3
         WHERE root_id = ?4 AND root_generation = ?5
           AND EXISTS(
             SELECT 1 FROM library_change_root_state AS active
             WHERE active.root_id = ?4 AND active.generation = ?5 AND active.is_active = 1
           )
           AND (
             (capability_state = 'unknown' AND continuity_state = 'baseline_required'
               AND protocol_version = 0 AND last_failure_code IS NULL)
             OR (capability_state = 'supported' AND continuity_state = 'baseline_required'
               AND protocol_version = ?1 AND contract_version = ?2 AND last_failure_code IS NULL)
             OR (capability_state = 'live_only' AND continuity_state = 'live_only')
           )",
            params![
                i64::from(request.protocol_version),
                i64::from(request.contract_version),
                request.authorized_unix_ms,
                request.root_id,
                sqlite_integer(request.root_generation.value(), "root generation")?
            ],
        )
        .map_err(database_error)?;
    if updated != 1 {
        return Err(invalid(
            "The root cannot atomically acquire supported baseline-required journal authority",
        ));
    }
    let state =
        load_root_authority_for_key(transaction, &request.root_id, request.root_generation)?;
    if state.capability_state != "supported"
        || state.continuity_state != "baseline_required"
        || state.protocol_version != request.protocol_version
        || state.contract_version != request.contract_version
    {
        return Err(invalid(
            "The root does not own supported baseline-required journal authority",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> ScanError {
    ScanError::new("persistent_journal_baseline_capability_invalid", message)
}
