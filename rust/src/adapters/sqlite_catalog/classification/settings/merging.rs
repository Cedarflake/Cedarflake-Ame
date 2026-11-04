use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rusqlite::params;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::merging_rules::validate_category_merge;
use crate::domain::classification::maintenance::{CategoryMergePlan, CategoryMergeSnapshot};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::CategoryMergeRepository;

use super::{SqliteCatalog, changed, coverage, database_error, operations};

mod basis;
mod evidence;

impl CategoryMergeRepository for SqliteCatalog {
    fn load_category_merge_snapshot(
        &self,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryMergeSnapshot, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let result = coverage::interruptible(&transaction, cancelled, basis::snapshot)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }

    fn apply_category_merge(
        &mut self,
        plan: &CategoryMergePlan,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        check_cancelled(cancelled)?;
        let configuration = plan.configuration();
        let payload = evidence::encode(plan)?;
        let mut hash = blake3::Hasher::new_derive_key("ame.classification.purpose-merge.v1");
        operations::field(&mut hash, super::request_digest(configuration)?.as_bytes());
        operations::field(&mut hash, payload.as_bytes());
        let digest = hash.finalize().to_hex().to_string();
        let transaction = self.begin_user_interactive_write()?;
        let result = coverage::interruptible(&transaction, cancelled, |connection| {
            if operations::replay(connection, &configuration.operation_id, &digest)?.is_none() {
                let current = basis::snapshot(connection)?;
                if current != plan.before
                    || !validate_category_merge(&current, &plan.coverage, &plan.merged, cancelled)?
                {
                    return Err(changed());
                }
                coverage::validate_snapshot(connection, &plan.coverage.snapshot)?;
                check_cancelled(cancelled)?;
                super::apply(connection, configuration, &digest)?.ok_or_else(changed)?;
                connection.execute(
                    "INSERT INTO classification_maintenance_evidence(operation_id, run_id, evidence_revision, payload)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![configuration.operation_id.as_str(), configuration.setup.run().id().as_str(),
                        blake3::hash(payload.as_bytes()).to_hex().to_string(), payload],
                ).map_err(database_error)?;
            }
            super::load(connection)
        })?;
        check_cancelled(cancelled)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
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
