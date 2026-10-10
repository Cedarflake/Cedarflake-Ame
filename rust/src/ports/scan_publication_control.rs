use std::sync::Arc;

/// Read-only interruption evidence; command meanings remain with the scan application.
#[derive(Clone)]
pub(crate) struct ScanPublicationControl {
    requested: Arc<dyn Fn() -> bool + Send + Sync>,
}

impl ScanPublicationControl {
    pub(crate) fn new(requested: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            requested: Arc::new(requested),
        }
    }

    pub(crate) fn is_requested(&self) -> bool {
        (self.requested)()
    }
}

impl Default for ScanPublicationControl {
    fn default() -> Self {
        Self::new(|| false)
    }
}
