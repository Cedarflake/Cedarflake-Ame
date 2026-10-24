use rusqlite::types::Value;
use rusqlite::{Connection, Transaction, params_from_iter};

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::validate_identifier;
use crate::domain::{GalleryCollection, GalleryQuery, ScanError};

use super::super::database_error;
use super::super::exact_duplicate_review::representative_expression;
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};
use super::push_source_filters;

pub(in super::super) fn validate_collection(
    connection: &Connection,
    query: &GalleryQuery,
) -> Result<(), ScanError> {
    let GalleryCollection::Album { album_id } = &query.collection else {
        return Ok(());
    };
    validate_identifier(album_id)?;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_albums AS album WHERE album.album_id = ?1
         AND NOT EXISTS (SELECT 1 FROM organization_album_retirements AS retired
           WHERE retired.album_id = album.album_id))",
            [album_id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !exists {
        return Err(ScanError::new(
            "organization_album_unavailable",
            "The selected album is no longer available",
        ));
    }
    Ok(())
}

// Retained display evidence preserves browsing during verification, but cannot
// admit commands. The organization command owner still requires current evidence.
pub(in super::super) fn record_expression(location: &str) -> String {
    let compatible = compatible_fingerprint(location);
    let bound = compatible_binding(location);
    format!(
        "COALESCE(
          (SELECT mapping.group_id FROM organization_bindings AS binding
           JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
           WHERE {bound}),
          (SELECT mapping.group_id FROM {DISPLAY_FINGERPRINTS}
           CROSS JOIN organization_bindings AS binding
             ON binding.fingerprint_scheme = f.scheme AND binding.digest = f.digest
               AND binding.byte_length = f.byte_length
           CROSS JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
           WHERE {compatible} AND binding.digest IS NOT NULL LIMIT 1))"
    )
}

pub(super) fn group_expression(location: &str) -> String {
    let record = record_expression(location);
    let compatible = compatible_fingerprint(location);
    format!(
        "COALESCE(
           'record:' || {record},
           (SELECT printf('content:%s:%s:%lld', f.scheme, f.digest, f.byte_length)
            FROM {DISPLAY_FINGERPRINTS} WHERE {compatible}),
           printf('version:%s:%lld', {location}.asset_id, {location}.source_generation))"
    )
}

fn compatible_binding(location: &str) -> String {
    format!(
        "binding.asset_id = {location}.asset_id
         AND binding.source_generation = {location}.source_generation
         AND binding.byte_length = {location}.file_size
         AND binding.file_identity_scheme IS {location}.file_identity_scheme
         AND binding.file_identity_value IS {location}.file_identity_value
         AND binding.source_revision_token IS {location}.source_revision_token"
    )
}

fn representative_joins() -> String {
    let bound = compatible_binding("locations");
    let compatible = compatible_fingerprint("locations");
    // Joining the union view materializes all historical fingerprints. Resolve the
    // compatible evidence version first so the optional join remains a point lookup.
    format!(
        "LEFT JOIN organization_bindings AS binding ON {bound}
         LEFT JOIN content_fingerprints AS f
           ON f.source_generation = (
             SELECT f.evidence_source_generation FROM {DISPLAY_FINGERPRINTS} WHERE {compatible})
           AND f.scheme = '{EXACT_FINGERPRINT_SCHEME}'
         LEFT JOIN exact_duplicate_decisions AS decision
           ON decision.scheme = COALESCE(f.scheme, binding.fingerprint_scheme)
           AND decision.digest = COALESCE(f.digest, binding.digest)
           AND decision.byte_length = COALESCE(f.byte_length, binding.byte_length)"
    )
}

pub(super) fn push_collection_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let condition = match &query.collection {
        GalleryCollection::Library => return,
        GalleryCollection::Favorites => "record.favorite = 1",
        GalleryCollection::Album { album_id } => {
            parameters.push(Value::Text(album_id.clone()));
            "record.album_id = ? AND record.disposition = 'organized'"
        }
    };
    let group = record_expression("locations");
    clauses.push(format!(
        "EXISTS(SELECT 1 FROM organization_records AS record
         WHERE record.record_id = {group} AND {condition})"
    ));
}

pub(super) fn push_group_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let mut scope = Vec::new();
    push_source_filters(query, &mut scope, parameters);
    if scope.is_empty() {
        scope.push("1".to_owned());
    }
    let joins = representative_joins();
    let representative = representative_expression("locations");
    let group = group_expression("locations");
    clauses.push(format!(
        "locations.location_id IN (
           SELECT {representative}
           FROM library_roots AS roots
           JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
           {joins}
           WHERE {} GROUP BY {group})",
        scope.join(" AND ")
    ));
}

pub(super) fn representative_for_anchor(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
    location_id: &str,
    asset_id: Option<&str>,
) -> Result<Option<String>, ScanError> {
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
    let current_group = group_expression("locations");
    let requested_group = group_expression("requested");
    scope.push(format!(
        "{current_group} = (
          SELECT {requested_group} FROM asset_locations AS requested
          JOIN library_roots AS requested_root ON requested.scan_id = requested_root.active_scan_id
          WHERE requested.location_id = ? {asset_guard})"
    ));
    let joins = representative_joins();
    let representative = representative_expression("locations");
    transaction
        .query_row(
            &format!(
                "SELECT {representative}
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations ON locations.scan_id = roots.active_scan_id
                 {joins}
                 WHERE {}",
                scope.join(" AND ")
            ),
            params_from_iter(parameters),
            |row| row.get(0),
        )
        .map_err(database_error)
}
