use std::sync::{Arc, atomic::AtomicBool};

use super::*;

pub enum ClassificationMaintenanceStep<T> {
    Idle,
    Progress,
    Prepared(T),
}

pub trait ClassificationMaintenance<R, E> {
    type Prepared;

    fn prepare(
        &mut self,
        repository: &R,
        executor: &mut E,
        catalog_notification: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<ClassificationMaintenanceStep<Self::Prepared>, ScanError>;

    fn release_resources(&mut self) {}
}

pub enum ClassificationDispatchEvent<T> {
    Step(ClassificationDispatchStep),
    Maintenance(T),
}

impl<R: ClassificationWorkRepository, E: ClassificationWorkExecutor, C: ClassificationDispatchClock>
    ClassificationDispatcher<R, E, C>
{
    pub fn run_with_maintenance<M: ClassificationMaintenance<R, E>>(
        &mut self,
        maintenance: &mut M,
    ) -> Result<Option<M::Prepared>, ScanError> {
        let result = self.run_until_maintenance(maintenance);
        maintenance.release_resources();
        self.executor.retire()?;
        result
    }

    fn run_until_maintenance<M: ClassificationMaintenance<R, E>>(
        &mut self,
        maintenance: &mut M,
    ) -> Result<Option<M::Prepared>, ScanError> {
        loop {
            let observed = self.signals.notification()?;
            match self.step_with_maintenance(maintenance)? {
                ClassificationDispatchEvent::Step(ClassificationDispatchStep::Progress) => {}
                ClassificationDispatchEvent::Step(ClassificationDispatchStep::Waiting {
                    wake_after,
                }) => {
                    self.signals.wait(observed, wake_after)?;
                }
                ClassificationDispatchEvent::Step(ClassificationDispatchStep::Closed) => {
                    return Ok(None);
                }
                ClassificationDispatchEvent::Maintenance(plan) => return Ok(Some(plan)),
            }
        }
    }

    pub fn step_with_maintenance<M: ClassificationMaintenance<R, E>>(
        &mut self,
        maintenance: &mut M,
    ) -> Result<ClassificationDispatchEvent<M::Prepared>, ScanError> {
        let step = self.step()?;
        if step == ClassificationDispatchStep::Closed {
            maintenance.release_resources();
        }
        if !matches!(step, ClassificationDispatchStep::Waiting { .. }) {
            return Ok(ClassificationDispatchEvent::Step(step));
        }
        let state = self.repository.classification_dispatch_state()?;
        if state.session_id.as_ref() != Some(&self.session) {
            return Err(ScanError::new(
                "classification_dispatch_changed",
                "The classification session changed",
            ));
        }
        let active = match self
            .signals
            .admission(state.mode == ClassificationDispatchMode::Automatic)?
        {
            Admission::Ready(active) => active,
            Admission::Closed => {
                maintenance.release_resources();
                self.executor.retire()?;
                return Ok(ClassificationDispatchEvent::Step(
                    ClassificationDispatchStep::Closed,
                ));
            }
            Admission::Wait { retire, delay } => {
                if retire {
                    maintenance.release_resources();
                    self.executor.retire()?;
                }
                return Ok(ClassificationDispatchEvent::Step(self.waiting(delay)));
            }
        };
        let ready = self
            .repository
            .classification_work_ready_at(&self.session)?;
        if self.seeded_revision != active.catalog_revision
            || ready.is_some_and(|time| time <= self.signals.unix_ms())
        {
            return Ok(ClassificationDispatchEvent::Step(
                ClassificationDispatchStep::Progress,
            ));
        }
        let result = maintenance.prepare(
            &self.repository,
            &mut self.executor,
            active.catalog_revision,
            &active.cancelled,
        );
        if active.cancelled.load(Ordering::Acquire)
            || self.repository.classification_dispatch_state()? != state
        {
            self.executor.retire()?;
            return Ok(ClassificationDispatchEvent::Step(
                ClassificationDispatchStep::Progress,
            ));
        }
        match result? {
            ClassificationMaintenanceStep::Prepared(plan) => {
                maintenance.release_resources();
                self.executor.retire()?;
                Ok(ClassificationDispatchEvent::Maintenance(plan))
            }
            ClassificationMaintenanceStep::Progress => Ok(ClassificationDispatchEvent::Step(
                ClassificationDispatchStep::Progress,
            )),
            ClassificationMaintenanceStep::Idle => {
                let now = self.signals.unix_ms();
                let next = self
                    .repository
                    .classification_work_ready_at(&self.session)?;
                self.signals.idle(active.catalog_revision)?;
                Ok(ClassificationDispatchEvent::Step(self.waiting(next.map(
                    |time| Duration::from_millis(time.saturating_sub(now)),
                ))))
            }
        }
    }
}
