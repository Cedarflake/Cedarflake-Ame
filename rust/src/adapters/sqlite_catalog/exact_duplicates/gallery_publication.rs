use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, Transaction, params, params_from_iter, types::Value};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, EXACT_PUBLICATION_BATCH_SIZE};
use crate::domain::organization::OrganizationSelection;

use super::super::{database_error, sqlite_integer};
use super::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};

pub(in crate::adapters::sqlite_catalog) fn publish_verified_source(
    transaction: &Transaction<'_>,
    source: &OrganizationSelection,
) -> Result<bool, ScanError> {
    let generation = transaction
        .query_row(
            &format!(
                "SELECT f.source_generation FROM {CURRENT_LOCATIONS}
            JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
            WHERE l.asset_id = ?2 AND l.location_id = ?3 AND l.source_generation = ?4"
            ),
            params![
                EXACT_FINGERPRINT_SCHEME,
                source.asset_id,
                source.location_id,
                sqlite_integer(source.source_generation, "source generation")?
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(generation) = generation else {
        return Ok(false);
    };
    publish_sources(transaction, &[generation])?;
    Ok(true)
}

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
    let mut generations = source_generations
        .iter()
        .map(|generation| sqlite_integer(*generation, "source generation"))
        .collect::<Result<Vec<_>, _>>()?;
    parameters.extend(generations.iter().copied().map(Value::Integer));
    let placeholders = vec!["?"; source_generations.len()].join(", ");
    let accepted = transaction
        .prepare(&format!(
            "SELECT source_generation FROM content_fingerprints
         WHERE run_id = ? AND scheme = ? AND source_generation IN ({placeholders})"
        ))
        .map_err(database_error)?
        .query_map(params_from_iter(&parameters), |row| row.get::<_, i64>(0))
        .map_err(database_error)?
        .collect::<rusqlite::Result<BTreeSet<_>>>()
        .map_err(database_error)?;
    generations.retain(|generation| accepted.contains(generation));
    publish_sources(transaction, &generations)
}

fn publish_sources(transaction: &Transaction<'_>, generations: &[i64]) -> Result<(), ScanError> {
    if generations.is_empty() {
        return Ok(());
    }
    let mut parameters = vec![Value::Text(EXACT_FINGERPRINT_SCHEME.to_owned())];
    parameters.extend(generations.iter().copied().map(Value::Integer));
    let placeholders = vec!["?"; generations.len()].join(", ");
    let changed = transaction
        .execute(
            &format!(
                "INSERT OR IGNORE INTO gallery_content_fingerprints(source_generation, scheme)
         SELECT source_generation, scheme FROM content_fingerprints
         WHERE scheme = ? AND source_generation IN ({placeholders})"
            ),
            params_from_iter(&parameters),
        )
        .map_err(database_error)?;
    super::super::organization::publication::published_generations(transaction, generations)?;
    transaction
        .execute(
            &format!(
                "DELETE FROM gallery_retained_fingerprints WHERE source_generation IN (
                   SELECT f.source_generation FROM content_fingerprints AS f
                   WHERE f.scheme = ?
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
