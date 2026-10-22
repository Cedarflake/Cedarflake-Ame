use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, sync_channel};
use std::thread::{Builder, Scope, ScopedJoinHandle};

use crate::adapters::{FileDiscovery, FileVisit, FileVisitOutcome};
use crate::domain::{MediaInspection, ScanError, ScanRequest};
use crate::ports::{MediaInspectionFailure, MediaInspector};

use super::FullScanReason;
use super::execution_registry::CONTROL_RUNNING;

const PREFETCH_CAPACITY: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InspectionMode {
    Serial,
    Prefetch,
}

impl InspectionMode {
    pub fn for_scan(
        request: &ScanRequest,
        reason: FullScanReason,
        has_active_locations: bool,
        visited_entries: u64,
    ) -> Self {
        if reason == FullScanReason::ExplicitUserRequest
            && !has_active_locations
            && visited_entries == 0
            && request.max_items.is_none()
            && request.max_entries.is_none()
        {
            Self::Prefetch
        } else {
            Self::Serial
        }
    }
}

pub(super) enum ScanInspection {
    Deferred,
    Complete(Result<MediaInspection, MediaInspectionFailure>),
}

pub(super) struct InspectedVisit {
    pub visit: FileVisit,
    pub inspection: ScanInspection,
}

pub(super) struct InspectionPrefetch<'scope> {
    receiver: Option<Receiver<InspectedVisit>>,
    stopped: Arc<AtomicBool>,
    worker: Option<ScopedJoinHandle<'scope, ()>>,
}

impl<'scope> InspectionPrefetch<'scope> {
    pub fn start<'env>(
        scope: &'scope Scope<'scope, 'env>,
        discovery: &'scope FileDiscovery,
        inspector: &'scope (impl MediaInspector + Sync),
        paths: Vec<String>,
        control: &'scope AtomicU8,
    ) -> Result<Self, ScanError> {
        Self::spawn(scope, paths, control, |path| {
            let visit = discovery.visit_relative_path(&path);
            let inspection = match &visit.outcome {
                FileVisitOutcome::File(file) => ScanInspection::Complete(inspector.inspect(file)),
                _ => ScanInspection::Deferred,
            };
            InspectedVisit { visit, inspection }
        })
    }

    fn spawn<'env>(
        scope: &'scope Scope<'scope, 'env>,
        paths: Vec<String>,
        control: &'scope AtomicU8,
        inspect: impl Fn(String) -> InspectedVisit + Send + 'scope,
    ) -> Result<Self, ScanError> {
        let (sender, receiver) = sync_channel(PREFETCH_CAPACITY);
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = Arc::clone(&stopped);
        let worker = Builder::new()
            .name("ame-import-inspection".to_owned())
            .spawn_scoped(scope, move || {
                for path in paths {
                    if worker_stopped.load(Ordering::Acquire)
                        || control.load(Ordering::Acquire) != CONTROL_RUNNING
                    {
                        break;
                    }
                    if sender.send(inspect(path)).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| {
                ScanError::new("scan_inspection_worker_unavailable", error.to_string())
            })?;
        Ok(Self {
            receiver: Some(receiver),
            stopped,
            worker: Some(worker),
        })
    }
}

impl Iterator for InspectionPrefetch<'_> {
    type Item = Result<InspectedVisit, ScanError>;

    fn next(&mut self) -> Option<Self::Item> {
        let receiver = self.receiver.as_ref()?;
        match measure_scan_operation!(InspectionWait, receiver.recv()) {
            Ok(visit) => Some(Ok(visit)),
            Err(_) => {
                self.receiver.take();
                self.worker.take().and_then(|worker| {
                    worker.join().err().map(|_| {
                        Err(ScanError::new(
                            "scan_inspection_worker_failed",
                            "The image inspection worker stopped unexpectedly",
                        ))
                    })
                })
            }
        }
    }
}

impl Drop for InspectionPrefetch<'_> {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        // Release a blocked sender before joining, including pause, cancellation and unwind.
        self.receiver.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests;
