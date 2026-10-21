#[cfg(test)]
use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use super::super::recovery_sources::{RecoverySourceTransfer, RecoverySources};
use super::{
    RecoveryTask, RecoveryTaskKind, RecoveryTaskOutcome, RecoveryWorkPolicy, RecoveryWorkRoot,
};
use crate::adapters::SqliteCatalogSession;
use crate::application::{
    AuthoritativeLibraryChangeReport, MetadataInventoryProgressPhase,
    MetadataInventoryRecoveryExecution, MetadataInventoryRecoveryPage,
    MetadataInventoryRecoveryReport, MetadataInventoryWorkerControl, defer_authoritative_change,
    leased_change_requires_metadata_inventory,
    process_leased_authoritative_library_change_cancellable,
    process_leased_metadata_inventory_change_with_retained_source,
};
use crate::domain::{
    LeasedLibraryChange, LibraryChangeLane, LibrarySynchronizationPhase, ScanError,
};
use crate::ports::IncrementalCatalogRepository;

#[cfg(test)]
thread_local! { static FAIL_NEXT_RECOVERY_WORKER_SPAWN: Cell<bool> = const { Cell::new(false) }; }

pub(in super::super) fn start(
    root: RecoveryWorkRoot,
    leased: LeasedLibraryChange,
    now_unix_ms: i64,
    catalog_session: Arc<SqliteCatalogSession>,
    policy: RecoveryWorkPolicy,
    sources: &mut RecoverySources,
) -> Result<RecoveryTask, ScanError> {
    let RecoveryWorkRoot {
        root_id,
        root_generation,
        continuity_revision,
    } = root;
    let is_inventory = leased_change_requires_metadata_inventory(&leased);
    let change_id = leased.change.id;
    let task_kind = if is_inventory {
        RecoveryTaskKind::MetadataInventory {
            continuity_revision,
            change_id,
        }
    } else {
        RecoveryTaskKind::BoundedAuthoritative {
            continuity_revision,
        }
    };
    let recovery_kind = if is_inventory {
        "metadata-inventory"
    } else {
        "bounded-authoritative"
    };
    #[cfg(debug_assertions)]
    eprintln!("[Ame sync] recovery started kind={recovery_kind} root={root_id}");
    let queue_policy = policy.queue;
    let recovery_policy = policy.recovery;
    let metadata_inventory_page_entries = policy.metadata_inventory_page_entries;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker_root_id = root_id.clone();
    let initial_phase = if is_inventory {
        LibrarySynchronizationPhase::InventoryEnumeration
    } else {
        LibrarySynchronizationPhase::Reconciliation
    };
    let phase = Arc::new(Mutex::new(initial_phase));
    let worker_phase = Arc::clone(&phase);
    let retained_inventory_source = if is_inventory {
        sources.transfer(change_id)
    } else {
        RecoverySourceTransfer::empty(change_id)
    };
    let worker_retained_inventory_source = retained_inventory_source.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = spawn_recovery_worker(format!("ame-{recovery_kind}-recovery"), move || {
        let result = (|| {
            let retained_inventory_source = worker_retained_inventory_source.take()?;
            let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Recovery)?;
            let Some(root) = catalog.load_incremental_catalog_root(&worker_root_id)? else {
                return Ok(if is_inventory {
                    RecoveryTaskOutcome::MetadataInventory(Box::new(
                        MetadataInventoryRecoveryPage {
                            report: MetadataInventoryRecoveryReport::default(),
                            retained_source: None,
                        },
                    ))
                } else {
                    RecoveryTaskOutcome::Authoritative(AuthoritativeLibraryChangeReport::default())
                });
            };
            if root.root_generation != root_generation
                || root.active_scan_id.is_none()
                || root.has_running_scan
            {
                let deferred = defer_authoritative_change(
                    &mut catalog,
                    &leased,
                    root.catalog_revision,
                    now_unix_ms,
                )?;
                return Ok(if is_inventory {
                    RecoveryTaskOutcome::MetadataInventory(Box::new(
                        MetadataInventoryRecoveryPage {
                            report: MetadataInventoryRecoveryReport {
                                incremental: deferred.incremental,
                                ..MetadataInventoryRecoveryReport::default()
                            },
                            retained_source: None,
                        },
                    ))
                } else {
                    RecoveryTaskOutcome::Authoritative(deferred)
                });
            }
            if is_inventory {
                let report_progress = |progress| {
                    if let Ok(mut phase) = worker_phase.lock() {
                        *phase = synchronization_phase(progress);
                    }
                };
                let yield_page = |_phase| thread::yield_now();
                process_leased_metadata_inventory_change_with_retained_source(
                    &mut catalog,
                    &root,
                    &leased,
                    MetadataInventoryRecoveryExecution::new(
                        now_unix_ms,
                        metadata_inventory_page_entries,
                        queue_policy,
                        MetadataInventoryWorkerControl::with_progress_and_page_yield(
                            &worker_cancelled,
                            &report_progress,
                            &yield_page,
                        ),
                    )
                    .with_recovery_time_slice(),
                    retained_inventory_source,
                )
                .map(|page| RecoveryTaskOutcome::MetadataInventory(Box::new(page)))
            } else {
                process_leased_authoritative_library_change_cancellable(
                    &mut catalog,
                    &root,
                    &leased,
                    now_unix_ms,
                    queue_policy,
                    recovery_policy,
                    &worker_cancelled,
                )
                .map(RecoveryTaskOutcome::Authoritative)
            }
        })();
        let _ = sender.send(result);
    });
    let worker = match worker {
        Ok(worker) => worker,
        Err(error) => {
            sources.restore_failed_start(&retained_inventory_source)?;
            return Err(ScanError::new(
                "authoritative_recovery_worker_start_failed",
                format!("Could not start {recovery_kind} recovery: {error}"),
            ));
        }
    };
    Ok(RecoveryTask {
        root_id,
        kind: task_kind,
        phase,
        cancelled,
        receiver,
        worker: Some(worker),
    })
}

const fn synchronization_phase(
    phase: MetadataInventoryProgressPhase,
) -> LibrarySynchronizationPhase {
    match phase {
        MetadataInventoryProgressPhase::Enumeration => {
            LibrarySynchronizationPhase::InventoryEnumeration
        }
        MetadataInventoryProgressPhase::Comparison => {
            LibrarySynchronizationPhase::InventoryComparison
        }
        MetadataInventoryProgressPhase::QueuePublication => {
            LibrarySynchronizationPhase::QueuePublication
        }
    }
}

fn spawn_recovery_worker(
    name: String,
    worker: impl FnOnce() + Send + 'static,
) -> std::io::Result<JoinHandle<()>> {
    #[cfg(test)]
    if FAIL_NEXT_RECOVERY_WORKER_SPAWN.with(|failure| failure.replace(false)) {
        return Err(std::io::Error::other(
            "injected recovery worker spawn failure",
        ));
    }
    thread::Builder::new().name(name).spawn(worker)
}

#[cfg(test)]
pub(in super::super) fn fail_next_recovery_worker_spawn() {
    FAIL_NEXT_RECOVERY_WORKER_SPAWN.with(|failure| failure.set(true));
}
