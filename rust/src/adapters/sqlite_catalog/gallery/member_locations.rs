use rusqlite::params;

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{EXACT_DUPLICATE_PAGE_SIZE, GalleryDuplicateCandidates};

use super::super::exact_duplicates::queries::{CANDIDATE_COLUMNS, CURRENT_LOCATIONS};
use super::super::exact_duplicates::{read_candidate, read_unsigned};
use super::super::{SqliteCatalog, database_error, sqlite_integer};
use super::duplicates::compatible_fingerprint;

pub(super) fn load(
    catalog: &mut SqliteCatalog,
    digest: &str,
    byte_length: u64,
    after: &str,
) -> Result<GalleryDuplicateCandidates, ScanError> {
    let transaction = catalog.connection.transaction().map_err(database_error)?;
    let matching = compatible_fingerprint("l");
    let scope = format!(
        "FROM {CURRENT_LOCATIONS}
         JOIN gallery_display_fingerprints AS f ON {matching}
         WHERE f.digest = ?1 AND f.byte_length = ?2"
    );
    let length = sqlite_integer(byte_length, "content length")?;
    let (location_count, pending_location_count) = transaction
        .query_row(
            &format!("SELECT COUNT(*), COALESCE(SUM(f.is_pending), 0) {scope}"),
            params![digest, length],
            |row| Ok((read_unsigned(row, 0)?, read_unsigned(row, 1)?)),
        )
        .map_err(database_error)?;
    let members = transaction
        .prepare(&format!(
            "SELECT {CANDIDATE_COLUMNS} {scope}
             AND l.location_id > ?3 ORDER BY l.location_id LIMIT ?4"
        ))
        .map_err(database_error)?
        .query_map(
            params![
                digest,
                length,
                after,
                (EXACT_DUPLICATE_PAGE_SIZE + 1) as i64
            ],
            read_candidate,
        )
        .map_err(database_error)?
        .map(|row| row.map_err(database_error)?)
        .collect::<Result<Vec<_>, ScanError>>()?;
    transaction.commit().map_err(database_error)?;
    Ok(GalleryDuplicateCandidates {
        members,
        location_count,
        pending_location_count,
    })
}
