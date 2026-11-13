use rusqlite::{Row, Transaction, params_from_iter, types::Value};

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
    projections: &'a super::super::gallery_projection_cache::GalleryProjectionCache,
    candidates: &'a super::candidate_cache::CandidateCache,
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
            projections: &catalog.session.gallery_projections,
            candidates: &catalog.session.semantic_candidates,
        })
    }

    fn read_windows(
        &self,
        query: &GalleryQuery,
        after_location: Option<&str>,
        consume: &mut dyn FnMut(Vec<SemanticCandidate>) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        let membership = after_location
            .is_none()
            .then(|| self.projections.membership(query, self.revision))
            .flatten();
        let query = candidate_query(query, after_location, membership.as_deref())?;
        let mut statement = self
            .transaction
            .prepare(&query.sql)
            .map_err(database_error)?;
        let mut rows = statement
            .query(params_from_iter(query.parameters))
            .map_err(database_error)?;
        let mut window = Vec::with_capacity(SEMANTIC_CANDIDATE_WINDOW);
        while let Some(row) = rows.next().map_err(database_error)? {
            window.push(read_candidate(row)?);
            if window.len() == SEMANTIC_CANDIDATE_WINDOW {
                consume(std::mem::replace(
                    &mut window,
                    Vec::with_capacity(SEMANTIC_CANDIDATE_WINDOW),
                ))?;
            }
        }
        if !window.is_empty() {
            consume(window)?;
        }
        Ok(self.revision)
    }
}

impl SemanticCatalogSnapshot for Snapshot<'_> {
    fn candidates(
        &self,
        query: &GalleryQuery,
        after_location: &str,
    ) -> Result<SemanticCandidateWindow, ScanError> {
        let mut candidates = Vec::new();
        self.read_windows(query, Some(after_location), &mut |window| {
            candidates = window;
            Ok(())
        })?;
        Ok(SemanticCandidateWindow {
            revision: self.revision,
            candidates,
        })
    }

    fn scan(
        &self,
        query: &GalleryQuery,
        consume: &mut dyn FnMut(Vec<SemanticCandidate>) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        if let Some(candidates) = self.candidates.get(query, self.revision) {
            for window in candidates.chunks(SEMANTIC_CANDIDATE_WINDOW) {
                consume(window.to_vec())?;
            }
            return Ok(self.revision);
        }
        // Retain only scopes already admitted by the revision-bound gallery cache.
        // Fingerprint publication advances that same revision; vector storage is read afresh.
        let mut capture = self
            .projections
            .membership(query, self.revision)
            .map(|_| super::candidate_cache::Capture::new(query));
        // Keep one SQLite cursor so grouped scope is evaluated once per search.
        self.read_windows(query, None, &mut |window| {
            if let Some(capture) = capture.as_mut() {
                capture.append(&window);
            }
            consume(window)
        })?;
        if let Some(capture) = capture {
            self.candidates.publish(query, self.revision, capture);
        }
        Ok(self.revision)
    }
}

fn candidate_query(
    query: &GalleryQuery,
    after_location: Option<&str>,
    membership: Option<&super::super::gallery_read_projection::GalleryReadProjection>,
) -> Result<gallery::BuiltGalleryQuery, ScanError> {
    gallery::validate_gallery_query(query)?;
    let mut clauses = Vec::new();
    let mut parameters = Vec::new();
    let source = if let Some(membership) = membership {
        parameters.push(Value::Text(
            serde_json::to_string(&membership.location_ids().collect::<Vec<_>>())
                .map_err(|error| ScanError::new("catalog_query_invalid", error.to_string()))?,
        ));
        "FROM json_each(?) AS selected
         CROSS JOIN asset_locations AS locations INDEXED BY asset_locations_location_id
           ON locations.location_id = selected.value
         JOIN library_roots AS roots
           ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id"
            .to_owned()
    } else {
        let source = gallery::semantic_candidate_source(query, &mut parameters);
        gallery::push_gallery_filters(query, &mut clauses, &mut parameters);
        source
    };
    let limit = if let Some(after) = after_location {
        clauses.push("locations.location_id > ?".to_owned());
        parameters.push(Value::Text(after.to_owned()));
        parameters.push(Value::Integer(SEMANTIC_CANDIDATE_WINDOW as i64));
        "LIMIT ?"
    } else {
        ""
    };
    let predicate = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    Ok(gallery::BuiltGalleryQuery {
        sql: format!(
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
             {predicate} ORDER BY locations.location_id {limit}"
        ),
        parameters,
    })
}

fn read_candidate(row: &Row<'_>) -> Result<SemanticCandidate, ScanError> {
    let digest: Option<String> = row.get(3).map_err(database_error)?;
    let bytes: Option<i64> = row.get(4).map_err(database_error)?;
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
            asset_id: row.get(0).map_err(database_error)?,
            location_id: row.get(1).map_err(database_error)?,
            source_generation: sqlite_unsigned(
                row.get(2).map_err(database_error)?,
                "source generation",
            )?,
        },
        content,
    })
}
