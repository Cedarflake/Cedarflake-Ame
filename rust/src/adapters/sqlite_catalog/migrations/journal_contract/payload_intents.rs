use rusqlite::{Connection, params};

use crate::domain::{LibraryChangeIntent, ScanError, persistent_journal_canonical_intent_entry};

use super::super::database_error;

pub(super) fn validate(
    connection: &Connection,
    payloads: &std::collections::BTreeMap<
        String,
        crate::domain::PersistentJournalBatchPayloadChildren,
    >,
) -> Result<(), ScanError> {
    let orphaned_evidence = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1
               FROM library_change_queue_catch_up_lineage AS lineage
               LEFT JOIN library_persistent_journal_source_ranges AS ranges
                 ON ranges.id = lineage.catch_up_watermark
               WHERE lineage.catch_up_source = 'persistent_journal_v1'
                 AND (
                   ranges.id IS NULL
                   OR NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS direct
                     WHERE direct.source_range_id = lineage.catch_up_watermark
                       AND direct.change_id = lineage.change_id
                   ) AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS owner
                     JOIN library_persistent_journal_cross_root_ranges AS owner_lineage
                       ON owner_lineage.source_range_id = owner.source_range_id
                     JOIN library_persistent_journal_cross_root_ranges AS peer_lineage
                       ON peer_lineage.lineage_id = owner_lineage.lineage_id
                      AND peer_lineage.source_range_id = lineage.catch_up_watermark
                      AND peer_lineage.enrolled_unix_ms = lineage.enrolled_unix_ms
                     WHERE owner.change_id = lineage.change_id
                   ) AND NOT EXISTS (
                     SELECT 1
                     FROM library_persistent_journal_queue_lineage AS recovery_owner
                     JOIN library_persistent_journal_source_ranges AS recovery_range
                       ON recovery_range.id = recovery_owner.source_range_id
                     JOIN library_change_queue AS recovery_change
                       ON recovery_change.id = recovery_owner.change_id
                     JOIN library_persistent_journal_cross_root_lineage AS recovery_lineage
                       ON recovery_lineage.previous_carry_id IS NOT NULL
                      AND recovery_lineage.previous_root_id = recovery_change.root_id
                      AND recovery_lineage.previous_root_generation = recovery_change.root_generation
                      AND recovery_lineage.previous_relative_path = recovery_change.relative_path
                      AND recovery_lineage.new_usn = recovery_change.first_sequence
                      AND recovery_lineage.new_usn = recovery_change.most_recent_sequence
                      AND recovery_lineage.volume_guid = recovery_range.volume_guid
                      AND recovery_lineage.volume_serial = recovery_range.volume_serial
                      AND recovery_lineage.journal_id = recovery_range.journal_id
                      AND CAST(recovery_range.requested_start_usn AS INTEGER)
                            <= CAST(recovery_lineage.new_usn AS INTEGER)
                      AND CAST(recovery_lineage.new_usn AS INTEGER)
                            < CAST(recovery_range.covered_until_usn AS INTEGER)
                     JOIN library_persistent_journal_cross_root_ranges AS recovery_endpoint
                       ON recovery_endpoint.lineage_id = recovery_lineage.id
                      AND recovery_endpoint.source_range_id = lineage.catch_up_watermark
                      AND recovery_endpoint.enrolled_unix_ms = lineage.enrolled_unix_ms
                     WHERE recovery_owner.change_id = lineage.change_id
                   )
                 )
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if orphaned_evidence {
        return Err(super::values::unverifiable_contract());
    }

    for (range_id, children) in payloads {
        let mut expected =
            crate::adapters::sqlite_catalog::change_queue::normalize_persistent_journal_intents(
                &children.intents,
            )
            .map_err(|_| super::values::unverifiable_contract())?
            .iter()
            .map(persistent_journal_canonical_intent_entry)
            .collect::<Vec<_>>();
        expected.sort();

        let range_enrolled_unix_ms = connection
            .query_row(
                "SELECT enrolled_unix_ms
                 FROM library_persistent_journal_source_ranges WHERE id = ?1",
                [range_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT changes.root_id, changes.root_generation, changes.intent_kind,
                        changes.scope, changes.relative_path, changes.previous_relative_path,
                        changes.origin, changes.first_observed_unix_ms,
                        changes.most_recent_observed_unix_ms, changes.first_sequence,
                        changes.most_recent_sequence, changes.coalesced_observation_count,
                        changes.status, changes.catch_up_source, changes.catch_up_watermark,
                        ownership.enrolled_unix_ms,
                        (SELECT COUNT(*)
                         FROM library_change_queue_catch_up_lineage AS lineage
                         WHERE lineage.change_id = changes.id
                           AND (
                             lineage.catch_up_source <> 'persistent_journal_v1'
                             OR NOT (
                               lineage.catch_up_watermark = ?1
                               OR EXISTS (
                                 SELECT 1
                                 FROM library_persistent_journal_cross_root_ranges AS owner_lineage
                                 JOIN library_persistent_journal_cross_root_ranges AS peer_lineage
                                   ON peer_lineage.lineage_id = owner_lineage.lineage_id
                                  AND peer_lineage.source_range_id = lineage.catch_up_watermark
                                  AND peer_lineage.enrolled_unix_ms = lineage.enrolled_unix_ms
                                 WHERE owner_lineage.source_range_id = ?1
                               ) OR EXISTS (
                                 SELECT 1
                                 FROM library_persistent_journal_source_ranges AS recovery_range
                                 JOIN library_persistent_journal_cross_root_lineage AS recovery_lineage
                                   ON recovery_lineage.previous_carry_id IS NOT NULL
                                  AND recovery_lineage.previous_root_id = changes.root_id
                                  AND recovery_lineage.previous_root_generation = changes.root_generation
                                  AND recovery_lineage.previous_relative_path = changes.relative_path
                                  AND recovery_lineage.new_usn = changes.first_sequence
                                  AND recovery_lineage.new_usn = changes.most_recent_sequence
                                  AND recovery_lineage.volume_guid = recovery_range.volume_guid
                                  AND recovery_lineage.volume_serial = recovery_range.volume_serial
                                  AND recovery_lineage.journal_id = recovery_range.journal_id
                                  AND CAST(recovery_range.requested_start_usn AS INTEGER)
                                        <= CAST(recovery_lineage.new_usn AS INTEGER)
                                  AND CAST(recovery_lineage.new_usn AS INTEGER)
                                        < CAST(recovery_range.covered_until_usn AS INTEGER)
                                 JOIN library_persistent_journal_cross_root_ranges AS recovery_endpoint
                                   ON recovery_endpoint.lineage_id = recovery_lineage.id
                                  AND recovery_endpoint.source_range_id = lineage.catch_up_watermark
                                  AND recovery_endpoint.enrolled_unix_ms = lineage.enrolled_unix_ms
                                 WHERE recovery_range.id = ?1
                               )
                             )
                           )),
                        (SELECT COUNT(*)
                         FROM library_change_queue_catch_up_lineage AS lineage
                         WHERE lineage.change_id = changes.id
                           AND lineage.catch_up_source = 'persistent_journal_v1'
                           AND lineage.catch_up_watermark = ?1
                           AND lineage.enrolled_unix_ms = ?2)
                 FROM library_persistent_journal_queue_lineage AS ownership
                 JOIN library_change_queue AS changes ON changes.id = ownership.change_id
                 WHERE ownership.source_range_id = ?1
                 ORDER BY changes.id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![range_id, range_enrolled_unix_ms], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Option<String>>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, i64>(17)?,
                ))
            })
            .map_err(database_error)?;
        let mut actual = Vec::new();
        for row in rows {
            let row = row.map_err(database_error)?;
            if !matches!(
                row.12.as_str(),
                "pending" | "leased" | "retry_wait" | "completed" | "superseded"
            ) || row.13.as_deref() != Some("persistent_journal_v1")
                || row.14.as_deref() != Some(range_id.as_str())
                || row.15 != range_enrolled_unix_ms
                || row.16 != 0
                || row.17 != 1
            {
                return Err(super::values::unverifiable_contract());
            }
            let intent = LibraryChangeIntent {
                root_id: row.0,
                root_generation: super::values::parse_root_generation(row.1)?,
                kind: super::values::parse_intent_kind(&row.2)?,
                scope: super::values::parse_intent_scope(&row.3)?,
                relative_path: row.4,
                previous_relative_path: row.5,
                origin: super::values::parse_intent_origin(&row.6)?,
                first_observed_unix_ms: row.7,
                most_recent_observed_unix_ms: row.8,
                first_sequence: super::values::parse_canonical_u64(&row.9)?,
                most_recent_sequence: super::values::parse_canonical_u64(&row.10)?,
                coalesced_observation_count: u32::try_from(row.11)
                    .map_err(|_| super::values::unverifiable_contract())?,
            };
            actual.push(persistent_journal_canonical_intent_entry(&intent));
        }
        actual.sort();
        if actual != expected {
            return Err(super::values::unverifiable_contract());
        }
    }
    Ok(())
}
