use std::sync::Arc;
use std::sync::atomic::Ordering;

use rusqlite::params;

use crate::application::classification_decisions::plan_classification_change;
use crate::domain::classification::{ClassificationAction, ClassificationRecordId};
use crate::domain::classification_catalog::ClassificationWriteReceipt;
use crate::domain::classification_review::{
    ClassificationReviewReceipt, ClassificationReviewSession,
};
use crate::domain::classification_selection::{
    ClassificationSelectionPhase, ClassificationSelectionSummary,
};
use crate::domain::organization::{MAX_ORGANIZATION_SELECTION, validate_identifier};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::classification_selection::{
    ClassificationSelectionControl, ClassificationSelectionRepository,
};

use super::super::{SqliteCatalog, database_error, gallery, organization};
use super::{changed, codec, configuration, observations, operations, writes};
use gallery::selection_staging as staging;

mod commands;
mod preparation;
mod review;

struct PreparedSelection {
    summary: ClassificationSelectionSummary,
    digest: String,
}

pub(crate) struct SqliteClassificationSelection {
    catalog: SqliteCatalog,
    query: GalleryQuery,
    catalog_revision: u64,
    all_matching: bool,
    prepared: Option<PreparedSelection>,
    control: Arc<ClassificationSelectionControl>,
}

impl SqliteClassificationSelection {
    pub(crate) fn new(
        catalog: SqliteCatalog,
        query: GalleryQuery,
        catalog_revision: u64,
        all_matching: bool,
        control: Arc<ClassificationSelectionControl>,
    ) -> Result<Self, ScanError> {
        gallery::validate_gallery_query(&query)?;
        control.check()?;
        catalog.connection.set_prepared_statement_cache_capacity(64);
        staging::initialize(&catalog.connection)?;
        catalog.connection.execute_batch(
            "CREATE TEMP TABLE classification_selection_observations(
                sequence INTEGER PRIMARY KEY, observation TEXT NOT NULL, confirmable INTEGER NOT NULL
             );",
        ).map_err(database_error)?;
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

    fn failed(&self, error: ScanError) -> ScanError {
        let progress = self.control.snapshot();
        let cancelled = self.control.check().err();
        self.control.report(
            if cancelled.is_some() {
                ClassificationSelectionPhase::Cancelled
            } else {
                ClassificationSelectionPhase::Failed
            },
            progress.completed_groups,
            progress.total_groups,
        );
        cancelled.unwrap_or(error)
    }
}

impl ClassificationSelectionRepository for SqliteClassificationSelection {
    fn apply_review(
        &mut self,
        id: &ClassificationRecordId,
        session: &ClassificationReviewSession,
        action: &ClassificationAction,
    ) -> Result<ClassificationReviewReceipt, ScanError> {
        let receipt = self
            .apply_review_inner(id, session, action)
            .map_err(|error| self.failed(error))?;
        self.control
            .report(ClassificationSelectionPhase::Completed, 1, 1);
        Ok(receipt)
    }

    fn append_targets(
        &mut self,
        targets: &[crate::domain::organization::OrganizationSelection],
    ) -> Result<(), ScanError> {
        crate::domain::organization::validate_selections(targets)?;
        organization::selection::prepare(&self.catalog.connection, targets)?;
        self.append_ids(
            &targets
                .iter()
                .map(|target| target.asset_id.clone())
                .collect::<Vec<_>>(),
        )
    }

    fn append_ids(&mut self, asset_ids: &[String]) -> Result<(), ScanError> {
        self.control.check()?;
        if self.prepared.is_some() || asset_ids.len() > MAX_ORGANIZATION_SELECTION {
            return Err(ScanError::new(
                "classification_selection_invalid",
                "The selection is prepared or the input chunk is oversized",
            ));
        }
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        for id in asset_ids {
            validate_identifier(id)?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO temp.organization_batch_ids VALUES (?1)",
                    params![id],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    fn prepare(&mut self) -> Result<ClassificationSelectionSummary, ScanError> {
        self.prepare_inner().map_err(|error| self.failed(error))
    }

    fn apply(
        &mut self,
        id: &ClassificationRecordId,
        action: &ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        let receipt = self
            .apply_inner(id, action)
            .map_err(|error| self.failed(error))?;
        let total = self
            .prepared
            .as_ref()
            .map_or(0, |value| value.summary.group_count);
        self.control
            .report(ClassificationSelectionPhase::Completed, total, total);
        Ok(receipt)
    }
}
