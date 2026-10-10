use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Read-only cancellation for one cleanup attempt; storage never owns runtime shutdown.
#[derive(Clone, Default)]
pub struct InventoryCleanupControl {
    cancelled: Arc<AtomicBool>,
}

impl InventoryCleanupControl {
    pub(crate) fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self { cancelled }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
