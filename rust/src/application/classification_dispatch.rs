use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{ClassificationDispatchMode, ClassificationWorkIssue};
use crate::ports::classification_execution::{
    ClassificationExecutionError, ClassificationExecutionOutcome, ClassificationWorkExecutor,
};
use crate::ports::classification_work::ClassificationWorkRepository;

mod signals;
use signals::Admission;
pub use signals::{
    ClassificationDispatchClock, ClassificationDispatchSignals, ClassificationForegroundState,
    MonotonicDispatchClock,
};

const FAILURE_RETRY: u64 = 5_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationDispatchStep {
    Progress,
    Waiting { wake_after: Option<Duration> },
    Closed,
}

pub struct ClassificationDispatcher<R, E, C = MonotonicDispatchClock> {
    repository: R,
    executor: E,
    session: ClassificationRecordId,
    signals: ClassificationDispatchSignals<C>,
    seeded_revision: u64,
}

impl<R: ClassificationWorkRepository, E: ClassificationWorkExecutor, C: ClassificationDispatchClock>
    ClassificationDispatcher<R, E, C>
{
    pub fn open(
        mut repository: R,
        executor: E,
        session: ClassificationRecordId,
        signals: ClassificationDispatchSignals<C>,
    ) -> Result<Self, ScanError> {
        let state = repository.classification_dispatch_state()?;
        repository.open_classification_session(state.revision, &session)?;
        Ok(Self {
            repository,
            executor,
            session,
            signals,
            seeded_revision: 0,
        })
    }

    /// Runs on the owning background thread; signals never wait for source or model work.
    pub fn run(&mut self) -> Result<(), ScanError> {
        let result = self.run_until_closed();
        self.executor.retire()?;
        result
    }

    pub fn retire(&mut self) -> Result<(), ScanError> {
        self.executor.retire()
    }

    fn run_until_closed(&mut self) -> Result<(), ScanError> {
        loop {
            let observed = self.signals.notification()?;
            match self.step()? {
                ClassificationDispatchStep::Progress => {}
                ClassificationDispatchStep::Waiting { wake_after } => {
                    self.signals.wait(observed, wake_after)?
                }
                ClassificationDispatchStep::Closed => return Ok(()),
            }
        }
    }

    pub fn step(&mut self) -> Result<ClassificationDispatchStep, ScanError> {
        let state = self.repository.classification_dispatch_state()?;
        if state.session_id.as_ref() != Some(&self.session) {
            self.executor.retire()?;
            return Err(ScanError::new(
                "classification_dispatch_changed",
                "The classification session changed",
            ));
        }
        self.executor.retire_if_idle()?;
        let active = match self
            .signals
            .admission(state.mode == ClassificationDispatchMode::Automatic)?
        {
            Admission::Closed => {
                self.executor.retire()?;
                return Ok(ClassificationDispatchStep::Closed);
            }
            Admission::Wait { delay, retire } => {
                if retire {
                    self.executor.retire()?;
                }
                return Ok(self.waiting(delay));
            }
            Admission::Ready(active) => active,
        };
        let now = self.signals.unix_ms();
        let mut more_sources = false;
        if self.seeded_revision != active.catalog_revision {
            let page = self
                .repository
                .seed_classification_backlog(&self.session, now)?;
            more_sources = page.has_more;
            if !more_sources {
                self.seeded_revision = active.catalog_revision;
            }
        }
        if active.cancelled.load(Ordering::Acquire) {
            self.executor.retire()?;
            return Ok(ClassificationDispatchStep::Progress);
        }
        let Some(lease) = self
            .repository
            .claim_classification_work(&self.session, now)?
        else {
            if more_sources {
                return Ok(ClassificationDispatchStep::Progress);
            }
            let next = self
                .repository
                .classification_work_ready_at(&self.session)?;
            self.signals.idle(active.catalog_revision)?;
            return Ok(
                self.waiting(next.map(|time| Duration::from_millis(time.saturating_sub(now))))
            );
        };
        let result = self.executor.execute(&lease, now, &active.cancelled);
        match result {
            Ok(ClassificationExecutionOutcome::Completed) => {}
            Ok(
                ClassificationExecutionOutcome::Cancelled
                | ClassificationExecutionOutcome::Superseded,
            ) => {
                self.repository.yield_classification_work(&lease)?;
            }
            Err(_) if active.cancelled.load(Ordering::Acquire) => {
                self.repository.yield_classification_work(&lease)?;
            }
            Err(ClassificationExecutionError::Work { kind, error }) => {
                self.repository.fail_classification_work(
                    &lease,
                    &ClassificationWorkIssue {
                        kind,
                        code: error.code,
                        retry_unix_ms: self.signals.unix_ms().saturating_add(FAILURE_RETRY),
                    },
                )?;
            }
            Err(ClassificationExecutionError::Blocked(error)) => {
                self.repository.yield_classification_work(&lease)?;
                self.executor.retire()?;
                return Err(error);
            }
        }
        if active.cancelled.load(Ordering::Acquire) {
            self.executor.retire()?;
        }
        Ok(ClassificationDispatchStep::Progress)
    }

    fn waiting(&self, delay: Option<Duration>) -> ClassificationDispatchStep {
        let wake_after = match (delay, self.executor.idle_delay()) {
            (Some(work), Some(retirement)) => Some(work.min(retirement)),
            (work, retirement) => work.or(retirement),
        };
        ClassificationDispatchStep::Waiting { wake_after }
    }
}
