use rusqlite::Connection;
use rusqlite::types::Value;

use crate::domain::organization::validate_identifier;
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::{GalleryCollection, GalleryQuery, ScanError};

use super::super::database_error;
use super::duplicates::{DISPLAY_FINGERPRINTS, compatible_fingerprint};

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

pub(super) fn compatible_binding(location: &str) -> String {
    format!(
        "binding.asset_id = {location}.asset_id
         AND binding.source_generation = {location}.source_generation
         AND binding.byte_length = {location}.file_size
         AND binding.file_identity_scheme IS {location}.file_identity_scheme
         AND binding.file_identity_value IS {location}.file_identity_value
         AND binding.source_revision_token IS {location}.source_revision_token"
    )
}

pub(super) fn push_collection_filter(
    query: &GalleryQuery,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    let Some(condition) = collection_condition(&query.collection, parameters) else {
        return;
    };
    let group = record_expression("locations");
    clauses.push(format!(
        "EXISTS(SELECT 1 FROM organization_records AS record
         WHERE record.record_id = {group} AND {condition})"
    ));
}

pub(super) fn presence_condition(
    query: &GalleryQuery,
    parameters: &mut Vec<Value>,
) -> Option<&'static str> {
    if let Some(condition) = collection_condition(&query.collection, parameters) {
        return Some(condition);
    }
    let queue = query.organization_queue?;
    if queue == OrganizationQueue::Pending {
        // Unrecorded groups are pending, so absent decisions cannot prove an empty queue.
        return None;
    }
    parameters.push(Value::Text(queue.disposition().stored().to_owned()));
    Some("record.disposition = ?")
}

fn collection_condition(
    collection: &GalleryCollection,
    parameters: &mut Vec<Value>,
) -> Option<&'static str> {
    match collection {
        GalleryCollection::Library => None,
        GalleryCollection::Favorites => Some("record.favorite = 1"),
        GalleryCollection::Album { album_id } => {
            parameters.push(Value::Text(album_id.clone()));
            Some("record.album_id = ? AND record.disposition = 'organized'")
        }
    }
}
