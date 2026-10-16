use rusqlite::{OptionalExtension, Transaction, params};

use super::*;

pub(super) fn publish(
    transaction: &Transaction<'_>,
    run_id: &str,
    candidate: &FingerprintCandidate,
    fingerprint: &ContentFingerprint,
) -> Result<bool, ScanError> {
    let location = &candidate.location;
    let root = &candidate.root;
    let source_generation = sqlite_integer(location.source_generation, "source generation")?;
    let file_size = sqlite_integer(location.file_size, "file size")?;
    let source_revision = location
        .source_revision
        .as_ref()
        .map(super::super::source_revision_token)
        .transpose()?;
    if fingerprint.byte_length != location.file_size
        || fingerprint.digest.len() != 64
        || !fingerprint
            .digest
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
    {
        return Err(ScanError::new(
            "exact_fingerprint_invalid",
            "Complete content evidence must match the admitted source length and digest scheme",
        ));
    }
    let active: bool = transaction
        .query_row(
            "SELECT EXISTS (
           SELECT 1 FROM asset_locations AS l
           JOIN library_roots AS r ON r.id = l.root_id AND r.active_scan_id = l.scan_id
           JOIN library_change_root_state AS state ON state.root_id = r.id AND state.is_active = 1
           JOIN library_root_publication_namespaces AS namespace
             ON namespace.root_id = r.id AND namespace.root_generation = state.generation
           JOIN exact_analysis_runs AS run ON run.run_id = ?1 AND run.scheme = ?2
           WHERE l.location_id = ?3 AND l.asset_id = ?4 AND l.root_id = ?5 AND l.scan_id = ?6
             AND l.source_generation = ?7 AND l.file_size = ?8 AND l.modified_unix_ms = ?9
             AND l.file_identity_scheme IS ?10 AND l.file_identity_value IS ?11
             AND l.source_revision_token IS ?12 AND l.absolute_path = ?13 AND l.relative_path = ?14
             AND state.generation = ?15 AND r.path = ?16
             AND namespace.identity_scheme = ?17 AND namespace.identity_value = ?18
             AND NOT EXISTS (SELECT 1 FROM exact_analysis_outcomes WHERE run_id = ?1)
         )",
            params![
                run_id,
                EXACT_FINGERPRINT_SCHEME,
                location.location_id,
                location.asset_id,
                location.root_id,
                location.scan_id,
                source_generation,
                file_size,
                location.modified_unix_ms,
                location.file_identity.as_ref().map(|value| &value.scheme),
                location.file_identity.as_ref().map(|value| &value.value),
                source_revision,
                location.absolute_path,
                location.relative_path,
                sqlite_integer(root.root_generation.value(), "root generation")?,
                root.root_path,
                root.publication_root_identity
                    .as_ref()
                    .map(|value| &value.scheme),
                root.publication_root_identity
                    .as_ref()
                    .map(|value| &value.value)
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !active {
        return Ok(false);
    }
    let existing: Option<(String, u64)> = transaction
        .query_row(
            "SELECT digest, byte_length FROM content_fingerprints
             WHERE source_generation = ?1 AND scheme = ?2",
            params![source_generation, EXACT_FINGERPRINT_SCHEME],
            |row| Ok((row.get(0)?, read_unsigned(row, 1)?)),
        )
        .optional()
        .map_err(database_error)?;
    if let Some((digest, byte_length)) = existing {
        if digest != fingerprint.digest || byte_length != fingerprint.byte_length {
            return Err(ScanError::new(
                "exact_fingerprint_conflict",
                "One source generation produced conflicting complete content evidence",
            ));
        }
        return Ok(false);
    }
    transaction
        .execute(
            "INSERT INTO content_fingerprints(source_generation, scheme, digest, byte_length,
            file_identity_scheme, file_identity_value, source_revision_token, run_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                source_generation,
                EXACT_FINGERPRINT_SCHEME,
                fingerprint.digest,
                file_size,
                location.file_identity.as_ref().map(|value| &value.scheme),
                location.file_identity.as_ref().map(|value| &value.value),
                source_revision,
                run_id
            ],
        )
        .map_err(database_error)?;
    Ok(true)
}
