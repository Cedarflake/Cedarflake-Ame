pub(super) mod candidate_cache;
mod snapshot;

use rusqlite::{OptionalExtension, params};

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::OrganizationSelection;
use crate::domain::semantic_discovery::ImageContentIdentity;

use crate::domain::ScanError;
use crate::ports::semantic_catalog::{SemanticCatalog, SemanticCatalogSnapshot};

use super::{SqliteCatalog, database_error, sqlite_unsigned};

impl SemanticCatalog for SqliteCatalog {
    fn publish_semantic_content(
        &mut self,
        source: &OrganizationSelection,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        let published = super::exact_duplicates::gallery_publication::publish_verified_source(
            &transaction,
            source,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(published)
    }

    fn semantic_source(
        &self,
        source: &OrganizationSelection,
    ) -> Result<Option<crate::domain::exact_duplicates::FingerprintCandidate>, ScanError> {
        super::exact_duplicates::queries::load_selected(&self.connection, source)
    }

    fn semantic_content(
        &self,
        source: &OrganizationSelection,
    ) -> Result<Option<ImageContentIdentity>, ScanError> {
        let stored: Option<(String, i64)> = self
            .connection
            .query_row(
                &format!(
                    "SELECT f.digest, f.byte_length FROM {}
                JOIN content_fingerprints AS f ON {}
                WHERE l.asset_id = ?2 AND l.location_id = ?3 AND l.source_generation = ?4",
                    super::exact_duplicates::queries::CURRENT_LOCATIONS,
                    super::exact_duplicates::queries::COMPATIBLE_FINGERPRINT
                ),
                params![
                    EXACT_FINGERPRINT_SCHEME,
                    source.asset_id,
                    source.location_id,
                    super::sqlite_integer(source.source_generation, "source generation")?
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(database_error)?;
        stored
            .map(|(digest, bytes)| {
                let hash = blake3::Hash::from_hex(&digest).map_err(|_| changed())?;
                ImageContentIdentity::new(
                    EXACT_FINGERPRINT_SCHEME.to_owned(),
                    *hash.as_bytes(),
                    sqlite_unsigned(bytes, "content length")?,
                )
            })
            .transpose()
    }

    fn semantic_snapshot(&self) -> Result<Box<dyn SemanticCatalogSnapshot + '_>, ScanError> {
        Ok(Box::new(snapshot::Snapshot::open(self)?))
    }
}

fn changed() -> ScanError {
    ScanError::new(
        "semantic_content_invalid",
        "The image fingerprint is invalid",
    )
}
