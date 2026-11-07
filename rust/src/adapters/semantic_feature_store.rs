use std::fs::File;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::domain::ScanError;
use crate::domain::image_features::{
    AnalysisRevision, Embedding, ImageFeature, ModelCompatibility,
};
use crate::domain::semantic_discovery::{
    FeaturePublication, ImageContentIdentity, SemanticFeatureStorage,
};
use crate::ports::semantic_features::SemanticFeatureRepository;

use super::PreviewCacheNamespace;

mod storage;

const DEFAULT_BUDGET: u64 = 1024 * 1024 * 1024;
const ENTRY_ALLOWANCE: u64 = 512;

pub(crate) struct SqliteSemanticFeatureStore {
    connection: Connection,
    budget: u64,
    _file: File,
    _namespace: PreviewCacheNamespace,
}

impl SqliteSemanticFeatureStore {
    pub(crate) fn open(directory: &Path) -> Result<Self, ScanError> {
        Self::open_with_budget(directory, DEFAULT_BUDGET)
    }

    fn open_with_budget(directory: &Path, budget: u64) -> Result<Self, ScanError> {
        if !(ENTRY_ALLOWANCE + 4..=DEFAULT_BUDGET).contains(&budget) {
            return Err(invalid("The semantic feature budget is invalid"));
        }
        let (connection, file, namespace) = storage::open(directory)?;
        let store = Self {
            connection,
            budget,
            _file: file,
            _namespace: namespace,
        };
        store.storage()?;
        Ok(store)
    }
}

impl SemanticFeatureRepository for SqliteSemanticFeatureStore {
    fn load_image(
        &self,
        model: ModelCompatibility,
        content: &ImageContentIdentity,
    ) -> Result<Option<ImageFeature>, ScanError> {
        let key = content.feature_key(model);
        let row: Option<(Vec<u8>, Vec<u8>)> = self.connection.query_row(
            "SELECT substr(vector, 1, ?2), substr(checksum, 1, 33) FROM image_features WHERE feature_key = ?1",
            params![key.bytes().as_slice(), model.dimensions() as i64 * 4 + 1],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(database_error)?;
        row.map(|(payload, checksum)| decode(model, key, &payload, &checksum))
            .transpose()
    }

    fn publish_image(
        &mut self,
        model: ModelCompatibility,
        content: &ImageContentIdentity,
        feature: &ImageFeature,
    ) -> Result<FeaturePublication, ScanError> {
        if feature.identity != model.image_identity()
            || feature.embedding.values().len() != model.dimensions()
        {
            return Err(invalid(
                "The image feature does not match the admitted model",
            ));
        }
        let key = content.feature_key(model);
        let payload: Vec<u8> = feature
            .embedding
            .values()
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let checksum = checksum(key, &payload);
        let cost = payload.len() as u64 + ENTRY_ALLOWANCE;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let existing: Option<(Vec<u8>, Vec<u8>)> = transaction.query_row(
            "SELECT substr(vector, 1, ?2), substr(checksum, 1, 33) FROM image_features WHERE feature_key = ?1",
            params![key.bytes().as_slice(), model.dimensions() as i64 * 4 + 1], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(database_error)?;
        if let Some((payload, stored_checksum)) = existing {
            decode(model, key, &payload, &stored_checksum)?;
            transaction.commit().map_err(database_error)?;
            return Ok(FeaturePublication::AlreadyPresent);
        }
        let accounted: u64 = transaction
            .query_row(
                "SELECT accounted_bytes FROM semantic_store_state WHERE singleton = 1",
                [],
                |row| unsigned(row, 0),
            )
            .map_err(database_error)?;
        if accounted.saturating_add(cost) > self.budget {
            transaction
                .execute(
                    "UPDATE semantic_store_state SET budget_reached = 1 WHERE singleton = 1",
                    [],
                )
                .map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
            return Ok(FeaturePublication::BudgetReached);
        }
        transaction.execute(
            "INSERT INTO image_features(feature_key, content_scheme, content_digest, byte_length,
                embedding_space, image_pipeline, dimensions, vector, checksum)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![key.bytes().as_slice(), content.scheme(), content.digest().as_slice(), content.byte_length() as i64,
                feature.identity.embedding_space.bytes().as_slice(), feature.identity.pipeline.bytes().as_slice(),
                model.dimensions() as i64, payload, checksum.bytes().as_slice()],
        ).map_err(database_error)?;
        transaction.execute(
            "UPDATE semantic_store_state SET generation = generation + 1, feature_count = feature_count + 1,
                accounted_bytes = accounted_bytes + ?1 WHERE singleton = 1", [cost as i64],
        ).map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(FeaturePublication::Stored)
    }

    fn storage(&self) -> Result<SemanticFeatureStorage, ScanError> {
        self.connection
            .query_row(
                "SELECT generation, feature_count, accounted_bytes, budget_reached
             FROM semantic_store_state WHERE singleton = 1",
                [],
                |row| {
                    Ok(SemanticFeatureStorage {
                        generation: unsigned(row, 0)?,
                        features: unsigned(row, 1)?,
                        accounted_bytes: unsigned(row, 2)?,
                        budget_bytes: self.budget,
                        budget_reached: row.get(3)?,
                    })
                },
            )
            .map_err(database_error)
    }
}

fn checksum(key: AnalysisRevision, payload: &[u8]) -> AnalysisRevision {
    AnalysisRevision::derive(
        "ame.semantic.image-feature.v1",
        [key.bytes().as_slice(), payload],
    )
}

fn decode(
    model: ModelCompatibility,
    key: AnalysisRevision,
    payload: &[u8],
    expected: &[u8],
) -> Result<ImageFeature, ScanError> {
    if payload.len() != model.dimensions() * 4 || checksum(key, payload).bytes() != expected {
        return Err(invalid("The stored image feature is damaged"));
    }
    let values = payload
        .chunks_exact(4)
        .map(|value| f32::from_le_bytes(value.try_into().expect("fixed float")))
        .collect();
    Ok(ImageFeature {
        identity: model.image_identity(),
        embedding: Embedding::from_normalized_values(values)
            .map_err(|error| invalid(error.to_string()))?,
    })
}

fn invalid(message: impl Into<String>) -> ScanError {
    ScanError::new("semantic_storage_invalid", message)
}

fn database_error(error: rusqlite::Error) -> ScanError {
    ScanError::new("semantic_storage_failed", error.to_string())
}

fn unsigned(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

#[cfg(test)]
mod tests;
