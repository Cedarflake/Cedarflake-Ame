use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Connection, params};

use crate::application::classification_discovery::equivalent_retirements;
use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryRetirementPlan, CategoryRetirementSnapshot,
};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::CategoryRetirementRepository;

use super::super::maintenance;
use super::{SqliteCatalog, changed, codec, database_error, operations};

mod evidence;

impl CategoryRetirementRepository for SqliteCatalog {
    fn load_category_retirement_snapshot(&self) -> Result<CategoryRetirementSnapshot, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = snapshot(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn apply_category_retirement(
        &mut self,
        plan: &CategoryRetirementPlan,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        check_cancelled(cancelled)?;
        let payload = evidence::encode(plan)?;
        let mut hash = blake3::Hasher::new_derive_key("ame.classification.category-retirement.v1");
        operations::field(
            &mut hash,
            super::request_digest(&plan.configuration)?.as_bytes(),
        );
        operations::field(&mut hash, payload.as_bytes());
        let digest = hash.finalize().to_hex().to_string();
        let transaction = self.begin_user_interactive_write()?;
        if operations::replay(&transaction, &plan.configuration.operation_id, &digest)?.is_none() {
            let current = snapshot(&transaction)?;
            if current != plan.before
                || current.configuration.dispatch.mode != ClassificationDispatchMode::Automatic
                || current.model != Some(plan.configuration.setup.run().model())
                || equivalent_retirements(&current, cancelled)? != plan.retirements
                || plan.retirements.is_empty()
            {
                return Err(changed());
            }
            check_cancelled(cancelled)?;
            super::apply(&transaction, &plan.configuration, &digest)?.ok_or_else(changed)?;
            transaction.execute(
                "INSERT INTO classification_maintenance_evidence(operation_id, run_id, evidence_revision, payload)
                 VALUES (?1, ?2, ?3, ?4)",
                params![plan.configuration.operation_id.as_str(), plan.configuration.setup.run().id().as_str(),
                    blake3::hash(payload.as_bytes()).to_hex().to_string(), payload],
            ).map_err(database_error)?;
        }
        let result = super::load(&transaction)?;
        check_cancelled(cancelled)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

pub(super) fn snapshot(connection: &Connection) -> Result<CategoryRetirementSnapshot, ScanError> {
    let configuration = super::load(connection)?;
    let model = super::authority::active_model(connection, &configuration)?;
    let categories: Vec<_> = configuration
        .catalog
        .taxonomy
        .as_ref()
        .into_iter()
        .flat_map(|rules| {
            rules
                .categories()
                .iter()
                .map(|category| category.id().clone())
        })
        .collect();
    let encoded = codec::encode(
        &categories
            .iter()
            .map(|category| category.as_str())
            .collect::<Vec<_>>(),
        32768,
    )?;
    let mut statement = connection
        .prepare(
            "SELECT value FROM json_each(?1) AS category
         WHERE EXISTS (
             SELECT 1 FROM classification_decisions AS decision
             WHERE decision.category_id = category.value)",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([encoded], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    let mut human_categories = BTreeSet::new();
    for row in rows {
        human_categories.insert(codec::category_id(row.map_err(database_error)?)?);
    }
    Ok(CategoryRetirementSnapshot {
        configuration,
        model,
        origins: maintenance::load(connection, &categories)?,
        human_categories,
    })
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
