use std::sync::Arc;
use std::sync::atomic::Ordering;

use rusqlite::params;

use crate::domain::organization::{
    MAX_ORGANIZATION_SELECTION, OrganizationWriteReceipt, stale_selection, validate_identifier,
};
use crate::domain::organization_batch::{
    OrganizationBatchAction, OrganizationBatchPhase, OrganizationBatchSummary,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::organization_batch::{OrganizationBatchControl, OrganizationBatchRepository};

use super::super::exact_duplicates::read_unsigned;
use super::super::{SqliteCatalog, database_error, gallery};
use super::{albums, authority, changes, favorites, inspection, operations, selection};

mod preparation;
mod staging;

struct PreparedBatch {
    summary: OrganizationBatchSummary,
    authority: String,
    decisions: String,
}

pub(crate) struct SqliteOrganizationBatch {
    catalog: SqliteCatalog,
    query: GalleryQuery,
    catalog_revision: u64,
    all_matching: bool,
    prepared: Option<PreparedBatch>,
    control: Arc<OrganizationBatchControl>,
}

impl SqliteOrganizationBatch {
    #[cfg(test)]
    pub(super) fn cancel_on_record_write(&self) {
        let control = Arc::clone(&self.control);
        self.catalog
            .connection
            .update_hook(Some(
                move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                    if table == "organization_records" {
                        control.cancelled.store(true, Ordering::Release);
                    }
                },
            ))
            .expect("install record-write cancellation hook");
    }

    pub(crate) fn new(
        catalog: SqliteCatalog,
        query: GalleryQuery,
        catalog_revision: u64,
        all_matching: bool,
        control: Arc<OrganizationBatchControl>,
    ) -> Result<Self, ScanError> {
        gallery::validate_gallery_query(&query)?;
        control.check()?;
        staging::initialize(&catalog.connection)?;
        let cancellation = Arc::clone(&control);
        catalog
            .connection
            .progress_handler(
                1000,
                Some(move || cancellation.cancelled.load(Ordering::Acquire)),
            )
            .map_err(database_error)?;
        Ok(Self {
            catalog,
            query,
            catalog_revision,
            all_matching,
            prepared: None,
            control,
        })
    }

    fn apply_inner(
        &mut self,
        operation_id: &str,
        action: &OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        self.control.check()?;
        validate_identifier(operation_id)?;
        let prepared = self.prepared.as_ref().ok_or_else(stale_selection)?;
        let (kind, payload) = match action {
            OrganizationBatchAction::Favorite { favorite } => ("favorite", favorite.to_string()),
            OrganizationBatchAction::Organize { action } => {
                action.validate()?;
                (
                    "organization",
                    format!(
                        "{}:{}",
                        action.request_name(),
                        action.decision().1.unwrap_or("")
                    ),
                )
            }
        };
        let digest = operations::request_digest(
            kind,
            prepared.summary.organization_revision,
            &[&prepared.authority, &payload],
        );
        let transaction = self.catalog.begin_user_interactive_write()?;
        let revision = match operations::admit(
            &transaction,
            operation_id,
            &digest,
            kind,
            prepared.summary.organization_revision,
        )? {
            operations::Admission::Replayed(receipt) => {
                transaction.commit().map_err(database_error)?;
                return Ok(receipt);
            }
            operations::Admission::New { revision } => revision,
        };
        if revisions(&transaction)?.0 != prepared.summary.catalog_revision {
            return Err(stale_selection());
        }
        if let OrganizationBatchAction::Organize { action } = action
            && let Some(album_id) = action.decision().1
        {
            albums::require_active(&transaction, album_id)?;
        }
        let total = prepared.summary.group_count;
        self.control
            .report(OrganizationBatchPhase::Saving, 0, total);
        let mut after = 0;
        let mut completed = 0;
        let mut changed = 0;
        let mut reconciled = false;
        loop {
            let groups = staging::groups(&transaction, after)?;
            if groups.is_empty() {
                break;
            }
            for group in groups {
                self.control.check()?;
                if group.target.as_ref() != Some(&selection::target(&transaction, &group.key)?) {
                    return Err(stale_selection());
                }
                let result = match action {
                    OrganizationBatchAction::Favorite { favorite } => favorites::apply_group(
                        &transaction,
                        operation_id,
                        revision,
                        &group.key,
                        *favorite,
                    )?,
                    OrganizationBatchAction::Organize { action } => changes::apply_group(
                        &transaction,
                        operation_id,
                        revision,
                        &group.key,
                        action,
                    )?,
                };
                changed += u64::from(result.changed);
                reconciled |= result.reconciled;
                after = group.sequence;
                completed += 1;
                self.control
                    .report(OrganizationBatchPhase::Saving, completed, total);
            }
        }
        self.control.check()?;
        let receipt =
            operations::finish(&transaction, operation_id, revision, changed, reconciled)?;
        self.control.check()?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }

    fn terminal_error(&self, error: ScanError) -> ScanError {
        let progress = self.control.snapshot();
        if let Err(cancelled) = self.control.check() {
            self.control.report(
                OrganizationBatchPhase::Cancelled,
                progress.completed_groups,
                progress.total_groups,
            );
            return cancelled;
        }
        self.control.report(
            OrganizationBatchPhase::Failed,
            progress.completed_groups,
            progress.total_groups,
        );
        error
    }
}

impl OrganizationBatchRepository for SqliteOrganizationBatch {
    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError> {
        self.control.check()?;
        if self.prepared.is_some() || asset_ids.len() > MAX_ORGANIZATION_SELECTION {
            return Err(ScanError::new(
                "organization_selection_invalid",
                "The selection is prepared or this input chunk is oversized",
            ));
        }
        for asset_id in asset_ids {
            validate_identifier(asset_id)?;
        }
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        {
            let mut statement = transaction
                .prepare("INSERT OR IGNORE INTO temp.organization_batch_ids VALUES (?1)")
                .map_err(database_error)?;
            for asset_id in asset_ids {
                statement
                    .execute(params![asset_id])
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn prepare(&mut self) -> Result<OrganizationBatchSummary, ScanError> {
        self.prepare_inner(false)
            .map_err(|error| self.terminal_error(error))
    }

    fn refresh(&mut self) -> Result<OrganizationBatchSummary, ScanError> {
        self.prepare_inner(true)
            .map_err(|error| self.terminal_error(error))
    }

    fn apply(
        &mut self,
        operation_id: &str,
        action: &OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        let receipt = self
            .apply_inner(operation_id, action)
            .map_err(|error| self.terminal_error(error))?;
        let count = self
            .prepared
            .as_ref()
            .map_or(0, |prepared| prepared.summary.group_count);
        self.control
            .report(OrganizationBatchPhase::Completed, count, count);
        Ok(receipt)
    }
}

fn revisions(connection: &rusqlite::Connection) -> Result<(u64, u64), ScanError> {
    connection
        .query_row(
            "SELECT catalog.revision, organization.revision
             FROM catalog_state AS catalog, organization_state AS organization
             WHERE organization.singleton = 1",
            [],
            |row| Ok((read_unsigned(row, 0)?, read_unsigned(row, 1)?)),
        )
        .map_err(database_error)
}
