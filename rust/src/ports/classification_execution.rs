use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::domain::ScanError;
use crate::domain::classification::work::{ClassificationWorkFailure, ClassificationWorkLease};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationExecutionOutcome {
    Completed,
    Cancelled,
    Superseded,
}

#[derive(Debug)]
pub enum ClassificationExecutionError {
    Work {
        kind: ClassificationWorkFailure,
        error: ScanError,
    },
    Blocked(ScanError),
}

pub trait ClassificationWorkExecutor {
    fn execute(
        &mut self,
        lease: &ClassificationWorkLease,
        now_unix_ms: u64,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationExecutionOutcome, ClassificationExecutionError>;

    fn retire(&mut self) -> Result<(), ScanError>;
    fn retire_if_idle(&mut self) -> Result<(), ScanError>;
    fn idle_delay(&self) -> Option<Duration>;
}
