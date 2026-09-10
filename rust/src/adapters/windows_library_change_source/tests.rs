use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel, sync_channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use notify::event::{CreateKind, Flag, ModifyKind, RemoveKind, RenameMode};
use notify::{Event, EventKind, Watcher};
use tempfile::TempDir;

use crate::application::plan_library_changes;
use crate::domain::{
    CatalogFreshnessState, LibraryChangeObservation, LibraryChangeObservationKind,
    LibraryChangeOrigin, LibraryChangePlanningContext, LibraryChangePlanningLimits,
    LibraryChangeScope, LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootGeneration,
};
use crate::ports::{LibraryChangeSource, LibraryChangeSourceRequest};

use super::{
    CallbackProcessor, CallbackState, RENAME_PAIR_GRACE, health_code, notify_start_issue_code,
    notify_watch_issue_code, start_windows_library_change_source,
};

#[test]
fn paired_and_split_rename_events_preserve_old_and_new_paths() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 8);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::Both,
    )))
    .add_path(root.path().join("old.jpg"))
    .add_path(root.path().join("new.jpg"))));
    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::From,
    )))
    .add_path(root.path().join("before.jpg"))));
    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::To,
    )))
    .add_path(root.path().join("after.jpg"))));

    let observations = drain_receiver(&receiver);
    assert_eq!(observations.len(), 2);
    assert!(observations.iter().all(|observation| {
        observation.kind
            == LibraryChangeObservationKind::Renamed {
                is_reliably_paired: true,
            }
    }));
    assert_eq!(
        observations[0].previous_relative_path.as_deref(),
        Some("old.jpg")
    );
    assert_eq!(observations[0].relative_path, "new.jpg");
    assert_eq!(
        observations[1].previous_relative_path.as_deref(),
        Some("before.jpg")
    );
    assert_eq!(observations[1].relative_path, "after.jpg");
}

#[test]
fn rename_halves_outside_the_grace_window_cannot_claim_reliable_pairing() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 8);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::From,
    )))
    .add_path(root.path().join("old.jpg"))));
    thread::sleep(RENAME_PAIR_GRACE + Duration::from_millis(10));
    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::To,
    )))
    .add_path(root.path().join("new.jpg"))));

    let observations = drain_receiver(&receiver);
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].kind, LibraryChangeObservationKind::Created);
    assert_eq!(observations[0].relative_path, "new.jpg");
    assert!(observations[0].previous_relative_path.is_none());
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Healthy);
}

#[test]
fn recoverable_evidence_gaps_keep_transport_healthy_until_callback_failure() {
    let root = TempDir::new().expect("temporary root");
    let (state, _) = callback_state(root.path(), 8);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Name(
        RenameMode::From,
    )))
    .add_path(root.path().join("old.jpg"))));
    thread::sleep(RENAME_PAIR_GRACE + Duration::from_millis(10));
    state.flush_expired_rename();
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Healthy);
    assert_eq!(
        state.take_issue_code().as_deref(),
        Some("change_source_rename_incomplete")
    );

    state.evidence_gap.store(false, Ordering::Release);
    processor.handle(Ok(Event::new(EventKind::Other).set_flag(Flag::Rescan)));
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Healthy);
    assert_eq!(
        state.take_issue_code().as_deref(),
        Some("change_source_rescan_required")
    );

    state.evidence_gap.store(false, Ordering::Release);
    processor.handle(Err(notify::Error::generic("forced callback failure")));
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Failed);
    assert_eq!(
        state.take_issue_code().as_deref(),
        Some("change_source_callback_failed")
    );

    state.evidence_gap.store(false, Ordering::Release);
    processor.handle(Err(notify::Error::io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "forced access failure",
    ))));
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(
        state.take_issue_code().as_deref(),
        Some("change_source_callback_access_denied")
    );
    let startup_error = notify::Error::io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "forced startup access failure",
    ));
    assert_eq!(
        notify_start_issue_code(&startup_error),
        "change_source_start_access_denied"
    );
    assert_eq!(
        notify_watch_issue_code(&startup_error),
        "change_source_watch_access_denied"
    );
}

#[test]
fn native_windows_buffer_overflow_emits_a_rescan_signal() {
    let root = TempDir::new().expect("temporary root");
    let callback_gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let callback_gate_state = Arc::clone(&callback_gate);
    let should_block = Arc::new(AtomicBool::new(true));
    let should_block_callback = Arc::clone(&should_block);
    let (sender, receiver) = channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        if should_block_callback.swap(false, Ordering::AcqRel) {
            let (lock, condition) = &*callback_gate_state;
            let mut state = lock.lock().expect("callback gate");
            state.0 = true;
            condition.notify_all();
            while !state.1 {
                state = condition.wait(state).expect("callback release");
            }
        }
        let _ = sender.send(result);
    })
    .expect("create native watcher");
    watcher
        .watch(root.path(), notify::RecursiveMode::Recursive)
        .expect("watch native overflow root");
    fs::write(root.path().join("trigger.jpg"), b"trigger").expect("trigger callback");

    let (lock, condition) = &*callback_gate;
    let mut state = lock.lock().expect("main gate");
    while !state.0 {
        state = condition.wait(state).expect("callback start");
    }
    for index in 0..512 {
        let name = format!("{index:04}-{}.jpg", "overflow-evidence-".repeat(10));
        fs::write(root.path().join(name), b"").expect("create overflow entry");
    }
    state.1 = true;
    condition.notify_all();
    drop(state);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_rescan = false;
    while Instant::now() < deadline {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) if event.need_rescan() => {
                saw_rescan = true;
                break;
            }
            Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert!(saw_rescan, "native overflow must emit a rescan signal");
}

#[test]
fn callback_ingress_is_bounded_and_overflow_becomes_an_evidence_gap() {
    let root = TempDir::new().expect("temporary root");
    let (state, _receiver) = callback_state(root.path(), 1);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(
        Event::new(EventKind::Create(CreateKind::File)).add_path(root.path().join("one.jpg"))
    ));
    processor.handle(Ok(
        Event::new(EventKind::Create(CreateKind::File)).add_path(root.path().join("two.jpg"))
    ));

    assert_eq!(state.dropped_observation_count.load(Ordering::Acquire), 1);
    assert!(state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Healthy);
}

#[test]
fn ambiguous_directory_and_removal_events_keep_subtree_scope() {
    let root = TempDir::new().expect("temporary root");
    let album = root.path().join("album");
    fs::create_dir(&album).expect("create album");
    let (state, receiver) = callback_state(root.path(), 8);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(
        Event::new(EventKind::Create(CreateKind::Any)).add_path(album.clone())
    ));
    processor.handle(Ok(
        Event::new(EventKind::Remove(RemoveKind::Any)).add_path(album)
    ));

    let observations = drain_receiver(&receiver);
    assert_eq!(observations.len(), 2);
    assert!(observations.iter().all(|observation| {
        observation.kind == LibraryChangeObservationKind::DirectoryChanged
            && observation.scope == crate::domain::LibraryChangeScope::Subtree
    }));
}

#[test]
fn failed_health_cannot_be_downgraded_by_a_later_rescan_signal() {
    let root = TempDir::new().expect("temporary root");
    let (state, _) = callback_state(root.path(), 4);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Err(notify::Error::generic("forced callback failure")));
    processor.handle(Ok(Event::new(EventKind::Other).set_flag(Flag::Rescan)));

    assert_eq!(state.health(), LibraryChangeSourceHealth::Failed);
    assert_eq!(
        state.take_issue_code().as_deref(),
        Some("change_source_callback_failed")
    );
}

#[test]
fn stopped_callback_is_ignored_without_reopening_ingress() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 4);
    state.accepting.store(false, Ordering::Release);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(
        Event::new(EventKind::Create(CreateKind::File)).add_path(root.path().join("late.jpg"))
    ));

    assert!(drain_receiver(&receiver).is_empty());
    assert_eq!(state.ignored_callback_count.load(Ordering::Acquire), 1);
}

#[test]
fn stop_gate_is_rechecked_at_the_bounded_channel_boundary() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 4);
    state.accepting.store(false, Ordering::Release);

    state.emit(LibraryChangeObservation {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        sequence: 1,
        observed_unix_ms: 1,
        kind: LibraryChangeObservationKind::Modified,
        scope: LibraryChangeScope::Path,
        relative_path: "late.jpg".to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
    });

    assert!(drain_receiver(&receiver).is_empty());
    assert_eq!(state.ignored_callback_count.load(Ordering::Acquire), 1);
}

#[test]
fn vanished_metadata_keeps_conservative_subtree_work_without_degrading_transport() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 4);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };

    processor.handle(Ok(Event::new(EventKind::Modify(ModifyKind::Any))
        .add_path(root.path().join("already-gone.jpg"))));

    let observations = drain_receiver(&receiver);
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].relative_path, "already-gone.jpg");
    assert_eq!(
        observations[0].kind,
        LibraryChangeObservationKind::DirectoryChanged
    );
    assert_eq!(observations[0].scope, LibraryChangeScope::Subtree);
    assert!(!state.evidence_gap.load(Ordering::Acquire));
    assert_eq!(state.health(), LibraryChangeSourceHealth::Healthy);
}

#[test]
fn controlled_windows_changes_produce_ame_intents_and_bounded_shutdown() {
    let root = TempDir::new().expect("temporary root");
    let sentinel_path = root.path().join("sentinel.bin");
    fs::write(&sentinel_path, b"unchanged-source-bytes").expect("write sentinel");
    let request = LibraryChangeSourceRequest {
        root_id: "fixture-root".to_owned(),
        root_generation: LibraryRootGeneration::new(9).expect("generation"),
        root_path: root.path().to_path_buf(),
        ingress_capacity: 64,
    };
    let mut source = start_windows_library_change_source(&request).expect("start watcher");
    let album = root.path().join("中文相册");
    fs::create_dir(&album).expect("create album");
    let old_path = album.join("旧照片.jpg");
    fs::write(&old_path, b"first").expect("create file");
    fs::write(&old_path, b"second").expect("modify file");
    let new_path = album.join("新照片.jpg");
    fs::rename(&old_path, &new_path).expect("rename file");
    fs::remove_file(&new_path).expect("remove file");

    let observations = wait_for_observations(&mut source, |observations| {
        observations.iter().any(|observation| {
            observation.relative_path == "中文相册/新照片.jpg"
                && matches!(
                    observation.kind,
                    LibraryChangeObservationKind::Renamed { .. }
                        | LibraryChangeObservationKind::Removed
                )
        })
    });
    assert!(!observations.is_empty());
    assert!(observations.iter().all(|observation| {
        observation.root_id == "fixture-root"
            && observation.root_generation == request.root_generation
            && !Path::new(&observation.relative_path).is_absolute()
    }));
    assert!(observations.iter().any(|observation| {
        observation.relative_path.starts_with("中文相册")
            && matches!(
                observation.kind,
                LibraryChangeObservationKind::Created
                    | LibraryChangeObservationKind::Modified
                    | LibraryChangeObservationKind::DirectoryChanged
            )
    }));
    assert!(observations.iter().any(|observation| {
        observation.relative_path == "中文相册/新照片.jpg"
            && matches!(
                observation.kind,
                LibraryChangeObservationKind::Renamed { .. }
                    | LibraryChangeObservationKind::Removed
            )
    }));

    let planning = plan_library_changes(
        &LibraryChangePlanningContext {
            root_id: request.root_id.clone(),
            root_generation: request.root_generation,
            availability: LibraryRootAvailability::Available,
            source_health: source.health(),
        },
        observations,
        LibraryChangePlanningLimits {
            max_observations: 64,
            max_intents: 64,
        },
    )
    .expect("plan observations");
    assert!(matches!(
        planning.freshness,
        CatalogFreshnessState::Updating | CatalogFreshnessState::NeedsReconciliation
    ));
    assert!(!planning.intents.is_empty());
    assert_eq!(
        fs::read(&sentinel_path).expect("read sentinel"),
        b"unchanged-source-bytes"
    );

    let stop_started = Instant::now();
    let report = source.stop().expect("stop watcher");
    assert!(stop_started.elapsed() < Duration::from_secs(2));
    assert!(report.elapsed_millis < 2_000);
    assert_eq!(source.health(), LibraryChangeSourceHealth::Stopped);
}

#[test]
fn configured_root_removal_and_window_close_stop_without_hanging() {
    let parent = TempDir::new().expect("temporary parent");
    let root_path = parent.path().join("removable-root");
    fs::create_dir(&root_path).expect("create root");
    let request = LibraryChangeSourceRequest {
        root_id: "removable-root".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        root_path: root_path.clone(),
        ingress_capacity: 16,
    };
    let mut source = start_windows_library_change_source(&request).expect("start watcher");

    fs::remove_dir(&root_path).expect("remove configured root");
    let observations = wait_for_observations(&mut source, |observations| {
        observations
            .iter()
            .any(|observation| observation.kind == LibraryChangeObservationKind::EvidenceGap)
    });

    assert!(observations.iter().any(|observation| {
        observation.kind == LibraryChangeObservationKind::EvidenceGap
            && observation.scope == LibraryChangeScope::Root
    }));
    assert_eq!(source.health(), LibraryChangeSourceHealth::Failed);
    let started = Instant::now();
    let report = source.stop().expect("stop removed root watcher");

    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(report.elapsed_millis < 2_000);
    assert_eq!(source.health(), LibraryChangeSourceHealth::Stopped);
}

#[test]
fn long_windows_relative_paths_remain_relative_and_lossless() {
    let root = TempDir::new().expect("temporary root");
    let (state, receiver) = callback_state(root.path(), 4);
    let mut processor = CallbackProcessor {
        state: Arc::clone(&state),
    };
    let relative_path = format!("相册/{}/照片.jpg", "很长".repeat(80));
    processor.handle(Ok(Event::new(EventKind::Create(CreateKind::File))
        .add_path(root.path().join(relative_path.replace('/', "\\")))));

    let observations = drain_receiver(&receiver);

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].relative_path, relative_path);
    assert!(!Path::new(&observations[0].relative_path).is_absolute());
}

#[test]
fn unavailable_or_unbounded_roots_are_rejected_before_watcher_creation() {
    let missing = PathBuf::from(r"C:\ame-r2c-b-missing-root");
    let request = LibraryChangeSourceRequest {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        root_path: missing,
        ingress_capacity: 16,
    };
    let error = match start_windows_library_change_source(&request) {
        Ok(_) => panic!("missing root must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, "change_source_root_unavailable");
    assert!(error.is_retryable);

    let root = TempDir::new().expect("temporary root");
    let unbounded = LibraryChangeSourceRequest {
        root_path: root.path().to_path_buf(),
        ingress_capacity: 4097,
        ..request
    };
    let error = match start_windows_library_change_source(&unbounded) {
        Ok(_) => panic!("unbounded ingress must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.code, "change_source_capacity_invalid");
}

fn callback_state(
    root: &Path,
    capacity: usize,
) -> (Arc<CallbackState>, Receiver<LibraryChangeObservation>) {
    let (sender, receiver) = sync_channel(capacity);
    (
        Arc::new(CallbackState {
            root_id: "root-a".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            root_path: root.to_path_buf(),
            sender,
            delivery_gate: Mutex::new(()),
            accepting: std::sync::atomic::AtomicBool::new(true),
            health: std::sync::atomic::AtomicU8::new(health_code(
                LibraryChangeSourceHealth::Healthy,
            )),
            evidence_gap: std::sync::atomic::AtomicBool::new(false),
            dropped_observation_count: std::sync::atomic::AtomicU64::new(0),
            ignored_callback_count: std::sync::atomic::AtomicU64::new(0),
            next_sequence: std::sync::atomic::AtomicU64::new(1),
            pending_rename_from: Mutex::new(None),
            last_issue: Mutex::new(None),
        }),
        receiver,
    )
}

fn drain_receiver(receiver: &Receiver<LibraryChangeObservation>) -> Vec<LibraryChangeObservation> {
    let mut observations = Vec::new();
    while let Ok(observation) = receiver.try_recv() {
        observations.push(observation);
    }
    observations
}

fn wait_for_observations(
    source: &mut super::WindowsLibraryChangeSource,
    predicate: impl Fn(&[LibraryChangeObservation]) -> bool,
) -> Vec<LibraryChangeObservation> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut observations = Vec::new();
    while Instant::now() < deadline {
        let batch = source.drain(64).expect("drain watcher");
        observations.extend(batch.observations);
        if predicate(&observations) {
            return observations;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for controlled filesystem observations: {observations:?}");
}
