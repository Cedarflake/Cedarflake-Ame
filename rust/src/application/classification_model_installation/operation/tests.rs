use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use crate::domain::classification::installation::ModelInstallationPhase;

use super::*;

#[test]
fn preparation_retains_ownership_until_cancelled_work_returns() {
    let operation = Arc::new(ModelInstallationOperation::new());
    let worker_operation = operation.clone();
    let (entered, entering) = mpsc::channel();
    let (retire, retiring) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        worker_operation.prepare_with(|cancelled, progress| {
            progress(ModelInstallationProgress {
                phase: ModelInstallationPhase::Downloading,
                completed_bytes: 2,
                total_bytes: 10,
            });
            entered.send(()).unwrap();
            retiring.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(cancelled.load(Ordering::Acquire));
            Err(ScanError::new("fixture_interrupted", "Transfer stopped"))
        })
    });
    entering.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(operation.progress().unwrap().unwrap().completed_bytes, 2);
    assert!(
        operation
            .prepare_with(|_, _| panic!("Duplicate preparation started"))
            .is_err()
    );
    assert!(operation.commit().is_err());
    operation.cancel().unwrap();
    assert!(operation.discard().is_err());
    assert!(matches!(
        *operation.state.lock().unwrap(),
        PreparationState::Preparing
    ));
    retire.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap().unwrap_err().code,
        "classification_cancelled"
    );
    assert!(matches!(
        *operation.state.lock().unwrap(),
        PreparationState::Closed
    ));
    assert!(operation.commit().is_err());
}

#[test]
fn shutdown_preserves_recovered_intent_and_explicit_cancel_retires_it_before_preparation() {
    use crate::adapters::DownloadIntent;

    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("installation.sqlite");
    let intent = DownloadIntent::authorize(&settings, "fixture").unwrap();
    let operation =
        ModelInstallationOperation::with_source(InstallationSource::Resume(PendingDownload {
            settings: settings.clone(),
            intent: intent.clone(),
        }));
    operation.cancel().unwrap();
    assert_eq!(
        DownloadIntent::pending(&settings, "fixture").unwrap(),
        Some(intent)
    );
    operation.discard().unwrap();
    assert!(
        DownloadIntent::pending(&settings, "fixture")
            .unwrap()
            .is_none()
    );
}

#[test]
fn cancelled_or_failed_operation_never_restarts_or_publishes() {
    let cancelled = ModelInstallationOperation::new();
    cancelled.cancel().unwrap();
    assert_eq!(
        cancelled
            .prepare_with(|_, _| panic!("Cancelled work started"))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    let failed = ModelInstallationOperation::new();
    assert_eq!(
        failed
            .prepare_with(|_, _| Err(ScanError::new("fixture_offline", "Offline")))
            .unwrap_err()
            .code,
        "fixture_offline"
    );
    assert!(
        failed
            .prepare_with(|_, _| panic!("Closed work restarted"))
            .is_err()
    );
    assert!(failed.commit().is_err());
}
