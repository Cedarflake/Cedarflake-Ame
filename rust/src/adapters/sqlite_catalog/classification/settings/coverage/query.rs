use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCoverageEntry, CategoryCoveragePage, CategoryCoverageSnapshot,
};
use crate::domain::classification::{ClassificationContent, ClassificationRecordId};
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;

use super::super::super::super::exact_duplicates::queries::{
    COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS,
};
use super::super::super::super::exact_duplicates::read_unsigned;
use super::super::super::super::{database_error, sqlite_integer};
use super::super::super::{codec, invalid_record};

pub(super) fn unverified_sources(connection: &Connection) -> Result<bool, ScanError> {
    connection
        .query_row(
            &format!(
                "SELECT EXISTS (
            SELECT 1 FROM {CURRENT_LOCATIONS}
            WHERE NOT EXISTS (
                SELECT 1 FROM content_fingerprints AS f
                JOIN gallery_content_fingerprints AS published USING(source_generation, scheme)
                WHERE {COMPATIBLE_FINGERPRINT}))"
            ),
            [EXACT_FINGERPRINT_SCHEME],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn page(
    connection: &Connection,
    snapshot: &CategoryCoverageSnapshot,
    after: Option<&ClassificationContent>,
    limit: usize,
) -> Result<CategoryCoveragePage, ScanError> {
    if after.is_some_and(|content| content.scheme.as_str() != EXACT_FINGERPRINT_SCHEME) {
        return Err(invalid_record());
    }
    let digest = after
        .map(|content| codec::digest(&content.digest))
        .unwrap_or_default();
    let bytes = after.map_or(0, |content| content.byte_length);
    let configuration = snapshot
        .configuration
        .catalog
        .configuration
        .ok_or_else(invalid_record)?;
    let mut statement = connection.prepare(&page_sql()).map_err(database_error)?;
    let rows = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                digest,
                sqlite_integer(bytes, "content length")?,
                (limit + 1) as i64,
                codec::revision(configuration),
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    read_unsigned(row, 1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(database_error)?;
    let mut entries = Vec::with_capacity(limit);
    let mut has_more = false;
    for row in rows {
        let (digest, byte_length, payload, run) = row.map_err(database_error)?;
        if entries.len() == limit {
            has_more = true;
            break;
        }
        let content = ClassificationContent {
            scheme: ClassificationRecordId::new(EXACT_FINGERPRINT_SCHEME.into())
                .map_err(|_| invalid_record())?,
            digest: codec::read_digest(&digest)?,
            byte_length,
        };
        let prediction = match (payload, run) {
            (Some(payload), Some(run)) => {
                Some(codec::prediction::load(&payload, codec::decode_run(&run)?)?)
            }
            (None, None) => None,
            _ => return Err(invalid_record()),
        };
        entries.push(CategoryCoverageEntry {
            content,
            prediction,
        });
    }
    Ok(CategoryCoveragePage { entries, has_more })
}

fn page_sql() -> String {
    // Nullable file identities cannot bound a lookup for one source generation.
    let locations = CURRENT_LOCATIONS.replacen(
        "asset_locations AS l",
        "asset_locations AS l INDEXED BY asset_locations_exact_source",
        1,
    );
    format!("WITH contents AS MATERIALIZED (
        SELECT f.digest, f.byte_length FROM content_fingerprints AS f
        WHERE f.scheme = ?1 AND (f.digest, f.byte_length) > (?2, ?3)
          AND EXISTS (
              SELECT 1 FROM {locations}
              JOIN gallery_content_fingerprints AS published
                ON published.source_generation = f.source_generation AND published.scheme = f.scheme
              WHERE {COMPATIBLE_FINGERPRINT})
        GROUP BY f.digest, f.byte_length ORDER BY f.digest, f.byte_length LIMIT ?4)
    SELECT contents.digest, contents.byte_length, prediction.payload, run.payload FROM contents
    LEFT JOIN classification_predictions AS prediction ON prediction.publication_id = (
        SELECT publication_id FROM classification_predictions
        WHERE fingerprint_scheme = ?1 AND digest = contents.digest AND byte_length = contents.byte_length
          AND configuration_revision = ?5
        ORDER BY publication_id DESC LIMIT 1)
    LEFT JOIN classification_runs AS run ON run.run_id = prediction.run_id
    ORDER BY contents.digest, contents.byte_length")
}

#[cfg(test)]
mod tests;
