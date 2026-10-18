use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::{AssetLocationView, ScanError};

use super::super::{database_error, source_revision_token, sqlite_integer};

pub(in super::super) struct RetainedMembership {
    source_generation: i64,
    evidence_source_generation: i64,
    scheme: String,
}

impl RetainedMembership {
    pub(in super::super) fn bind(
        self,
        transaction: &Transaction<'_>,
        generation: i64,
    ) -> Result<(), ScanError> {
        if generation != self.evidence_source_generation {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO gallery_retained_fingerprints
                   (source_generation, evidence_source_generation, scheme) VALUES (?1, ?2, ?3)",
                    params![generation, self.evidence_source_generation, self.scheme],
                )
                .map_err(database_error)?;
        }
        transaction
            .execute(
                "DELETE FROM gallery_retained_fingerprints
             WHERE source_generation = ?1 AND source_generation <> ?2
               AND NOT EXISTS (SELECT 1 FROM asset_locations WHERE source_generation = ?1)",
                params![self.source_generation, generation],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

// Capture before a delta removes its previous location. The snapshot is consumed
// in that same transaction after the guarded replacement chooses its generation.
pub(in super::super) fn capture_unchanged_display(
    transaction: &Transaction<'_>,
    location: &AssetLocationView,
) -> Result<Option<RetainedMembership>, ScanError> {
    let Some(identity) = &location.file_identity else {
        return Ok(None);
    };
    let size = sqlite_integer(location.file_size, "file size")?;
    let revision = location
        .source_revision
        .as_ref()
        .map(source_revision_token)
        .transpose()?;
    transaction.query_row(
        "SELECT prior.source_generation, f.evidence_source_generation, f.scheme
         FROM asset_locations AS prior
         JOIN library_roots AS root ON root.id = prior.root_id AND root.active_scan_id = prior.scan_id
         JOIN gallery_display_fingerprints AS f ON f.source_generation = prior.source_generation
         WHERE prior.file_identity_scheme = ?1 AND prior.file_identity_value = ?2
           AND prior.file_size = ?3 AND prior.modified_unix_ms = ?4
           AND prior.source_revision_token IS ?5
           AND f.scheme = ?6 AND f.byte_length = ?3
           AND f.file_identity_scheme = ?1 AND f.file_identity_value = ?2
           AND f.source_revision_token IS ?5
         LIMIT 1",
        params![
            identity.scheme,
            identity.value,
            size,
            location.modified_unix_ms,
            revision,
            EXACT_FINGERPRINT_SCHEME,
        ],
        |row| Ok(RetainedMembership {
            source_generation: row.get(0)?,
            evidence_source_generation: row.get(1)?,
            scheme: row.get(2)?,
        }),
    ).optional().map_err(database_error)
}

pub(in super::super) fn rebind_unchanged_display(
    transaction: &Transaction<'_>,
    location: &AssetLocationView,
    generation: i64,
) -> Result<(), ScanError> {
    // Dirty evidence still invalidates content. Only the prior display relationship
    // survives an otherwise unchanged physical observation, bound to the new generation.
    if let Some(retained) = capture_unchanged_display(transaction, location)? {
        retained.bind(transaction, generation)?;
    }
    let Some(identity) = &location.file_identity else {
        return Ok(());
    };
    transaction
        .execute(
            "DELETE FROM gallery_retained_fingerprints
             WHERE source_generation <> ?1 AND source_generation IN (
               SELECT source_generation FROM asset_locations
               WHERE file_identity_scheme = ?2 AND file_identity_value = ?3)",
            params![generation, identity.scheme, identity.value],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(in super::super) fn discard_retired_batch(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "WITH candidates AS MATERIALIZED (
               SELECT source_generation, scheme FROM gallery_retained_fingerprints
               ORDER BY source_generation, scheme LIMIT 256
             )
             DELETE FROM gallery_retained_fingerprints WHERE source_generation IN (
               SELECT retained.source_generation FROM candidates AS retained
               WHERE EXISTS (
                 SELECT 1 FROM gallery_content_fingerprints AS published
                 WHERE published.source_generation = retained.source_generation AND published.scheme = retained.scheme)
               OR NOT EXISTS (
                 SELECT 1 FROM asset_locations AS location
                 WHERE location.source_generation = retained.source_generation))",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}
