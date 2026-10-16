use rusqlite::Transaction;
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::{GalleryDuplicateMode, GalleryQuery, ScanError};

use super::super::exact_duplicate_review::{DECISION_JOIN, representative_expression};
use super::{database_error, push_source_filters};

pub(in super::super) fn publish_completed_evidence(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    // Fingerprints are complete immutable records, including those retained by a
    // cancelled process. Admit them together so a gallery cursor never changes
    // membership while retaining its revision.
    let changed = transaction
        .execute(
            "INSERT INTO gallery_content_fingerprints(source_generation, scheme)
             SELECT source_generation, scheme FROM content_fingerprints AS f
             WHERE NOT EXISTS (
               SELECT 1 FROM gallery_content_fingerprints AS published
               WHERE published.source_generation = f.source_generation
                 AND published.scheme = f.scheme)",
            [],
        )
        .map_err(database_error)?;
    if changed > 0 {
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
    }
    Ok(())
}

pub(super) fn compatible_fingerprint(location: &str) -> String {
    format!(
        "f.source_generation = {location}.source_generation
         AND f.scheme = '{EXACT_FINGERPRINT_SCHEME}'
         AND f.byte_length = {location}.file_size
         AND f.file_identity_scheme IS {location}.file_identity_scheme
         AND f.file_identity_value IS {location}.file_identity_value
         AND f.source_revision_token IS {location}.source_revision_token"
    )
}

pub(super) const PUBLISHED_FINGERPRINTS: &str = "(content_fingerprints AS f
    JOIN gallery_content_fingerprints AS published
      ON published.source_generation = f.source_generation AND published.scheme = f.scheme)";

pub(super) fn push_duplicate_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if query.duplicate_mode == GalleryDuplicateMode::All {
        return;
    }
    let mut scope = Vec::new();
    push_source_filters(query, &mut scope, parameters);
    let scope = if scope.is_empty() {
        "1".to_owned()
    } else {
        scope.join(" AND ")
    };
    let compatible = compatible_fingerprint("locations");
    let representative = representative_expression("locations");
    clauses.push(format!(
        "(NOT EXISTS (
           SELECT 1 FROM {PUBLISHED_FINGERPRINTS} WHERE {compatible})
         OR locations.location_id IN (
           SELECT {representative}
           FROM library_roots AS roots
           JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
           JOIN {PUBLISHED_FINGERPRINTS} ON {compatible}
           {DECISION_JOIN}
           WHERE {scope}
           GROUP BY f.scheme, f.digest, f.byte_length))"
    ));
}

pub(super) fn representative_for_anchor(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    location_id: &str,
    asset_id: Option<&str>,
) -> Result<Option<String>, ScanError> {
    if query.duplicate_mode == GalleryDuplicateMode::All {
        return Ok(None);
    }
    let mut scope = Vec::new();
    let mut parameters = Vec::new();
    push_source_filters(query, &mut scope, &mut parameters);
    parameters.push(Value::Text(location_id.to_owned()));
    let asset_guard = if let Some(asset_id) = asset_id {
        parameters.push(Value::Text(asset_id.to_owned()));
        "AND requested.asset_id = ?"
    } else {
        ""
    };
    let current = compatible_fingerprint("locations");
    let representative = representative_expression("locations");
    let requested = compatible_fingerprint("requested");
    scope.push(format!(
        "(f.digest, f.byte_length) = (
          SELECT f.digest, f.byte_length
          FROM asset_locations AS requested
          JOIN library_roots AS requested_root ON requested.scan_id = requested_root.active_scan_id
          JOIN {PUBLISHED_FINGERPRINTS} ON {requested}
          WHERE requested.location_id = ? {asset_guard})"
    ));
    transaction
        .query_row(
            &format!(
                "SELECT {representative}
             FROM library_roots AS roots
             JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
             JOIN {PUBLISHED_FINGERPRINTS} ON {current}
             {DECISION_JOIN}
             WHERE {}",
                scope.join(" AND ")
            ),
            params_from_iter(parameters),
            |row| row.get(0),
        )
        .map_err(database_error)
}
