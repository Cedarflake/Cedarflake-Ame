use std::sync::Arc;

use super::*;

#[derive(Default)]
struct PackageState {
    damaged: AtomicBool,
    prepared: AtomicBool,
    applied: AtomicBool,
    downloads: AtomicU64,
}

struct Package(Arc<PackageState>);

impl ApplicationUpdatePackage for Package {
    fn release(&self) -> ApplicationRelease {
        ApplicationRelease {
            version: "0.2.0".into(),
            build: 5,
            download_bytes: 7,
            notes: String::new(),
        }
    }
    fn download(
        &self,
        observer: &dyn ApplicationDownloadObserver,
    ) -> Result<(), ApplicationUpdateError> {
        if !observer.cancelled() {
            self.0.downloads.fetch_add(1, Ordering::Relaxed);
            observer.received(7);
        }
        Ok(())
    }
    fn verify(&self, _: &dyn ApplicationDownloadObserver) -> Result<(), ApplicationUpdateError> {
        if self.0.damaged.load(Ordering::Relaxed) {
            return Err(ApplicationUpdateError::new(
                ApplicationUpdateFailureCode::DamagedDownload,
                "Changed package",
            ));
        }
        Ok(())
    }
    fn prepare_handoff(&self) -> Result<(), ApplicationUpdateError> {
        self.0.prepared.store(true, Ordering::Relaxed);
        Ok(())
    }
    fn apply_after_exit(&self) -> Result<(), ApplicationUpdateError> {
        self.0.applied.store(true, Ordering::Relaxed);
        Ok(())
    }
}

#[test]
fn cancellation_before_worker_start_is_retained_until_an_explicit_retry() {
    let state = Arc::new(PackageState::default());
    let operation = ApplicationUpdateDownload::new(Box::new(Package(state.clone())));
    operation.prepare_download().unwrap();
    operation.cancel();
    operation.download().unwrap();
    assert_eq!(
        operation.progress().phase,
        ApplicationDownloadPhase::Cancelled
    );
    assert_eq!(state.downloads.load(Ordering::Relaxed), 0);
    assert!(operation.prepare_handoff().is_err());
    assert!(operation.apply_after_exit().is_err());
    operation.prepare_download().unwrap();
    operation.download().unwrap();
    operation.prepare_handoff().unwrap();
    operation.apply_after_exit().unwrap();
    assert_eq!(state.downloads.load(Ordering::Relaxed), 1);
    assert!(state.prepared.load(Ordering::Relaxed) && state.applied.load(Ordering::Relaxed));
}

#[test]
fn failed_revalidation_revokes_previously_ready_handoff_authority() {
    let state = Arc::new(PackageState::default());
    let operation = ApplicationUpdateDownload::new(Box::new(Package(state.clone())));
    operation.prepare_download().unwrap();
    operation.download().unwrap();
    assert_eq!(operation.progress().phase, ApplicationDownloadPhase::Ready);
    state.damaged.store(true, Ordering::Relaxed);
    assert_eq!(
        operation.verify().unwrap_err().code,
        ApplicationUpdateFailureCode::DamagedDownload
    );
    assert!(operation.prepare_handoff().is_err());
    assert!(operation.apply_after_exit().is_err());
    assert!(!state.prepared.load(Ordering::Relaxed) && !state.applied.load(Ordering::Relaxed));
}
