use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};

use crate::domain::{CatalogDeltaBatch, FileIdentityEvidence, MediaAdmissionRecheck, ScanError};

use super::{database_error, sqlite_integer};

pub(super) fn load(
    connection: &Connection,
    root_id: &str,
    relative_path: &str,
) -> Result<Option<MediaAdmissionRecheck>, ScanError> {
    connection
        .query_row(
            "SELECT recheck.asset_id, recheck.location_id,
                recheck.file_identity_scheme, recheck.file_identity_value, recheck.state
         FROM library_media_admission_rechecks AS recheck
         JOIN library_change_root_state AS root ON root.root_id = recheck.root_id
           AND root.generation = recheck.root_generation AND root.is_active = 1
         WHERE recheck.root_id = ?1 AND recheck.relative_path = ?2",
            params![root_id, relative_path],
            |row| {
                let scheme: Option<String> = row.get(2)?;
                let value: Option<String> = row.get(3)?;
                Ok(MediaAdmissionRecheck {
                    asset_id: row.get(0)?,
                    location_id: row.get(1)?,
                    file_identity: scheme
                        .zip(value)
                        .map(|(scheme, value)| FileIdentityEvidence { scheme, value }),
                    requires_inspection: row.get::<_, String>(4)? != "rejected",
                })
            },
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn finish_publication(
    transaction: &Transaction<'_>,
    batch: &CatalogDeltaBatch,
) -> Result<(), ScanError> {
    if batch.completions.is_empty() {
        return Ok(());
    }
    let changes = batch
        .completions
        .iter()
        .map(|completion| sqlite_integer(completion.change_id.value(), "media recheck change"))
        .collect::<Result<Vec<_>, _>>()?;
    let placeholders = std::iter::repeat_n("?", changes.len())
        .collect::<Vec<_>>()
        .join(",");
    let completed_rechecks = {
        let mut statement = transaction
            .prepare(&format!(
                "SELECT work.id, work.relative_path, work.previous_relative_path
             FROM library_change_queue AS work
             WHERE work.scope = 'path' AND work.id IN ({placeholders})
               AND (EXISTS (SELECT 1 FROM library_media_admission_rechecks AS recheck
                      WHERE recheck.root_id = work.root_id AND recheck.root_generation = work.root_generation
                        AND recheck.relative_path = work.relative_path)
                 OR EXISTS (SELECT 1 FROM library_media_admission_rechecks AS recheck
                      WHERE recheck.root_id = work.root_id AND recheck.root_generation = work.root_generation
                        AND recheck.relative_path = work.previous_relative_path))"
            ))
            .map_err(database_error)?;
        statement
            .query_map(params_from_iter(changes), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    let generation = sqlite_integer(batch.root_generation.value(), "root generation")?;
    for (change_id, relative_path, previous_path) in completed_rechecks {
        // A paired rename can arrive ahead of the queued recheck. Carry its rejected identity
        // only when publication removed both paths and verified the same physical file.
        if let Some(previous_path) = previous_path {
            transaction.execute(
            "UPDATE library_media_admission_rechecks AS recheck
             SET relative_path = ?3, state = 'rejected', change_id = NULL
             WHERE recheck.root_id = ?1 AND recheck.root_generation = ?2
               AND recheck.relative_path = ?4
               AND EXISTS (SELECT 1 FROM library_terminal_media_evidence AS evidence
                 WHERE evidence.root_id = recheck.root_id AND evidence.relative_path = ?3
                   AND evidence.file_identity_scheme = recheck.file_identity_scheme
                   AND evidence.file_identity_value = recheck.file_identity_value
                   AND NOT EXISTS (SELECT 1 FROM library_media_admission_rechecks AS target
                     WHERE target.root_id = recheck.root_id AND target.relative_path = ?3)
                   AND NOT EXISTS (SELECT 1 FROM asset_locations AS location
                     JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
                     WHERE location.root_id = recheck.root_id
                       AND location.relative_path IN (?3, ?4)))",
            params![batch.root_id, generation, relative_path, previous_path],
            ).map_err(database_error)?;
        }
        // Completion and negative evidence are already source-revalidated by the reconciler.
        // Keep only the rejected identity reservation, so repairing the bytes can restore its ID.
        transaction.execute(
            "UPDATE library_media_admission_rechecks AS recheck SET state = 'rejected', change_id = NULL
             WHERE recheck.root_id = ?1 AND recheck.root_generation = ?2
               AND recheck.relative_path = ?3
               AND NOT EXISTS (SELECT 1 FROM asset_locations AS location
                 JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
                 WHERE location.root_id = recheck.root_id AND location.relative_path = recheck.relative_path)
               AND EXISTS (SELECT 1 FROM library_terminal_media_evidence AS evidence
                 WHERE evidence.root_id = recheck.root_id AND evidence.relative_path = recheck.relative_path
                   AND evidence.file_identity_scheme IS recheck.file_identity_scheme
                   AND evidence.file_identity_value IS recheck.file_identity_value)",
            params![batch.root_id, generation, relative_path],
        ).map_err(database_error)?;
        transaction.execute(
            "DELETE FROM library_media_admission_rechecks AS recheck
             WHERE recheck.root_id = ?1 AND recheck.root_generation = ?2
               AND recheck.relative_path = ?3
               AND recheck.state <> 'rejected'
               AND NOT EXISTS (SELECT 1 FROM asset_locations AS location
                 JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
                 WHERE location.root_id = recheck.root_id AND location.relative_path = recheck.relative_path
                   AND location.asset_id = recheck.asset_id AND location.width = 0 AND location.height = 0
                   AND location.preview_issue_code IN ('image_format_unsupported', 'media_type_unsupported'))",
            params![batch.root_id, generation, relative_path],
        ).map_err(database_error)?;
        transaction.execute(
            "DELETE FROM library_media_admission_rechecks AS recheck
             WHERE recheck.root_id = ?1 AND recheck.root_generation = ?2
               AND recheck.relative_path = ?3 AND recheck.state = 'rejected'
               AND EXISTS (SELECT 1 FROM asset_locations AS location
                 JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
                 WHERE location.root_id = recheck.root_id AND location.relative_path = recheck.relative_path)",
            params![batch.root_id, generation, relative_path],
        ).map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_media_admission_rechecks SET state = 'pending', change_id = NULL
             WHERE root_id = ?1 AND relative_path = ?2 AND change_id = ?3 AND state = 'queued'",
                params![batch.root_id, relative_path, change_id],
            )
            .map_err(database_error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
