use std::fs::File;
use std::path::Path;

use rusqlite::{Connection, TransactionBehavior, params};

use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationContent, ClassificationRevision, Embedding, FeatureIdentity, ImageFeature,
    ModelCompatibility, TextFeature,
};
use crate::ports::classification_features::{
    ClassificationFeatureCacheStats, ClassificationFeatureRepository, validate_text_feature_window,
};

use super::PreviewCacheNamespace;

mod read;
mod storage;

const DEFAULT_BUDGET: u64 = 256 * 1024 * 1024;
const ENTRY_ALLOWANCE: u64 = 256;
const MAX_VECTOR_BYTES: usize = 4096 * 4;

pub struct SqliteClassificationFeatureCache {
    connection: Connection,
    budget: u64,
    _file: File,
    _namespace: PreviewCacheNamespace,
}

impl SqliteClassificationFeatureCache {
    pub fn open(directory: &Path) -> Result<Self, ScanError> {
        Self::open_with_budget(directory, DEFAULT_BUDGET)
    }

    fn open_with_budget(directory: &Path, budget: u64) -> Result<Self, ScanError> {
        if !(MAX_VECTOR_BYTES as u64 + ENTRY_ALLOWANCE..=DEFAULT_BUDGET).contains(&budget) {
            return Err(invalid(
                "The feature-cache budget is outside its admitted bounds",
            ));
        }
        let (connection, file, namespace) = storage::open(directory)?;
        Ok(Self {
            connection,
            budget,
            _file: file,
            _namespace: namespace,
        })
    }

    fn load(
        &mut self,
        key: ClassificationRevision,
        dimensions: usize,
    ) -> Result<Option<Embedding>, ScanError> {
        Ok(
            read::load_features(&mut self.connection, &[key], dimensions)?
                .pop()
                .expect("one requested feature"),
        )
    }

    fn store(
        &mut self,
        key: ClassificationRevision,
        embedding: &Embedding,
    ) -> Result<(), ScanError> {
        let bytes: Vec<_> = embedding
            .values()
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let checksum = payload_digest(key, &bytes);
        let cost = bytes.len() as u64 + ENTRY_ALLOWANCE;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM features WHERE key = ?1)",
                [key.bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !exists {
            storage::make_room(&transaction, self.budget, cost)?;
            transaction
                .execute(
                    "INSERT INTO features(key, payload, checksum, touched)
                 VALUES (?1, ?2, ?3, (SELECT clock FROM feature_cache_state WHERE singleton = 1))",
                    params![key.bytes().as_slice(), bytes, checksum.bytes().as_slice()],
                )
                .map_err(database_error)?;
        }
        storage::touch(&transaction, key)?;
        transaction.commit().map_err(database_error)
    }
}

impl ClassificationFeatureRepository for SqliteClassificationFeatureCache {
    fn load_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
    ) -> Result<Option<ImageFeature>, ScanError> {
        Ok(self
            .load(image_key(model, content), model.dimensions())?
            .map(|embedding| ImageFeature {
                identity: model.image_identity(),
                embedding,
            }))
    }

    fn store_image_feature(
        &mut self,
        model: ModelCompatibility,
        content: &ClassificationContent,
        feature: &ImageFeature,
    ) -> Result<(), ScanError> {
        validate_feature(
            model.image_identity(),
            model.dimensions(),
            feature.identity,
            &feature.embedding,
        )?;
        self.store(image_key(model, content), &feature.embedding)
    }

    fn load_text_feature(
        &mut self,
        model: ModelCompatibility,
        description: ClassificationRevision,
    ) -> Result<Option<TextFeature>, ScanError> {
        Ok(self
            .load(text_key(model, description), model.dimensions())?
            .map(|embedding| TextFeature {
                identity: model.text_identity(),
                description,
                embedding,
            }))
    }

    fn store_text_feature(
        &mut self,
        model: ModelCompatibility,
        feature: &TextFeature,
    ) -> Result<(), ScanError> {
        validate_feature(
            model.text_identity(),
            model.dimensions(),
            feature.identity,
            &feature.embedding,
        )?;
        self.store(text_key(model, feature.description), &feature.embedding)
    }

    fn load_text_features(
        &mut self,
        model: ModelCompatibility,
        descriptions: &[ClassificationRevision],
    ) -> Result<Vec<Option<TextFeature>>, ScanError> {
        validate_text_feature_window(descriptions.len())?;
        let keys: Vec<_> = descriptions
            .iter()
            .map(|description| text_key(model, *description))
            .collect();
        let features = read::load_features(&mut self.connection, &keys, model.dimensions())?;
        Ok(descriptions
            .iter()
            .zip(features)
            .map(|(description, embedding)| {
                embedding.map(|embedding| TextFeature {
                    identity: model.text_identity(),
                    description: *description,
                    embedding,
                })
            })
            .collect())
    }

    fn feature_cache_stats(&self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        storage::stats(&self.connection)
    }

    fn clear_feature_cache_batch(&mut self) -> Result<ClassificationFeatureCacheStats, ScanError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        transaction.execute(
            "DELETE FROM features WHERE key IN (SELECT key FROM features ORDER BY touched, key LIMIT 256)", [],
        ).map_err(database_error)?;
        let remaining = storage::stats(&transaction)?;
        transaction.commit().map_err(database_error)?;
        self.connection
            .execute_batch("PRAGMA incremental_vacuum(256)")
            .map_err(database_error)?;
        Ok(remaining)
    }
}

fn validate_feature(
    expected: FeatureIdentity,
    dimensions: usize,
    identity: FeatureIdentity,
    embedding: &Embedding,
) -> Result<(), ScanError> {
    if identity != expected || embedding.values().len() != dimensions {
        return Err(invalid("The feature does not match its cache identity"));
    }
    Ok(())
}

fn image_key(model: ModelCompatibility, content: &ClassificationContent) -> ClassificationRevision {
    let identity = model.image_identity();
    ClassificationRevision::derive(
        "ame.classification.image-cache.v1",
        [
            identity.embedding_space.bytes().as_slice(),
            identity.pipeline.bytes(),
            &(model.dimensions() as u64).to_le_bytes(),
            content.scheme.as_str().as_bytes(),
            &content.digest,
            &content.byte_length.to_le_bytes(),
        ],
    )
}

fn text_key(
    model: ModelCompatibility,
    description: ClassificationRevision,
) -> ClassificationRevision {
    let identity = model.text_identity();
    ClassificationRevision::derive(
        "ame.classification.text-cache.v1",
        [
            identity.embedding_space.bytes().as_slice(),
            identity.pipeline.bytes(),
            &(model.dimensions() as u64).to_le_bytes(),
            description.bytes(),
        ],
    )
}

fn payload_digest(key: ClassificationRevision, bytes: &[u8]) -> ClassificationRevision {
    ClassificationRevision::derive(
        "ame.classification.feature-payload.v1",
        [key.bytes().as_slice(), bytes],
    )
}

fn invalid(message: impl Into<String>) -> ScanError {
    ScanError::new("classification_feature_cache_invalid", message)
}

fn database_error(error: rusqlite::Error) -> ScanError {
    ScanError::new(
        "classification_feature_cache_unavailable",
        format!("The feature cache could not be accessed: {error}"),
    )
}

#[cfg(test)]
mod tests;
