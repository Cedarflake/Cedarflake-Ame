use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::file_operation_journal::FileOperationRecord;
use crate::domain::file_operations::FileAction;
use crate::domain::{AssetLocationView, ScanError};

use super::{
    COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS, EXACT_FINGERPRINT_SCHEME, ExactDuplicateReviewKey,
    database_error, load_decision, save_decision, sqlite_integer,
};

pub(in crate::adapters::sqlite_catalog) fn capture(
    transaction: &Transaction<'_>,
    record: &FileOperationRecord,
) -> Result<Option<ExactDuplicateReviewKey>, ScanError> {
    let binding = &record.authorization.catalog;
    let source = &record.authorization.request.source.state;
    let row: Option<(String, i64)> = transaction.query_row(
        "SELECT digest, byte_length FROM content_fingerprints WHERE source_generation = ?1 AND scheme = ?2 AND byte_length = ?3
         UNION ALL SELECT digest, byte_length FROM exact_duplicate_decisions
         WHERE scheme = ?2 AND preferred_asset_id = ?4 AND preferred_location_id = ?5 LIMIT 1",
        params![sqlite_integer(binding.source_generation, "original generation")?, EXACT_FINGERPRINT_SCHEME, sqlite_integer(source.file_size, "source size")?, binding.asset_id, binding.location_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(database_error)?;
    row.map(|(digest, byte_length)| {
        Ok(ExactDuplicateReviewKey {
            digest,
            byte_length: super::super::sqlite_unsigned(byte_length, "content length")?,
        })
    })
    .transpose()
}

pub(in crate::adapters::sqlite_catalog) fn publish(
    transaction: &Transaction<'_>,
    record: &FileOperationRecord,
    key: Option<ExactDuplicateReviewKey>,
    target: Option<&AssetLocationView>,
) -> Result<(), ScanError> {
    if record.authorization.request.action == FileAction::Copy {
        return Ok(());
    }
    let Some(key) = key else {
        return Ok(());
    };
    let binding = &record.authorization.catalog;
    let mut decision = load_decision(transaction, &key)?;
    if decision
        .preferred_location_id
        .as_ref()
        .is_some_and(|id| id != &binding.location_id)
    {
        return Ok(());
    }
    let preferred = match target {
        Some(target) => Some((target.location_id.clone(), target.asset_id.clone())),
        None => transaction
            .query_row(
                &format!(
                    "SELECT l.location_id, l.asset_id FROM {CURRENT_LOCATIONS}
              JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
              WHERE f.digest = ?2 AND f.byte_length = ?3 ORDER BY l.location_id LIMIT 1"
                ),
                params![
                    EXACT_FINGERPRINT_SCHEME,
                    key.digest,
                    sqlite_integer(key.byte_length, "content length")?
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(database_error)?,
    };
    decision.preferred_location_id = preferred.as_ref().map(|value| value.0.clone());
    decision.preferred_asset_id = preferred.map(|value| value.1);
    save_decision(transaction, &key, &decision)
}
