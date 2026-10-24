use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization_cleanup::{
    OrganizationCleanupInspection, OrganizationCleanupPhase, OrganizationCleanupProgress,
    OrganizationCleanupReceipt, OrganizationCleanupRequest,
};
use crate::ports::organization_cleanup::OrganizationCleanupRepository;

use super::super::{SqliteCatalog, database_error, load_catalog_revision};
use super::operations::{self, Admission};

mod inspection;
mod retirement;

impl OrganizationCleanupRepository for SqliteCatalog {
    fn inspect_organization_cleanup(
        &mut self,
        cutoff_unix_ms: Option<i64>,
        progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    ) -> Result<OrganizationCleanupInspection, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let inspection = inspection::stage(&transaction, cutoff_unix_ms, progress)?;
        report(
            progress,
            OrganizationCleanupPhase::Inspecting,
            inspection.groups,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(inspection)
    }

    fn apply_organization_cleanup(
        &mut self,
        request: &OrganizationCleanupRequest,
        progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    ) -> Result<OrganizationCleanupReceipt, ScanError> {
        request.validate()?;
        let expected = &request.inspection;
        let digest = operations::request_digest(
            "cleanup",
            expected.organization_revision,
            &[
                &expected.catalog_revision.to_string(),
                &expected.cutoff_unix_ms.to_string(),
                &expected.selection_digest,
            ],
        );
        let transaction = self.begin_user_interactive_write()?;
        let admission = operations::admit(
            &transaction,
            &request.operation_id,
            &digest,
            "cleanup",
            expected.organization_revision,
        )?;
        let receipt = match admission {
            Admission::Replayed(receipt) => receipt,
            Admission::New { revision } => {
                if load_catalog_revision(&transaction)? != expected.catalog_revision {
                    return Err(changed());
                }
                let actual =
                    inspection::stage(&transaction, Some(expected.cutoff_unix_ms), progress)?;
                if actual.selection_digest != expected.selection_digest {
                    return Err(changed());
                }
                let removed = actual.groups + actual.duplicate_choices;
                retirement::apply(&transaction, progress)?;
                report(progress, OrganizationCleanupPhase::Applying, removed)?;
                operations::finish(
                    &transaction,
                    &request.operation_id,
                    revision,
                    removed,
                    false,
                )?
            }
        };
        transaction.commit().map_err(database_error)?;
        Ok(OrganizationCleanupReceipt {
            operation_id: receipt.operation_id,
            organization_revision: receipt.organization_revision,
            removed_records: receipt.changed_groups,
        })
    }
}

fn changed() -> ScanError {
    ScanError::new(
        "organization_cleanup_changed",
        "The records changed after inspection; inspect them again before cleanup",
    )
}

fn report(
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    phase: OrganizationCleanupPhase,
    examined_records: u64,
) -> Result<(), ScanError> {
    if !progress(OrganizationCleanupProgress {
        phase,
        examined_records,
    }) {
        return Err(ScanError::new(
            "organization_cleanup_cancelled",
            "Cleanup was cancelled",
        ));
    }
    Ok(())
}

fn reset_selection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_groups (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;
         CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_duplicates (
           scheme TEXT NOT NULL, digest TEXT NOT NULL, byte_length INTEGER NOT NULL,
           PRIMARY KEY(scheme, digest, byte_length)
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_cleanup_groups;
         DELETE FROM temp.organization_cleanup_duplicates;",
        )
        .map_err(database_error)
}
