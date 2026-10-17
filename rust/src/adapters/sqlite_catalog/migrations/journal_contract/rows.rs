use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::{database_error, persistent_journal_range_history};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    let invalid_relations = connection
        .query_row(
            "SELECT
               EXISTS(
                 SELECT 1 FROM library_roots AS roots
                 JOIN library_change_root_state AS generations ON generations.root_id = roots.id
                 LEFT JOIN library_persistent_journal_root_state AS journal
                   ON journal.root_id = roots.id
                  AND journal.root_generation = generations.generation
                 WHERE generations.is_active = 1 AND journal.root_id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_root_state AS journal
                 JOIN library_change_root_state AS generations
                   ON generations.root_id = journal.root_id
                 WHERE journal.root_generation > generations.generation
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_root_state AS journal
                 LEFT JOIN library_persistent_journal_checkpoints AS checkpoints
                   ON checkpoints.root_id = journal.root_id
                  AND checkpoints.root_generation = journal.root_generation
                 WHERE journal.continuity_state = 'current' AND checkpoints.root_id IS NULL
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_queue_lineage AS lineage
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = lineage.source_range_id
                 WHERE NOT EXISTS(
                   SELECT 1 FROM library_change_queue_catch_up_lineage AS legacy_lineage
                   WHERE legacy_lineage.change_id = lineage.change_id
                     AND legacy_lineage.catch_up_source = 'persistent_journal_v1'
                     AND legacy_lineage.catch_up_watermark = ranges.id
                 )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_queue_lineage
                 WHERE enrolled_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_lineage
                 WHERE created_unix_ms < 0 OR updated_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges
                 WHERE enrolled_unix_ms < 0
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_ranges AS owners
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = owners.source_range_id
                 JOIN library_persistent_journal_cross_root_lineage AS lineage
                   ON lineage.id = owners.lineage_id
                 WHERE ranges.volume_guid <> lineage.volume_guid
                    OR ranges.volume_serial <> lineage.volume_serial
                    OR ranges.journal_id <> lineage.journal_id
                    OR (
                      owners.participant_role = 'previous'
                      AND (
                        ranges.root_id <> lineage.previous_root_id
                        OR ranges.root_generation <> lineage.previous_root_generation
                      )
                    )
                    OR (
                      owners.participant_role = 'current'
                      AND (
                        ranges.root_id <> lineage.current_root_id
                        OR ranges.root_generation <> lineage.current_root_generation
                      )
                    )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_cross_root_lineage AS lineage
                 WHERE 2 <> (
                   SELECT COUNT(*)
                   FROM library_persistent_journal_cross_root_ranges AS owners
                   WHERE owners.lineage_id = lineage.id
                 )
                    OR 1 <> (
                      SELECT COUNT(*)
                      FROM library_persistent_journal_cross_root_ranges AS owners
                      WHERE owners.lineage_id = lineage.id
                        AND owners.participant_role = 'previous'
                    )
                    OR 1 <> (
                      SELECT COUNT(*)
                      FROM library_persistent_journal_cross_root_ranges AS owners
                      WHERE owners.lineage_id = lineage.id
                        AND owners.participant_role = 'current'
                    )
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_source_ranges AS ranges
                 LEFT JOIN library_persistent_journal_range_lifecycle AS lifecycle
                   ON lifecycle.source_range_id = ranges.id
                 WHERE lifecycle.source_range_id IS NULL
                    OR (ranges.status = 'superseded'
                        AND lifecycle.lifecycle_state <> 'superseded')
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_range_lifecycle AS lifecycle
                 WHERE lifecycle.updated_unix_ms < 0
                    OR (lifecycle.lifecycle_state = 'completed'
                        AND EXISTS(
                          SELECT 1
                          FROM library_persistent_journal_queue_lineage AS ownership
                          JOIN library_change_queue AS changes
                            ON changes.id = ownership.change_id
                          WHERE ownership.source_range_id = lifecycle.source_range_id
                            AND changes.status NOT IN ('completed', 'superseded')
                        ))
               )
               OR EXISTS(
                 SELECT 1 FROM library_persistent_journal_pending_renames AS pending
                 JOIN library_persistent_journal_source_ranges AS ranges
                   ON ranges.id = pending.source_range_id
                 WHERE pending.volume_guid <> ranges.volume_guid
                    OR pending.volume_serial <> ranges.volume_serial
                    OR pending.journal_id <> ranges.journal_id
                    OR pending.previous_root_id <> ranges.root_id
                    OR pending.previous_root_generation <> ranges.root_generation
                    OR CAST(pending.old_usn AS INTEGER)
                       < CAST(ranges.requested_start_usn AS INTEGER)
                    OR CAST(pending.old_usn AS INTEGER)
                       >= CAST(ranges.covered_until_usn AS INTEGER)
               )
               OR EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if invalid_relations {
        return Err(super::values::unverifiable_contract());
    }
    let has_recovery_authority = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'library_recovery_authorities'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    let invalid_inventory = if has_recovery_authority {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_metadata_inventory_runs AS runs
                   WHERE (
                     runs.status IN ('running', 'comparing')
                     OR (runs.absence_authority <> 0 AND runs.status <> 'completed')
                   )
                   AND NOT EXISTS(
                     SELECT 1
                     FROM library_recovery_authorities AS authority
                     WHERE authority.run_id = runs.id
                       AND authority.root_id = runs.root_id
                       AND authority.root_generation = runs.root_generation
                       AND authority.retired_unix_ms IS NULL
                   )
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    } else {
        connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_metadata_inventory_runs
                   WHERE status IN ('running', 'comparing')
                      OR (absence_authority <> 0 AND status <> 'completed')
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
    };
    if invalid_inventory {
        return Err(super::values::unverifiable_contract());
    }
    super::state_rows::validate_roots(connection)?;
    super::state_rows::validate_checkpoints(connection)?;
    super::state_rows::validate_ranges(connection)?;
    persistent_journal_range_history::validate(connection)?;
    super::lineage::validate_lineage_rows(connection)?;
    super::lineage::validate_pending_renames(connection)?;
    super::payload_children::validate_payload_children(connection)
}
