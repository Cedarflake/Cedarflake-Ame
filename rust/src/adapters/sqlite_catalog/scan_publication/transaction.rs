use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::ScanPublicationControl;

use super::super::{
    SqliteCatalog, SqliteWritePreemptCallback, database_error, load_scan_catch_up_lineage,
    sqlite_integer, sqlite_unsigned, unix_time_ms,
};
use super::{ScanPublicationReceipt, ValidatedStagingProof};

const PUBLICATION_PROGRESS_OPERATION_INTERVAL: i32 = 1_000;

#[derive(Clone)]
pub(super) struct PublicationInterruption {
    preempted: Arc<AtomicBool>,
    control: ScanPublicationControl,
    active: Arc<AtomicBool>,
    callback_gate: Arc<Mutex<()>>,
}

impl PublicationInterruption {
    fn new(control: &ScanPublicationControl) -> Self {
        Self {
            preempted: Arc::new(AtomicBool::new(false)),
            control: control.clone(),
            active: Arc::new(AtomicBool::new(true)),
            callback_gate: Arc::new(Mutex::new(())),
        }
    }

    fn is_interrupted(&self) -> bool {
        self.active.load(Ordering::Acquire)
            && (self.control.is_requested() || self.preempted.load(Ordering::Acquire))
    }

    fn retirement_guard(&self) -> AttemptRetirement {
        AttemptRetirement(self.clone())
    }

    fn retire(&self) {
        let _gate = self
            .callback_gate
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.active.store(false, Ordering::Release);
    }

    fn preemption_callback(
        &self,
        interrupt: impl Fn() + Send + Sync + 'static,
    ) -> SqliteWritePreemptCallback {
        let state = self.clone();
        Arc::new(move || {
            let _gate = state
                .callback_gate
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if state.active.load(Ordering::Acquire) {
                state.preempted.store(true, Ordering::Release);
                interrupt();
            }
        })
    }

    pub(super) fn ensure_running(&self) -> Result<(), ScanError> {
        if self.control.is_requested() {
            Err(ScanError::new(
                "catalog_scan_publication_controlled",
                "The scan requested interruption before catalog publication committed",
            ))
        } else if self.preempted.load(Ordering::Acquire) {
            Err(ScanError::new(
                "catalog_scan_publication_preempted",
                "A newer live or journal change preempted catalog publication",
            ))
        } else {
            Ok(())
        }
    }

    fn classify_error(&self, error: ScanError) -> ScanError {
        if error.code == "catalog_database_interrupted"
            && let Err(controlled) = self.ensure_running()
        {
            return controlled;
        }
        error
    }

    #[cfg(test)]
    pub(super) fn preemption_flag(&self) -> &AtomicBool {
        &self.preempted
    }
}

struct AttemptRetirement(PublicationInterruption);

impl Drop for AttemptRetirement {
    fn drop(&mut self) {
        self.0.retire();
    }
}

pub(crate) fn publish_scan_with_proof(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    asset_count: u64,
    issue_count: u64,
    proof: Option<&ValidatedStagingProof>,
    control: &ScanPublicationControl,
) -> Result<ScanPublicationReceipt, ScanError> {
    let interruption = PublicationInterruption::new(control);
    let _attempt = interruption.retirement_guard();
    interruption.ensure_running()?;
    let retry_relative_paths = catalog.pending_authoritative_retry_paths.clone();
    catalog.flush_pending_locations()?;
    let _reported_asset_count = sqlite_integer(asset_count, "reported asset count")?;
    let issue_count = sqlite_integer(issue_count, "issue count")?;
    let progress = interruption.clone();
    catalog
        .connection
        .progress_handler(
            PUBLICATION_PROGRESS_OPERATION_INTERVAL,
            Some(move || progress.is_interrupted()),
        )
        .map_err(database_error)?;

    let result = publish_transaction(
        catalog,
        scan_id,
        root_id,
        issue_count,
        &retry_relative_paths,
        &interruption,
        proof,
    );
    let _handler_cleanup = catalog.connection.progress_handler(0, None::<fn() -> bool>);

    match result {
        Ok(receipt) => {
            // COMMIT, not the last observed command, owns the terminal result. A late request
            // cannot retroactively revoke the committed catalog or restore its previous revision.
            catalog.pending_authoritative_retry_paths.clear();
            Ok(receipt)
        }
        Err(error) => Err(interruption.classify_error(error)),
    }
}

fn publish_transaction(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    issue_count: i64,
    retry_relative_paths: &[String],
    interruption: &PublicationInterruption,
    proof: Option<&ValidatedStagingProof>,
) -> Result<ScanPublicationReceipt, ScanError> {
    interruption.ensure_running()?;
    let interrupt = Arc::new(catalog.connection.get_interrupt_handle());
    let preempt = interruption.preemption_callback(move || interrupt.interrupt());
    let transaction =
        catalog.begin_preemptible_write_in_lane(LibraryChangeLane::Recovery, preempt)?;
    // Reverse local-drop order retires callbacks before rollback, while the transaction still
    // owns its write permit. This also covers errors and unwinding from an interrupted phase.
    let _transaction_attempt = interruption.retirement_guard();
    interruption.ensure_running()?;
    let authority = super::load_publication_authority(
        &transaction,
        scan_id,
        root_id,
        !retry_relative_paths.is_empty(),
    )?;
    super::ensure_change_queue_is_publishable(&transaction, scan_id, root_id, &authority)?;
    if let Some(proof) = proof {
        proof.require_current_staging(&transaction, scan_id, root_id)?;
    }

    let completed_unix_ms = unix_time_ms();
    super::reconcile_identity_pages(&transaction, scan_id, interruption)?;
    let asset_count = super::count_staged_assets(&transaction, scan_id, root_id, interruption)?;
    let catch_up_lineage = load_scan_catch_up_lineage(&transaction, scan_id)?;
    super::retain_previous_snapshot_handoffs(
        &transaction,
        scan_id,
        root_id,
        authority.previous_active_scan.as_deref(),
        completed_unix_ms,
    )?;
    super::complete_scan_and_replace_projection(
        &transaction,
        scan_id,
        root_id,
        &authority,
        asset_count,
        issue_count,
        completed_unix_ms,
        interruption,
    )?;
    let published_revision = super::publish_root_authority(
        &transaction,
        scan_id,
        root_id,
        &authority,
        completed_unix_ms,
        interruption,
    )?;
    super::settle_change_lineage(
        &transaction,
        scan_id,
        root_id,
        &authority,
        retry_relative_paths,
        &catch_up_lineage,
        published_revision,
        completed_unix_ms,
        interruption,
    )?;
    let receipt = ScanPublicationReceipt {
        asset_count: sqlite_unsigned(asset_count, "published asset count")?,
    };
    interruption.ensure_running()?;
    // Admission to COMMIT is the cancellation cutoff; rollback on a COMMIT error also runs
    // without the old attempt's progress or priority callback interrupting cleanup.
    interruption.retire();
    transaction.commit().map_err(database_error)?;
    Ok(receipt)
}

#[cfg(test)]
mod tests;
