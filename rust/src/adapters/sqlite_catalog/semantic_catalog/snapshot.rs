use rusqlite::{Transaction, params_from_iter, types::Value};

use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::OrganizationSelection;
use crate::domain::semantic_discovery::ImageContentIdentity;
use crate::domain::semantic_results::SEMANTIC_CANDIDATE_WINDOW;
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::semantic_catalog::{
    SemanticCandidate, SemanticCandidateWindow, SemanticCatalogSnapshot,
};

use super::super::{
    SqliteCatalog, database_error, gallery, load_catalog_revision, sqlite_unsigned,
};
use super::changed;

pub(super) struct Snapshot<'a> {
    transaction: Transaction<'a>,
    revision: u64,
}

impl<'a> Snapshot<'a> {
    pub(super) fn open(catalog: &'a SqliteCatalog) -> Result<Self, ScanError> {
        let transaction = catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        Ok(Self {
            transaction,
            revision,
        })
    }
}

impl SemanticCatalogSnapshot for Snapshot<'_> {
    fn candidates(
        &self,
        query: &GalleryQuery,
        after_location: &str,
    ) -> Result<SemanticCandidateWindow, ScanError> {
        gallery::validate_gallery_query(query)?;
        let mut clauses = Vec::new();
        let mut parameters = Vec::new();
        let source = gallery::gallery_source(query, &mut parameters);
        gallery::push_gallery_filters(query, &mut clauses, &mut parameters);
        clauses.push("locations.location_id > ?".to_owned());
        parameters.push(Value::Text(after_location.to_owned()));
        parameters.push(Value::Integer(SEMANTIC_CANDIDATE_WINDOW as i64));
        let sql = format!(
            "SELECT locations.asset_id, locations.location_id, locations.source_generation,
                    evidence.digest, evidence.byte_length
             {source}
             LEFT JOIN content_fingerprints AS evidence
               ON evidence.source_generation = locations.source_generation
              AND evidence.scheme = '{EXACT_FINGERPRINT_SCHEME}'
              AND evidence.byte_length = locations.file_size
              AND evidence.file_identity_scheme IS locations.file_identity_scheme
              AND evidence.file_identity_value IS locations.file_identity_value
              AND evidence.source_revision_token IS locations.source_revision_token
              AND EXISTS (SELECT 1 FROM gallery_content_fingerprints AS published
                  WHERE published.source_generation = evidence.source_generation
                    AND published.scheme = evidence.scheme)
             WHERE {} ORDER BY locations.location_id LIMIT ?",
            clauses.join(" AND ")
        );
        let mut statement = self.transaction.prepare(&sql).map_err(database_error)?;
        let candidates = statement
            .query_map(params_from_iter(parameters), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(database_error)?
            .map(|row| {
                let (asset_id, location_id, generation, digest, bytes) =
                    row.map_err(database_error)?;
                let content = digest
                    .zip(bytes)
                    .map(|(digest, bytes)| {
                        let hash = blake3::Hash::from_hex(&digest).map_err(|_| changed())?;
                        ImageContentIdentity::new(
                            EXACT_FINGERPRINT_SCHEME.to_owned(),
                            *hash.as_bytes(),
                            sqlite_unsigned(bytes, "content length")?,
                        )
                    })
                    .transpose()?;
                Ok(SemanticCandidate {
                    source: OrganizationSelection {
                        asset_id,
                        location_id,
                        source_generation: sqlite_unsigned(generation, "source generation")?,
                    },
                    content,
                })
            })
            .collect::<Result<Vec<_>, ScanError>>()?;
        drop(statement);
        Ok(SemanticCandidateWindow {
            revision: self.revision,
            candidates,
        })
    }
}
