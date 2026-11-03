use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};

use crate::domain::ScanError;
use crate::domain::classification_selection::{
    ClassificationAnnotation, ClassificationReviewReason,
};
use crate::domain::organization::{OrganizationSelection, stale_selection, validate_selections};

use super::super::{SqliteCatalog, database_error, load_catalog_revision, sqlite_integer};
use super::classification::{PredictionReadStrategy, effective_projection};
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};

impl SqliteCatalog {
    pub(crate) fn classification_annotations(
        &self,
        revision: u64,
        selections: &[OrganizationSelection],
    ) -> Result<Vec<ClassificationAnnotation>, ScanError> {
        validate_selections(selections)?;
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        if load_catalog_revision(&transaction)? != revision {
            return Err(ScanError::new(
                "catalog_revision_changed",
                "Classification annotations require the displayed catalog revision",
            ));
        }
        let selected = selections.iter().collect::<BTreeSet<_>>();
        let placeholders = vec!["(?, ?, ?)"; selected.len()].join(", ");
        let mut values = Vec::with_capacity(selected.len() * 3);
        for selection in &selected {
            values.extend([
                Value::Text(selection.asset_id.clone()),
                Value::Text(selection.location_id.clone()),
                Value::Integer(sqlite_integer(
                    selection.source_generation,
                    "source generation",
                )?),
            ]);
        }
        let query = format!(
            "WITH selected(asset_id, location_id, source_generation) AS (VALUES {placeholders})
             SELECT locations.location_id, (
               {}
               SELECT CASE effective.review
                 WHEN 'confirmed' THEN NULL
                 WHEN 'conflict' THEN 'conflict'
                 WHEN 'deferred' THEN 'deferred'
                 WHEN 'suggested' THEN 'routine_sampling'
                 WHEN 'needs_review' THEN json_extract(evidence.payload, '$.review')
                 WHEN 'unclassified' THEN CASE WHEN {} THEN 'analysis_failed' END
               END
               FROM effective LEFT JOIN classification_predictions AS evidence
                 ON evidence.publication_id = effective.publication_id
             ) AS reason
             FROM selected
             JOIN asset_locations AS locations ON locations.location_id = selected.location_id
               AND locations.asset_id = selected.asset_id
               AND locations.source_generation = selected.source_generation
             JOIN library_roots AS roots ON roots.active_scan_id = locations.scan_id
             ORDER BY locations.location_id",
            effective_projection("locations", PredictionReadStrategy::Indexed),
            failed_work("locations")
        );
        let rows = {
            let mut statement = transaction.prepare(&query).map_err(database_error)?;
            statement
                .query_map(params_from_iter(values), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?
        };
        if rows.len() != selected.len() {
            return Err(stale_selection());
        }
        let mut result = Vec::new();
        for (location_id, stored_reason) in rows {
            if let Some(stored_reason) = stored_reason {
                result.push(ClassificationAnnotation {
                    location_id,
                    reason: reason(&stored_reason)?,
                });
            }
        }
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

pub(in super::super) fn has_failed_work(
    connection: &Connection,
    selection: &OrganizationSelection,
) -> Result<bool, ScanError> {
    let query = format!(
        "SELECT {} FROM asset_locations AS locations
         JOIN library_roots AS roots ON roots.active_scan_id = locations.scan_id
         WHERE locations.location_id = ?1 AND locations.asset_id = ?2
           AND locations.source_generation = ?3",
        failed_work("locations")
    );
    connection
        .query_row(
            &query,
            params![
                selection.location_id,
                selection.asset_id,
                sqlite_integer(selection.source_generation, "source generation")?,
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(stale_selection)
}

fn failed_work(location: &str) -> String {
    format!(
        "EXISTS (
           SELECT 1 FROM classification_state AS state
           JOIN classification_work_items AS work
             ON work.configuration_revision = state.configuration_revision
               AND work.kind = 'source' AND work.source_generation = {location}.source_generation
           WHERE state.singleton = 1 AND work.phase = 'failed'
         ) OR EXISTS (
           SELECT 1 FROM {DISPLAY_FINGERPRINTS}
           JOIN classification_state AS state ON state.singleton = 1
           JOIN classification_work_items AS work
             ON work.configuration_revision = state.configuration_revision
               AND work.kind = 'content' AND work.fingerprint_scheme = f.scheme
               AND work.digest = f.digest AND work.byte_length = f.byte_length
           WHERE {} AND work.phase = 'failed'
         )",
        compatible_fingerprint(location)
    )
}

fn reason(value: &str) -> Result<ClassificationReviewReason, ScanError> {
    match value {
        "uncalibrated" => Ok(ClassificationReviewReason::Uncalibrated),
        "low_separation" => Ok(ClassificationReviewReason::Uncertain),
        "routine_sampling" => Ok(ClassificationReviewReason::Sampling),
        "deferred" => Ok(ClassificationReviewReason::Deferred),
        "conflict" => Ok(ClassificationReviewReason::Conflict),
        "analysis_failed" => Ok(ClassificationReviewReason::AnalysisFailed),
        _ => Err(ScanError::new(
            "classification_annotation_invalid",
            "The classification review reason is invalid",
        )),
    }
}
