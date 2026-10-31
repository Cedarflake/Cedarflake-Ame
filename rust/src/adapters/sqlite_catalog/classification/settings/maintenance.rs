use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryExpansionPlan, CategoryExpansionSnapshot,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{CategoryId, ClassificationContent};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::CategoryExpansionRepository;

use super::super::super::exact_duplicates::queries::CURRENT_LOCATIONS;
use super::super::maintenance;
use super::{SqliteCatalog, changed, codec, database_error, operations, setup};

impl CategoryExpansionRepository for SqliteCatalog {
    fn load_category_expansion_snapshot(
        &self,
        proposed: &[CategoryId],
    ) -> Result<CategoryExpansionSnapshot, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let snapshot = snapshot(&transaction, proposed)?;
        transaction.commit().map_err(database_error)?;
        Ok(snapshot)
    }

    fn apply_category_expansion(
        &mut self,
        plan: &CategoryExpansionPlan,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        check_cancelled(cancelled)?;
        let payload = maintenance::encode(plan)?;
        let digest = request_digest(plan, &payload)?;
        let transaction = self.begin_user_interactive_write()?;
        if operations::replay(&transaction, &plan.configuration.operation_id, &digest)?.is_none() {
            let current = snapshot(&transaction, &plan.additions)?;
            let before = &current.configuration;
            if before.catalog.revision != plan.configuration.expected_catalog_revision
                || before.dispatch.revision != plan.configuration.expected_dispatch_revision
                || before.dispatch.mode != ClassificationDispatchMode::Automatic
                || current.model != Some(plan.configuration.setup.run().model())
                || !current.known_categories.is_empty()
            {
                return Err(changed());
            }
            validate_sources(&transaction, &plan.sources)?;
            check_cancelled(cancelled)?;
            super::apply(&transaction, &plan.configuration, &digest)?.ok_or_else(changed)?;
            maintenance::publish(&transaction, plan, &payload)?;
        }
        let result = super::load(&transaction)?;
        check_cancelled(cancelled)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

fn snapshot(
    connection: &Connection,
    proposed: &[CategoryId],
) -> Result<CategoryExpansionSnapshot, ScanError> {
    if proposed.is_empty() || proposed.len() > 64 {
        return Err(ScanError::new(
            "classification_category_limit",
            "Category expansion exceeds its candidate window",
        ));
    }
    let configuration = super::load(connection)?;
    let model = match configuration.dispatch.run_id.as_ref() {
        Some(run_id) => setup::load(connection, run_id.as_str())?
            .filter(|setup| {
                configuration.catalog.active_run_id.as_deref() == Some(run_id.as_str())
                    && configuration.catalog.configuration == Some(setup.run().configuration())
                    && configuration.catalog.taxonomy.as_ref() == Some(setup.primary())
            })
            .map(|setup| setup.run().model()),
        None => None,
    };
    let encoded =
        serde_json::to_string(&proposed.iter().map(CategoryId::as_str).collect::<Vec<_>>())
            .map_err(|_| changed())?;
    let mut statement = connection.prepare(
        "SELECT category_id FROM classification_categories WHERE category_id IN (SELECT value FROM json_each(?1))"
    ).map_err(database_error)?;
    let rows = statement
        .query_map([encoded], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    let mut known_categories = BTreeSet::new();
    for row in rows {
        known_categories
            .insert(CategoryId::new(row.map_err(database_error)?).map_err(|_| changed())?);
    }
    Ok(CategoryExpansionSnapshot {
        configuration,
        model,
        known_categories,
        managed_categories: maintenance::load(connection, proposed)?,
    })
}

fn validate_sources(
    connection: &Connection,
    sources: &[ClassificationContent],
) -> Result<(), ScanError> {
    let encoded = source_payload(sources)?;
    let sql = format!(
        "SELECT COUNT(*) FROM json_each(?1) AS proof
         WHERE NOT EXISTS (
             SELECT 1 FROM {CURRENT_LOCATIONS}
             JOIN content_fingerprints AS f ON f.source_generation = l.source_generation
               AND f.byte_length = l.file_size
               AND f.file_identity_scheme IS l.file_identity_scheme
               AND f.file_identity_value IS l.file_identity_value
               AND f.source_revision_token IS l.source_revision_token
             WHERE f.scheme = json_extract(proof.value, '$[0]')
               AND f.digest = json_extract(proof.value, '$[1]')
               AND f.byte_length = json_extract(proof.value, '$[2]')
               AND EXISTS (
                   SELECT 1 FROM gallery_content_fingerprints AS published
                   WHERE published.source_generation = f.source_generation
                     AND published.scheme = f.scheme))"
    );
    let missing: i64 = connection
        .query_row(&sql, [encoded], |row| row.get(0))
        .map_err(database_error)?;
    if missing > 0 {
        return Err(ScanError::new(
            "classification_discovery_sources_changed",
            "The category samples are no longer present in the current catalog",
        ));
    }
    Ok(())
}

fn source_payload(sources: &[ClassificationContent]) -> Result<String, ScanError> {
    if !(2..=512).contains(&sources.len()) {
        return Err(changed());
    }
    let values: Vec<_> = sources
        .iter()
        .map(|content| {
            (
                content.scheme.as_str(),
                codec::digest(&content.digest),
                content.byte_length,
            )
        })
        .collect();
    serde_json::to_string(&values).map_err(|_| changed())
}

fn request_digest(plan: &CategoryExpansionPlan, payload: &str) -> Result<String, ScanError> {
    let mut hasher = blake3::Hasher::new_derive_key("ame.classification.category-expansion.v1");
    operations::field(
        &mut hasher,
        super::request_digest(&plan.configuration)?.as_bytes(),
    );
    operations::field(&mut hasher, plan.evidence.bytes());
    operations::field(&mut hasher, payload.as_bytes());
    operations::field(&mut hasher, source_payload(&plan.sources)?.as_bytes());
    for category in &plan.additions {
        operations::field(&mut hasher, category.as_str().as_bytes());
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    Ok(())
}
