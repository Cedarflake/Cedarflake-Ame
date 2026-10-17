use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use super::{RecoveryTask, RecoveryTaskKind, RecoveryTaskOutcome, RecoveryWorkRoot};
use crate::adapters::SqliteCatalogSession;
use crate::application::{
    process_ready_metadata_inventory_recovery_candidates_cancellable,
    process_ready_unowned_recovery_paths_cancellable,
};
use crate::domain::{
    IncrementalLibraryChangeReport, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibrarySynchronizationPhase, ScanError,
};

pub(in super::super) fn start_candidates(
    root: RecoveryWorkRoot,
    now_unix_ms: i64,
    catalog_session: Arc<SqliteCatalogSession>,
    policy: LibraryChangeQueuePolicy,
) -> Result<RecoveryTask, ScanError> {
    let RecoveryWorkRoot {
        root_id,
        root_generation,
        continuity_revision,
    } = root;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker_root_id = root_id.clone();
    let phase = Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation));
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::Builder::new()
        .name("ame-p2-candidate-drain".to_owned())
        .spawn(move || {
            let result = if worker_cancelled.load(Ordering::Acquire) {
                Ok(RecoveryTaskOutcome::CandidateDrain(
                    IncrementalLibraryChangeReport::default(),
                ))
            } else {
                catalog_session
                    .open_in_lane(LibraryChangeLane::Recovery)
                    .and_then(|mut catalog| {
                        process_ready_metadata_inventory_recovery_candidates_cancellable(
                            &mut catalog,
                            &worker_root_id,
                            root_generation,
                            now_unix_ms,
                            policy,
                            &worker_cancelled,
                        )
                        .map(RecoveryTaskOutcome::CandidateDrain)
                    })
            };
            let _ = sender.send(result);
        })
        .map_err(|error| {
            ScanError::new(
                "authoritative_recovery_worker_start_failed",
                format!("Could not start the P2 candidate drain page: {error}"),
            )
        })?;
    Ok(RecoveryTask {
        root_id,
        kind: RecoveryTaskKind::CandidateDrain {
            continuity_revision,
        },
        phase,
        cancelled,
        receiver,
        worker: Some(worker),
    })
}

pub(in super::super) fn start_legacy_unowned(
    root: RecoveryWorkRoot,
    now_unix_ms: i64,
    catalog_session: Arc<SqliteCatalogSession>,
    policy: LibraryChangeQueuePolicy,
) -> Result<RecoveryTask, ScanError> {
    let RecoveryWorkRoot {
        root_id,
        root_generation,
        continuity_revision,
    } = root;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker_root_id = root_id.clone();
    let phase = Arc::new(Mutex::new(LibrarySynchronizationPhase::Reconciliation));
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = thread::Builder::new()
        .name("ame-p2-legacy-unowned-drain".to_owned())
        .spawn(move || {
            let result = if worker_cancelled.load(Ordering::Acquire) {
                Ok(RecoveryTaskOutcome::LegacyUnownedDrain(
                    IncrementalLibraryChangeReport::default(),
                ))
            } else {
                catalog_session
                    .open_in_lane(LibraryChangeLane::Recovery)
                    .and_then(|mut catalog| {
                        let compacted = catalog.compact_legacy_unowned_recovery_controls(
                            &worker_root_id,
                            root_generation,
                            now_unix_ms,
                            policy,
                        )?;
                        let mut report = process_ready_unowned_recovery_paths_cancellable(
                            &mut catalog,
                            &worker_root_id,
                            root_generation,
                            now_unix_ms,
                            policy,
                            &worker_cancelled,
                        )?;
                        report.superseded_count = report
                            .superseded_count
                            .checked_add(compacted)
                            .ok_or_else(|| {
                                ScanError::new(
                                    "library_change_count_overflow",
                                    "The compacted legacy recovery count overflowed",
                                )
                            })?;
                        Ok(RecoveryTaskOutcome::LegacyUnownedDrain(report))
                    })
            };
            let _ = sender.send(result);
        })
        .map_err(|error| {
            ScanError::new(
                "authoritative_recovery_worker_start_failed",
                format!("Could not start the legacy unowned P2 drain page: {error}"),
            )
        })?;
    Ok(RecoveryTask {
        root_id,
        kind: RecoveryTaskKind::LegacyUnownedDrain {
            continuity_revision,
        },
        phase,
        cancelled,
        receiver,
        worker: Some(worker),
    })
}
