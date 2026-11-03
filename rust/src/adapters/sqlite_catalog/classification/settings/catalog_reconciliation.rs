use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::params;
use serde_json::{Value, json};

use crate::domain::ScanError;
use crate::domain::classification::maintenance::CategoryCatalogReconciliationPlan;
use crate::domain::classification::maintenance::catalog_retirements;
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::CategoryCatalogRepository;

use super::{SqliteCatalog, changed, codec, database_error, operations, retirement};

impl CategoryCatalogRepository for SqliteCatalog {
    fn apply_category_catalog_reconciliation(
        &mut self,
        plan: &CategoryCatalogReconciliationPlan,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        check_cancelled(cancelled)?;
        let payload = encode(plan)?;
        let mut hash =
            blake3::Hasher::new_derive_key("ame.classification.catalog-reconciliation.v1");
        operations::field(
            &mut hash,
            super::request_digest(&plan.configuration)?.as_bytes(),
        );
        operations::field(&mut hash, payload.as_bytes());
        let digest = hash.finalize().to_hex().to_string();
        let transaction = self.begin_user_interactive_write()?;
        if operations::replay(&transaction, &plan.configuration.operation_id, &digest)?.is_none() {
            let current = retirement::snapshot(&transaction)?;
            if current != plan.before
                || current.configuration.dispatch.mode != ClassificationDispatchMode::Automatic
                || current.model != Some(plan.configuration.setup.run().model())
                || catalog_retirements(&current, &plan.catalog, cancelled)? != plan.retired
                || plan.retired.is_empty()
            {
                return Err(changed());
            }
            check_cancelled(cancelled)?;
            super::apply(&transaction, &plan.configuration, &digest)?.ok_or_else(changed)?;
            transaction
                .execute(
                    "INSERT INTO classification_maintenance_evidence
                    (operation_id, run_id, evidence_revision, payload)
                 VALUES (?1, ?2, ?3, ?4)",
                    params![
                        plan.configuration.operation_id.as_str(),
                        plan.configuration.setup.run().id().as_str(),
                        blake3::hash(payload.as_bytes()).to_hex().to_string(),
                        payload
                    ],
                )
                .map_err(database_error)?;
        }
        let result = super::load(&transaction)?;
        check_cancelled(cancelled)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

fn encode(plan: &CategoryCatalogReconciliationPlan) -> Result<String, ScanError> {
    let before = plan
        .before
        .configuration
        .catalog
        .taxonomy
        .as_ref()
        .ok_or_else(changed)?;
    let retired = before
        .categories()
        .iter()
        .filter(|category| plan.retired.contains(category.id()))
        .map(|category| {
            json!({
                "id": category.id().as_str(),
                "label": category.label(),
                "semantic_revision": codec::revision(category.semantic_revision()),
            })
        })
        .collect::<Vec<_>>();
    codec::encode(
        &json!({
            "version": 1,
            "kind": "managed_catalog_reconciliation",
            "catalog_revision": codec::revision(plan.catalog.revision),
            "allowed_categories": plan.catalog.categories.iter().map(|category| category.as_str()).collect::<Vec<_>>(),
            "previous_rules": codec::revision(before.revision()),
            "run": serde_json::from_str::<Value>(&codec::encode_run(plan.configuration.setup.run())?).map_err(|_| changed())?,
            "retired": retired,
        }),
        524288,
    )
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
