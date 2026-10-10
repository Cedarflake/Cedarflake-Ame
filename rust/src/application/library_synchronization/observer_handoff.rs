use crate::domain::{
    LibraryChangeEnqueueReport, LibraryChangePlanningResult, LibraryChangeQueuePolicy, ScanError,
};
use crate::ports::LibraryChangeIngress;

use super::super::library_change_queue::validate_library_change_plan;

pub(super) struct ObserverHandoff<Reservation> {
    pending: Option<LibraryChangePlanningResult>,
    reservation: Option<Reservation>,
}

impl<Reservation> ObserverHandoff<Reservation> {
    pub(super) fn new() -> Self {
        Self {
            pending: None,
            reservation: None,
        }
    }

    pub(super) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn is_waiting_for_writer(&self) -> bool {
        self.reservation.is_some()
    }

    pub(super) fn retain(&mut self, plan: LibraryChangePlanningResult) -> Result<(), ScanError> {
        if self.has_pending() {
            return Err(ScanError::new(
                "change_ingress_handoff_occupied",
                "Uncommitted observation evidence cannot be replaced by another plan",
            ));
        }
        self.pending = Some(plan);
        Ok(())
    }

    pub(super) fn release_admission(&mut self) {
        self.reservation = None;
    }

    pub(super) fn persist<Queue: LibraryChangeIngress<Reservation = Reservation>>(
        &mut self,
        queue: &mut Queue,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<(), ScanError> {
        let Some(plan) = self.pending.as_ref() else {
            return Ok(());
        };
        if let Err(error) = validate_library_change_plan(plan, policy) {
            self.release_admission();
            return Err(error);
        }
        let count = plan.intents.len();
        if count == 0 {
            self.pending = None;
            return Ok(());
        }
        match self.attempt(queue, count, now_unix_ms, policy) {
            Ok(_) => {
                self.pending = None;
                Ok(())
            }
            Err(error) if error.code == "change_queue_backpressure" && count > 1 => {
                let mut prefix = (count / 2).max(1);
                loop {
                    match self.attempt(queue, prefix, now_unix_ms, policy) {
                        Ok(_) => {
                            self.pending
                                .as_mut()
                                .expect("retained plan")
                                .intents
                                .drain(..prefix);
                            return Err(error);
                        }
                        Err(error) if error.code == "change_queue_backpressure" && prefix > 1 => {
                            prefix = (prefix / 2).max(1);
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
            Err(error) => Err(error),
        }
    }

    fn attempt<Queue: LibraryChangeIngress<Reservation = Reservation>>(
        &mut self,
        queue: &mut Queue,
        count: usize,
        now_unix_ms: i64,
        policy: LibraryChangeQueuePolicy,
    ) -> Result<LibraryChangeEnqueueReport, ScanError> {
        let plan = self
            .pending
            .as_ref()
            .expect("pending plan owns the attempt");
        let intents = &plan.intents[..count];
        if self.reservation.is_none() {
            self.reservation = Some(queue.reserve_change_ingress(intents)?);
        }
        let result = queue.try_enqueue_reserved_changes(
            self.reservation.as_mut().expect("reserved ingress"),
            intents,
            now_unix_ms,
            policy,
        );
        if !result
            .as_ref()
            .is_err_and(|error| super::is_transient_persistence_contention(&error.code))
        {
            self.release_admission();
        }
        result
    }
}
