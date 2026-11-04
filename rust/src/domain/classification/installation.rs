#[cfg(any(all(windows, feature = "classification-worker"), test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClassificationInstallation {
    pub models: std::path::PathBuf,
    pub runtime: std::path::PathBuf,
}

#[cfg(any(all(windows, feature = "classification-worker"), test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadIntent {
    pub generation: i64,
    pub package: String,
}

#[cfg(all(windows, feature = "classification-worker"))]
pub(crate) fn check_cancelled(
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<(), crate::domain::ScanError> {
    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
        Err(crate::domain::ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelInstallationPhase {
    Verifying,
    Downloading,
    Prepared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelInstallationProgress {
    pub phase: ModelInstallationPhase,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}
