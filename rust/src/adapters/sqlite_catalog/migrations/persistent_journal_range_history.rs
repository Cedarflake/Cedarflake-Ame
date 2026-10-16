use rusqlite::Connection;

use crate::domain::ScanError;

use super::{database_error, unverifiable_persistent_journal_contract};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let has_baselines = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_persistent_journal_baselines'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    // A closed reset authorizes a new USN namespace. Retained older ranges remain
    // history; their offsets cannot be compared with the replacement checkpoint.
    // The baseline contract separately proves the complete successor chain.
    let historical_reset_proof = if has_baselines {
        "EXISTS(
           SELECT 1 FROM library_persistent_journal_baselines AS reset
           JOIN library_recovery_authorities AS authority
             ON authority.change_id = reset.change_id
           JOIN library_change_queue AS control ON control.id = reset.change_id
           WHERE reset.root_id = ranges.root_id
             AND reset.root_generation = ranges.root_generation
             AND reset.volume_guid = ranges.volume_guid
             AND reset.volume_serial = ranges.volume_serial
             AND reset.root_reference_version = checkpoints.root_reference_version
             AND reset.root_file_reference = checkpoints.root_file_reference
             AND reset.protocol_version = ranges.protocol_version
             AND reset.protocol_version = checkpoints.protocol_version
             AND reset.contract_version = ranges.contract_version
             AND reset.contract_version = checkpoints.contract_version
             AND reset.journal_id <> ranges.journal_id
             AND reset.phase IN ('replay', 'absence', 'completed')
             AND reset.closing_next_usn IS NOT NULL
             AND authority.reason = 'journal_reset'
             AND authority.root_id = ranges.root_id
             AND authority.root_generation = ranges.root_generation
             AND control.root_id = ranges.root_id
             AND control.root_generation = ranges.root_generation
             AND control.scope = 'root'
             AND ranges.checkpointed_unix_ms <= authority.authorized_unix_ms
             AND authority.authorized_unix_ms <= reset.authorized_unix_ms
         )"
    } else {
        // Older schemas have no reset authority and retain the strict original rule.
        "0"
    };
    let invalid = connection
        .query_row(
            &format!(
                "SELECT EXISTS(
                   SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                   JOIN library_persistent_journal_checkpoints AS checkpoints
                     ON checkpoints.root_id = ranges.root_id
                    AND checkpoints.root_generation = ranges.root_generation
                   WHERE ranges.status = 'checkpointed'
                     AND (
                       checkpoints.volume_guid <> ranges.volume_guid
                       OR checkpoints.volume_serial <> ranges.volume_serial
                       OR CASE WHEN checkpoints.journal_id = ranges.journal_id
                         THEN CAST(checkpoints.next_unread_usn AS INTEGER)
                              < CAST(ranges.covered_until_usn AS INTEGER)
                         ELSE NOT ({historical_reset_proof})
                       END
                     )
                 )"
            ),
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid {
        return Err(unverifiable_persistent_journal_contract());
    }
    Ok(())
}
