use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::adapters::{LocatedLibraryRoot, SqliteCatalogSession, locate_library_root};
use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, LibraryChangeLane, LibraryRootAvailability,
    LibrarySynchronizationSnapshot, ScanError,
};

use super::finish_runtime_task_until;

const RETRY_INTERVAL: Duration = Duration::from_secs(30);

type DirectoryLocator = dyn Fn(&str, &FileIdentityEvidence) -> Result<Option<LocatedLibraryRoot>, ScanError>
    + Send
    + Sync;

pub(super) struct RootLocationRecoveryOwner {
    locator: Arc<DirectoryLocator>,
    task: Option<LocationTask>,
    attempts: BTreeMap<String, LocationAttempt>,
    stopping: bool,
}

impl Default for RootLocationRecoveryOwner {
    fn default() -> Self {
        Self {
            locator: Arc::new(locate_library_root),
            task: None,
            attempts: BTreeMap::new(),
            stopping: false,
        }
    }
}

struct LocationAttempt {
    root: IncrementalCatalogRoot,
    next_check: Instant,
    issue_code: Option<String>,
}

struct LocationTask {
    root: IncrementalCatalogRoot,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Result<(), ScanError>>,
    worker: Option<JoinHandle<()>>,
}

impl RootLocationRecoveryOwner {
    #[cfg(test)]
    pub(super) fn with_locator(
        locator: impl Fn(&str, &FileIdentityEvidence) -> Result<Option<LocatedLibraryRoot>, ScanError>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            locator: Arc::new(locator),
            ..Self::default()
        }
    }

    pub(super) fn poll(
        &mut self,
        session: Arc<SqliteCatalogSession>,
        roots: Vec<IncrementalCatalogRoot>,
        snapshot: &mut LibrarySynchronizationSnapshot,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        let now = Instant::now();
        self.reap_finished(now);
        self.attempts
            .retain(|_, attempt| roots.iter().any(|root| same_binding(root, &attempt.root)));
        for status in &mut snapshot.roots {
            if let Some(attempt) = self.attempts.get(&status.root_id)
                && status.root_generation == attempt.root.root_generation.value()
                && attempt.issue_code.is_some()
            {
                status.last_issue_code.clone_from(&attempt.issue_code);
            }
        }
        if self.stopping || cancelled.load(Ordering::Acquire) || self.task.is_some() {
            return Ok(());
        }
        let root = roots.into_iter().find(|root| {
            root.active_scan_id.is_some()
                && !root.has_running_scan
                && root.publication_root_identity.is_some()
                && snapshot.roots.iter().any(|status| {
                    status.root_id == root.root_id
                        && status.root_generation == root.root_generation.value()
                        && status.availability == LibraryRootAvailability::Missing
                })
                && self
                    .attempts
                    .get(&root.root_id)
                    .is_none_or(|attempt| now >= attempt.next_check)
        });
        let Some(root) = root else {
            return Ok(());
        };
        let worker_root = root.clone();
        let worker_cancelled = Arc::clone(&cancelled);
        let locator = Arc::clone(&self.locator);
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("ame-root-location".to_owned())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    recover(&session, &worker_root, &worker_cancelled, locator.as_ref())
                }))
                .unwrap_or_else(|_| Err(worker_panic()));
                let _ = sender.send(result);
            })
            .map_err(|error| {
                ScanError::new(
                    "root_location_worker_unavailable",
                    format!("Could not start directory recovery: {error}"),
                )
            })?;
        self.task = Some(LocationTask {
            root,
            cancelled,
            receiver,
            worker: Some(worker),
        });
        Ok(())
    }

    fn reap_finished(&mut self, now: Instant) {
        let Some(task) = self.task.as_mut() else {
            return;
        };
        if task
            .worker
            .as_ref()
            .is_some_and(|worker| !worker.is_finished())
        {
            return;
        }
        let joined = task.worker.take().map(JoinHandle::join);
        let result = if joined.is_some_and(|result| result.is_err()) {
            Err(worker_panic())
        } else {
            task.receiver.try_recv().unwrap_or_else(|_| {
                Err(ScanError::new(
                    "root_location_worker_disconnected",
                    "Directory recovery ended without its result",
                ))
            })
        };
        self.attempts.insert(
            task.root.root_id.clone(),
            LocationAttempt {
                root: task.root.clone(),
                next_check: now + RETRY_INTERVAL,
                issue_code: result.err().map(|error| error.code),
            },
        );
        self.task = None;
    }

    pub(super) fn request_stop(&mut self) {
        self.stopping = true;
        if let Some(task) = &self.task {
            task.cancelled.store(true, Ordering::Release);
        }
    }

    pub(super) fn finish_stopping_until(&mut self, deadline: Instant) -> Result<(), ScanError> {
        self.request_stop();
        if let Some(task) = self.task.as_mut() {
            finish_runtime_task_until(
                &task.receiver,
                &mut task.worker,
                deadline,
                "root_location_stop_timeout",
                "Directory recovery exceeded the shutdown window",
            )?;
            self.task = None;
        }
        Ok(())
    }
}

fn same_binding(left: &IncrementalCatalogRoot, right: &IncrementalCatalogRoot) -> bool {
    left.root_id == right.root_id
        && left.root_generation == right.root_generation
        && left.root_path == right.root_path
        && left.active_scan_id == right.active_scan_id
        && left.publication_root_identity == right.publication_root_identity
}

fn recover(
    session: &SqliteCatalogSession,
    root: &IncrementalCatalogRoot,
    cancelled: &AtomicBool,
    locator: &DirectoryLocator,
) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(());
    }
    let Some(identity) = root.publication_root_identity.as_ref() else {
        return Ok(());
    };
    let Some(located) = locator(&root.root_path, identity)? else {
        return Ok(());
    };
    if cancelled.load(Ordering::Acquire) || located.path() == root.root_path {
        return Ok(());
    }
    let mut catalog = session.open_in_lane(LibraryChangeLane::Recovery)?;
    catalog.recover_located_root(root, &located, cancelled)
}

fn worker_panic() -> ScanError {
    ScanError::new(
        "root_location_worker_panicked",
        "Directory recovery worker panicked",
    )
}

#[cfg(test)]
mod tests;
