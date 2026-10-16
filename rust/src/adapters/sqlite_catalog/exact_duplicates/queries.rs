use rusqlite::{Connection, params};

use super::*;

pub(in crate::adapters::sqlite_catalog) const CANDIDATE_COLUMNS: &str =
    "l.asset_id, l.location_id, l.root_id, l.scan_id,
    l.absolute_path, l.relative_path, l.preview_path, l.file_size, l.created_unix_ms,
    l.modified_unix_ms, l.width, l.height, l.preview_status, l.preview_issue_code,
    l.preview_issue_message, l.metadata_engine_id, l.metadata_engine_version,
    l.capture_local_time, l.capture_offset_minutes, l.capture_time_source, l.capture_raw_value,
    l.file_identity_scheme, l.file_identity_value, l.source_revision_token, l.source_generation,
    r.path, state.generation, namespace.identity_scheme, namespace.identity_value";

pub(in crate::adapters::sqlite_catalog) const CURRENT_LOCATIONS: &str = "asset_locations AS l
    JOIN library_roots AS r ON r.id = l.root_id AND r.active_scan_id = l.scan_id
    JOIN library_change_root_state AS state ON state.root_id = r.id AND state.is_active = 1
    LEFT JOIN library_root_publication_namespaces AS namespace
      ON namespace.root_id = r.id AND namespace.root_generation = state.generation";

pub(in crate::adapters::sqlite_catalog) const COMPATIBLE_FINGERPRINT: &str =
    "f.source_generation = l.source_generation
    AND f.scheme = ?1 AND f.byte_length = l.file_size
    AND f.file_identity_scheme IS l.file_identity_scheme
    AND f.file_identity_value IS l.file_identity_value
    AND f.source_revision_token IS l.source_revision_token";

pub(super) fn load_candidates(
    connection: &Connection,
    after: &str,
) -> Result<Vec<FingerprintCandidate>, ScanError> {
    let sql = format!(
        "SELECT {CANDIDATE_COLUMNS} FROM {CURRENT_LOCATIONS}
         WHERE l.location_id > ?2 AND l.file_size > 0
           AND NOT EXISTS (SELECT 1 FROM content_fingerprints AS f WHERE {COMPATIBLE_FINGERPRINT})
           AND EXISTS (
             SELECT 1 FROM asset_locations AS peer
             JOIN library_roots AS peer_root ON peer_root.id = peer.root_id AND peer_root.active_scan_id = peer.scan_id
             WHERE peer.file_size = l.file_size AND peer.location_id <> l.location_id)
         ORDER BY l.location_id LIMIT ?3"
    );
    read_candidates(
        connection,
        &sql,
        params![
            EXACT_FINGERPRINT_SCHEME,
            after,
            EXACT_DUPLICATE_PAGE_SIZE as i64
        ],
    )
}

pub(in crate::adapters::sqlite_catalog) fn load_locations(
    connection: &Connection,
    digest: &str,
    byte_length: u64,
    after: &str,
) -> Result<Vec<FingerprintCandidate>, ScanError> {
    let sql = format!(
        "SELECT {CANDIDATE_COLUMNS} FROM {CURRENT_LOCATIONS}
         JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
         WHERE f.digest = ?2 AND f.byte_length = ?3 AND l.location_id > ?4
         ORDER BY l.location_id LIMIT ?5"
    );
    read_candidates(
        connection,
        &sql,
        params![
            EXACT_FINGERPRINT_SCHEME,
            digest,
            sqlite_integer(byte_length, "content length")?,
            after,
            (EXACT_DUPLICATE_PAGE_SIZE + 1) as i64
        ],
    )
}

fn read_candidates(
    connection: &Connection,
    sql: &str,
    values: impl rusqlite::Params,
) -> Result<Vec<FingerprintCandidate>, ScanError> {
    let mut statement = connection.prepare(sql).map_err(database_error)?;
    statement
        .query_map(values, read_candidate)
        .map_err(database_error)?
        .map(|row| row.map_err(database_error)?)
        .collect()
}

pub(super) fn load_groups(
    connection: &Connection,
    after: Option<&ExactDuplicateGroupCursor>,
) -> Result<ExactDuplicateGroupPage, ScanError> {
    let sql = format!(
        "WITH grouped AS (
           SELECT f.digest, f.byte_length, COUNT(*) AS locations,
                  COUNT(DISTINCT l.source_generation) AS physical_files,
                  MIN(l.location_id) AS representative
           FROM {CURRENT_LOCATIONS}
           JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
           WHERE (f.digest, f.byte_length) > (?2, ?3)
           GROUP BY f.digest, f.byte_length HAVING COUNT(*) > 1
           ORDER BY f.digest, f.byte_length LIMIT ?4
         )
         SELECT g.digest, g.byte_length, g.locations, g.physical_files,
                g.representative, l.absolute_path
         FROM grouped AS g JOIN asset_locations AS l ON l.location_id = g.representative
         JOIN library_roots AS r ON r.id = l.root_id AND r.active_scan_id = l.scan_id
         ORDER BY g.digest, g.byte_length"
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut groups = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                after.map_or("", |value| value.digest.as_str()),
                sqlite_integer(after.map_or(0, |value| value.byte_length), "content length")?,
                (EXACT_DUPLICATE_PAGE_SIZE + 1) as i64
            ],
            |row| {
                Ok(ExactDuplicateGroup {
                    digest: row.get(0)?,
                    byte_length: read_unsigned(row, 1)?,
                    location_count: read_unsigned(row, 2)?,
                    physical_file_count: read_unsigned(row, 3)?,
                    representative_location_id: row.get(4)?,
                    representative_path: super::super::user_visible_path(&row.get::<_, String>(5)?),
                })
            },
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let next = if groups.len() > EXACT_DUPLICATE_PAGE_SIZE {
        groups.truncate(EXACT_DUPLICATE_PAGE_SIZE);
        groups.last().map(|group| ExactDuplicateGroupCursor {
            digest: group.digest.clone(),
            byte_length: group.byte_length,
        })
    } else {
        None
    };
    Ok(ExactDuplicateGroupPage { groups, next })
}
