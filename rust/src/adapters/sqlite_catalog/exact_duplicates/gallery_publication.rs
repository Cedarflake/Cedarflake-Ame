use rusqlite::{Transaction, params_from_iter, types::Value};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, EXACT_PUBLICATION_BATCH_SIZE};

use super::super::{database_error, sqlite_integer};

pub(super) fn publish(
    transaction: &Transaction<'_>,
    run_id: &str,
    source_generations: &[u64],
) -> Result<(), ScanError> {
    if source_generations.is_empty() {
        return Ok(());
    }
    if source_generations.len() > EXACT_PUBLICATION_BATCH_SIZE {
        return Err(ScanError::new(
            "exact_publication_batch_exceeded",
            "Gallery evidence publication exceeded its source bound",
        ));
    }
    let active: bool = transaction
        .query_row(
            "SELECT EXISTS (
               SELECT 1 FROM exact_analysis_runs AS run
               WHERE run.run_id = ?1 AND NOT EXISTS (
                 SELECT 1 FROM exact_analysis_outcomes WHERE run_id = ?1))",
            [run_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !active {
        return Err(ScanError::new(
            "exact_publication_run_retired",
            "A gallery evidence batch requires an active analysis run",
        ));
    }
    let mut parameters = vec![
        Value::Text(run_id.to_owned()),
        Value::Text(EXACT_FINGERPRINT_SCHEME.to_owned()),
    ];
    for generation in source_generations {
        parameters.push(Value::Integer(sqlite_integer(
            *generation,
            "source generation",
        )?));
    }
    let placeholders = vec!["?"; source_generations.len()].join(", ");
    let changed = transaction
        .execute(
            &format!(
                "INSERT OR IGNORE INTO gallery_content_fingerprints(source_generation, scheme)
                 SELECT f.source_generation, f.scheme FROM content_fingerprints AS f
                 WHERE f.run_id = ? AND f.scheme = ?
                   AND f.source_generation IN ({placeholders})"
            ),
            params_from_iter(&parameters),
        )
        .map_err(database_error)?;
    super::super::organization::publication::batch(transaction, run_id, source_generations)?;
    transaction
        .execute(
            &format!(
                "DELETE FROM gallery_retained_fingerprints WHERE source_generation IN (
                   SELECT f.source_generation FROM content_fingerprints AS f
                   WHERE f.run_id = ? AND f.scheme = ?
                     AND f.source_generation IN ({placeholders}))"
            ),
            params_from_iter(&parameters),
        )
        .map_err(database_error)?;
    if changed > 0 {
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
    }
    Ok(())
}
