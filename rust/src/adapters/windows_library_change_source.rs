use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::domain::{
    LibraryChangeObservation, LibraryChangeObservationKind, LibraryChangeOrigin,
    LibraryChangeScope, LibraryChangeSourceBatch, LibraryChangeSourceError,
    LibraryChangeSourceHealth, LibraryChangeSourceStopReport, LibraryRootGeneration,
};
use crate::ports::{LibraryChangeSource, LibraryChangeSourceRequest};

const MAX_INGRESS_CAPACITY: usize = 4096;
const RENAME_PAIR_GRACE: Duration = Duration::from_millis(50);
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

pub struct WindowsLibraryChangeSource {
    watcher: Option<RecommendedWatcher>,
    receiver: Receiver<LibraryChangeObservation>,
    callback_state: Arc<CallbackState>,
}

struct CallbackState {
    root_id: String,
    root_generation: LibraryRootGeneration,
    root_path: PathBuf,
    sender: SyncSender<LibraryChangeObservation>,
    delivery_gate: Mutex<()>,
    accepting: AtomicBool,
    health: AtomicU8,
    evidence_gap: AtomicBool,
    dropped_observation_count: AtomicU64,
    ignored_callback_count: AtomicU64,
    next_sequence: AtomicU64,
    pending_rename_from: Mutex<Option<PendingRenameFrom>>,
}

struct PendingRenameFrom {
    path: PathBuf,
    observed_at: Instant,
}

struct CallbackProcessor {
    state: Arc<CallbackState>,
}

pub(super) fn start_windows_library_change_source(
    request: &LibraryChangeSourceRequest,
) -> Result<WindowsLibraryChangeSource, LibraryChangeSourceError> {
    validate_request(request)?;
    let root_path = std::fs::canonicalize(&request.root_path).map_err(|_| {
        LibraryChangeSourceError::retryable(
            "change_source_root_unavailable",
            "The library root could not be resolved for observation.",
        )
    })?;
    let (sender, receiver) = sync_channel(request.ingress_capacity);
    let callback_state = Arc::new(CallbackState {
        root_id: request.root_id.clone(),
        root_generation: request.root_generation,
        root_path: root_path.clone(),
        sender,
        delivery_gate: Mutex::new(()),
        accepting: AtomicBool::new(true),
        health: AtomicU8::new(health_code(LibraryChangeSourceHealth::Starting)),
        evidence_gap: AtomicBool::new(false),
        dropped_observation_count: AtomicU64::new(0),
        ignored_callback_count: AtomicU64::new(0),
        next_sequence: AtomicU64::new(1),
        pending_rename_from: Mutex::new(None),
    });
    let processor_state = Arc::clone(&callback_state);
    let mut processor = CallbackProcessor {
        state: processor_state,
    };
    let mut watcher =
        notify::recommended_watcher(move |result| processor.handle(result)).map_err(|_| {
            callback_state.set_health(LibraryChangeSourceHealth::Failed);
            LibraryChangeSourceError::retryable(
                "change_source_start_failed",
                "The Windows library observer could not be created.",
            )
        })?;
    watcher
        .watch(&root_path, RecursiveMode::Recursive)
        .map_err(|_| {
            callback_state.accepting.store(false, Ordering::Release);
            callback_state.set_health(LibraryChangeSourceHealth::Failed);
            LibraryChangeSourceError::retryable(
                "change_source_watch_failed",
                "The library root could not be observed recursively.",
            )
        })?;
    callback_state.compare_health(
        LibraryChangeSourceHealth::Starting,
        LibraryChangeSourceHealth::Healthy,
    );

    Ok(WindowsLibraryChangeSource {
        watcher: Some(watcher),
        receiver,
        callback_state,
    })
}

impl LibraryChangeSource for WindowsLibraryChangeSource {
    fn health(&self) -> LibraryChangeSourceHealth {
        self.callback_state.health()
    }

    fn drain(
        &mut self,
        max_observations: usize,
    ) -> Result<LibraryChangeSourceBatch, LibraryChangeSourceError> {
        if max_observations == 0 || max_observations > MAX_INGRESS_CAPACITY {
            return Err(LibraryChangeSourceError::new(
                "change_source_drain_limit_invalid",
                "The observation drain limit must be within the bounded ingress capacity.",
            ));
        }

        let dropped_observation_count = self
            .callback_state
            .dropped_observation_count
            .swap(0, Ordering::AcqRel);
        let ignored_callback_count = self
            .callback_state
            .ignored_callback_count
            .swap(0, Ordering::AcqRel);
        let mut observations = Vec::with_capacity(max_observations);
        self.callback_state.flush_expired_rename();
        if self
            .callback_state
            .evidence_gap
            .swap(false, Ordering::AcqRel)
        {
            observations.push(self.callback_state.evidence_gap_observation());
        }
        while observations.len() < max_observations {
            match self.receiver.try_recv() {
                Ok(observation) => observations.push(observation),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.callback_state
                        .set_health(LibraryChangeSourceHealth::Failed);
                    return Err(LibraryChangeSourceError::retryable(
                        "change_source_ingress_disconnected",
                        "The library observation ingress disconnected unexpectedly.",
                    ));
                }
            }
        }

        Ok(LibraryChangeSourceBatch {
            observations,
            health: self.health(),
            dropped_observation_count,
            ignored_callback_count,
        })
    }

    fn stop(&mut self) -> Result<LibraryChangeSourceStopReport, LibraryChangeSourceError> {
        let started = Instant::now();
        self.callback_state.stop_accepting();
        self.callback_state
            .set_health(LibraryChangeSourceHealth::Stopped);
        let Some(mut watcher) = self.watcher.take() else {
            return Ok(stop_report(started, &self.callback_state));
        };
        let (finished_sender, finished_receiver) = sync_channel(1);
        thread::Builder::new()
            .name("ame-notify-stop".to_owned())
            .spawn(move || {
                let result = watcher.shutdown().map_err(|_| {
                    LibraryChangeSourceError::new(
                        "change_source_native_stop_failed",
                        "The native Windows library observer did not shut down cleanly.",
                    )
                });
                let _ = finished_sender.send(result);
            })
            .map_err(|_| {
                LibraryChangeSourceError::new(
                    "change_source_stop_thread_failed",
                    "The library observer shutdown task could not be started.",
                )
            })?;
        finished_receiver.recv_timeout(STOP_TIMEOUT).map_err(|_| {
            LibraryChangeSourceError::new(
                "change_source_stop_timeout",
                "The library observer did not stop within the bounded shutdown interval.",
            )
        })??;
        Ok(stop_report(started, &self.callback_state))
    }
}

impl Drop for WindowsLibraryChangeSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl CallbackProcessor {
    fn handle(&mut self, result: notify::Result<Event>) {
        if !self.state.accepting.load(Ordering::Acquire) {
            self.state
                .ignored_callback_count
                .fetch_add(1, Ordering::Relaxed);
            return;
        }
        let event = match result {
            Ok(event) => event,
            Err(_) => {
                self.flush_incomplete_rename();
                self.state
                    .mark_evidence_gap(LibraryChangeSourceHealth::Failed);
                return;
            }
        };
        if event.need_rescan() {
            self.flush_incomplete_rename();
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        }
        if matches!(event.kind, EventKind::Remove(_))
            && event.paths.iter().any(|path| path == &self.state.root_path)
        {
            self.flush_incomplete_rename();
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Failed);
            return;
        }

        match event.kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                self.handle_rename_from(&event.paths)
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                self.handle_rename_to(&event.paths)
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                self.flush_incomplete_rename();
                self.handle_paired_rename(&event.paths);
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Any | RenameMode::Other)) => {
                self.flush_incomplete_rename();
                self.state
                    .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                self.emit_paths(&event.paths, LibraryChangeObservationKind::Modified, None);
            }
            EventKind::Access(_) => self.flush_incomplete_rename(),
            EventKind::Create(kind) => {
                self.flush_incomplete_rename();
                self.emit_create_paths(&event.paths, kind);
            }
            EventKind::Remove(kind) => {
                self.flush_incomplete_rename();
                self.emit_remove_paths(&event.paths, kind);
            }
            EventKind::Modify(_) | EventKind::Any | EventKind::Other => {
                self.flush_incomplete_rename();
                self.emit_modify_paths(&event.paths);
            }
        }
    }

    fn handle_rename_from(&mut self, paths: &[PathBuf]) {
        self.flush_incomplete_rename();
        if paths.len() == 1 {
            if let Ok(mut pending) = self.state.pending_rename_from.lock() {
                *pending = paths.first().cloned().map(|path| PendingRenameFrom {
                    path,
                    observed_at: Instant::now(),
                });
            } else {
                self.state
                    .mark_evidence_gap(LibraryChangeSourceHealth::Failed);
            }
        } else {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
        }
    }

    fn handle_rename_to(&mut self, paths: &[PathBuf]) {
        if paths.len() != 1 {
            self.flush_incomplete_rename();
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        }
        let pending = self
            .state
            .pending_rename_from
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());
        let Some(pending) = pending else {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            self.emit_paths(paths, LibraryChangeObservationKind::Created, None);
            return;
        };
        if pending.observed_at.elapsed() > RENAME_PAIR_GRACE {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            self.emit_paths(paths, LibraryChangeObservationKind::Created, None);
            return;
        }
        let current_path = paths.first().expect("single rename target");
        self.emit_rename(&pending.path, current_path);
    }

    fn handle_paired_rename(&self, paths: &[PathBuf]) {
        if paths.len() != 2 {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        }
        self.emit_rename(&paths[0], &paths[1]);
    }

    fn emit_rename(&self, previous_path: &Path, current_path: &Path) {
        let Some(previous_relative_path) = self.state.relative_path(previous_path) else {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        };
        let Some(relative_path) = self.state.relative_path(current_path) else {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        };
        let scope = match std::fs::metadata(current_path) {
            Ok(metadata) if metadata.is_dir() => LibraryChangeScope::Subtree,
            Ok(_) => LibraryChangeScope::Path,
            Err(_) => {
                self.state
                    .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                LibraryChangeScope::Path
            }
        };
        self.state.emit(LibraryChangeObservation {
            root_id: self.state.root_id.clone(),
            root_generation: self.state.root_generation,
            sequence: self.state.next_sequence(),
            observed_unix_ms: observed_unix_ms(),
            kind: LibraryChangeObservationKind::Renamed {
                is_reliably_paired: true,
            },
            scope,
            relative_path,
            previous_relative_path: Some(previous_relative_path),
            origin: LibraryChangeOrigin::LiveNotification,
        });
    }

    fn emit_paths(
        &self,
        paths: &[PathBuf],
        kind: LibraryChangeObservationKind,
        scope: Option<LibraryChangeScope>,
    ) {
        if paths.is_empty() {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            return;
        }
        for path in paths {
            let Some(relative_path) = self.state.relative_path(path) else {
                self.state
                    .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                continue;
            };
            self.state.emit(LibraryChangeObservation {
                root_id: self.state.root_id.clone(),
                root_generation: self.state.root_generation,
                sequence: self.state.next_sequence(),
                observed_unix_ms: observed_unix_ms(),
                kind,
                scope: scope.unwrap_or(LibraryChangeScope::Path),
                relative_path,
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
            });
        }
    }

    fn emit_create_paths(&self, paths: &[PathBuf], kind: CreateKind) {
        for path in paths {
            let is_directory = match std::fs::metadata(path) {
                Ok(metadata) => kind == CreateKind::Folder || metadata.is_dir(),
                Err(_) if kind == CreateKind::Folder => true,
                Err(_) => {
                    self.state
                        .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                    false
                }
            };
            self.emit_paths(
                std::slice::from_ref(path),
                if is_directory {
                    LibraryChangeObservationKind::DirectoryChanged
                } else {
                    LibraryChangeObservationKind::Created
                },
                is_directory.then_some(LibraryChangeScope::Subtree),
            );
        }
        if paths.is_empty() {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
        }
    }

    fn emit_remove_paths(&self, paths: &[PathBuf], kind: RemoveKind) {
        let is_known_file = kind == RemoveKind::File;
        self.emit_paths(
            paths,
            if is_known_file {
                LibraryChangeObservationKind::Removed
            } else {
                LibraryChangeObservationKind::DirectoryChanged
            },
            (!is_known_file).then_some(LibraryChangeScope::Subtree),
        );
    }

    fn emit_modify_paths(&self, paths: &[PathBuf]) {
        for path in paths {
            let is_directory = match std::fs::metadata(path) {
                Ok(metadata) => metadata.is_dir(),
                Err(_) => {
                    self.state
                        .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                    false
                }
            };
            self.emit_paths(
                std::slice::from_ref(path),
                if is_directory {
                    LibraryChangeObservationKind::DirectoryChanged
                } else {
                    LibraryChangeObservationKind::Modified
                },
                is_directory.then_some(LibraryChangeScope::Subtree),
            );
        }
        if paths.is_empty() {
            self.state
                .mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
        }
    }

    fn flush_incomplete_rename(&mut self) {
        self.state.flush_incomplete_rename();
    }
}

impl CallbackState {
    fn emit(&self, observation: LibraryChangeObservation) {
        let _delivery_guard = self
            .delivery_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.accepting.load(Ordering::Acquire) {
            self.ignored_callback_count.fetch_add(1, Ordering::Relaxed);
            return;
        }
        match self.sender.try_send(observation) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                self.dropped_observation_count
                    .fetch_add(1, Ordering::Relaxed);
                self.mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
            }
            Err(TrySendError::Disconnected(_)) => {
                self.dropped_observation_count
                    .fetch_add(1, Ordering::Relaxed);
                self.mark_evidence_gap(LibraryChangeSourceHealth::Failed);
            }
        }
    }

    fn stop_accepting(&self) {
        let _delivery_guard = self
            .delivery_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.accepting.store(false, Ordering::Release);
    }

    fn relative_path(&self, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(&self.root_path).ok()?;
        let value = relative.to_str()?.replace('\\', "/");
        (!value.contains('\0')).then_some(value)
    }

    fn evidence_gap_observation(&self) -> LibraryChangeObservation {
        LibraryChangeObservation {
            root_id: self.root_id.clone(),
            root_generation: self.root_generation,
            sequence: self.next_sequence(),
            observed_unix_ms: observed_unix_ms(),
            kind: LibraryChangeObservationKind::EvidenceGap,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
        }
    }

    fn flush_incomplete_rename(&self) {
        match self.pending_rename_from.lock() {
            Ok(mut pending) => {
                if pending.take().is_some() {
                    self.mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                }
            }
            Err(_) => self.mark_evidence_gap(LibraryChangeSourceHealth::Failed),
        }
    }

    fn flush_expired_rename(&self) {
        match self.pending_rename_from.lock() {
            Ok(mut pending) => {
                if pending
                    .as_ref()
                    .is_some_and(|rename| rename.observed_at.elapsed() >= RENAME_PAIR_GRACE)
                {
                    pending.take();
                    self.mark_evidence_gap(LibraryChangeSourceHealth::Degraded);
                }
            }
            Err(_) => self.mark_evidence_gap(LibraryChangeSourceHealth::Failed),
        }
    }

    fn mark_evidence_gap(&self, health: LibraryChangeSourceHealth) {
        self.evidence_gap.store(true, Ordering::Release);
        self.raise_health(health);
    }

    fn next_sequence(&self) -> u64 {
        self.next_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(1))
            })
            .unwrap_or(u64::MAX)
    }

    fn health(&self) -> LibraryChangeSourceHealth {
        health_from_code(self.health.load(Ordering::Acquire))
    }

    fn set_health(&self, health: LibraryChangeSourceHealth) {
        self.health.store(health_code(health), Ordering::Release);
    }

    fn raise_health(&self, health: LibraryChangeSourceHealth) {
        let next = health_code(health);
        let _ = self
            .health
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.max(next))
            });
    }

    fn compare_health(&self, current: LibraryChangeSourceHealth, next: LibraryChangeSourceHealth) {
        let _ = self.health.compare_exchange(
            health_code(current),
            health_code(next),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

fn validate_request(request: &LibraryChangeSourceRequest) -> Result<(), LibraryChangeSourceError> {
    if request.root_id.trim().is_empty() || request.root_id.contains('\0') {
        return Err(LibraryChangeSourceError::new(
            "change_source_root_id_invalid",
            "The library observer requires a non-empty root identifier.",
        ));
    }
    if request.ingress_capacity == 0 || request.ingress_capacity > MAX_INGRESS_CAPACITY {
        return Err(LibraryChangeSourceError::new(
            "change_source_capacity_invalid",
            "The library observer ingress capacity is outside the supported bound.",
        ));
    }
    if !request.root_path.is_absolute() || !request.root_path.is_dir() {
        return Err(LibraryChangeSourceError::retryable(
            "change_source_root_unavailable",
            "The library observer requires an available absolute directory root.",
        ));
    }
    Ok(())
}

fn observed_unix_ms() -> i64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn stop_report(started: Instant, state: &CallbackState) -> LibraryChangeSourceStopReport {
    LibraryChangeSourceStopReport {
        elapsed_millis: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        ignored_callback_count: state.ignored_callback_count.load(Ordering::Acquire),
    }
}

const fn health_code(health: LibraryChangeSourceHealth) -> u8 {
    match health {
        LibraryChangeSourceHealth::Healthy => 0,
        LibraryChangeSourceHealth::Starting => 1,
        LibraryChangeSourceHealth::Degraded => 2,
        LibraryChangeSourceHealth::Failed => 3,
        LibraryChangeSourceHealth::Stopped => 4,
        LibraryChangeSourceHealth::Unsupported => 5,
    }
}

const fn health_from_code(code: u8) -> LibraryChangeSourceHealth {
    match code {
        0 => LibraryChangeSourceHealth::Healthy,
        1 => LibraryChangeSourceHealth::Starting,
        2 => LibraryChangeSourceHealth::Degraded,
        3 => LibraryChangeSourceHealth::Failed,
        4 => LibraryChangeSourceHealth::Stopped,
        _ => LibraryChangeSourceHealth::Unsupported,
    }
}

#[cfg(test)]
mod tests;
