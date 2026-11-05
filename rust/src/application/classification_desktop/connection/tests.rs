use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::ports::classification_work::ClassificationWorkRepository;

use super::super::open_with_storage;
use super::super::tests::{install_fixture, paused_catalog, storage};
use super::*;

#[test]
fn reconnect_preserves_execution_and_retires_obsolete_observers_without_closing_the_runtime() {
    let directory = tempfile::tempdir().unwrap();
    let storage = storage(directory.path());
    let catalog = paused_catalog(&storage);
    install_fixture(&storage, directory.path());
    let executable = directory.path().join("app.exe");
    let original = Arc::new(
        open_with_storage(&storage, executable.clone())
            .unwrap()
            .unwrap(),
    );
    let identity = catalog.classification_dispatch_state().unwrap().session_id;
    let executing = original.clone();
    let execution = std::thread::spawn(move || executing.run());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !original.connection.lock().unwrap().running {
        assert!(
            Instant::now() < deadline,
            "the original execution did not start"
        );
        std::thread::yield_now();
    }

    let observer = Arc::new(
        open_with_storage(&storage, executable.clone())
            .unwrap()
            .unwrap(),
    );
    assert!(Arc::ptr_eq(&original.connection, &observer.connection));
    original.close().unwrap();
    assert!(!observer.connection.control.signals().is_closed().unwrap());
    let waiting = observer.clone();
    let (finished, result) = mpsc::channel();
    let observation = std::thread::spawn(move || finished.send(waiting.run().map(|v| v.is_none())));

    let current = open_with_storage(&storage, executable.clone())
        .unwrap()
        .unwrap();
    assert!(
        result
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap()
    );
    observation.join().unwrap().unwrap();
    observer.request_close().unwrap();
    drop(observer);
    assert!(!current.connection.control.signals().is_closed().unwrap());
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().session_id,
        identity
    );
    assert!(!execution.is_finished());

    current.request_close().unwrap();
    assert!(current.run().unwrap().is_none());
    assert!(execution.join().unwrap().unwrap().is_none());
    current.close().unwrap();
    drop(current);
    drop(original);
    let replacement = open_with_storage(&storage, executable).unwrap().unwrap();
    assert_ne!(
        catalog.classification_dispatch_state().unwrap().session_id,
        identity
    );
    replacement.close().unwrap();
}
