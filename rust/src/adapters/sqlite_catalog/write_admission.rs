use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::domain::LibraryChangeLane;

#[cfg(test)]
mod diagnostics;
#[cfg(test)]
mod tests;

const SQLITE_USER_INTERACTIVE_ADMISSION_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) type SqliteWritePreemptCallback = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
pub(super) struct SqliteWriteAdmissionState {
    pub(super) is_active: bool,
    pub(super) active_priority: usize,
    pub(super) active_preempt: Option<SqliteWritePreemptCallback>,
    pub(super) waiting: [VecDeque<Arc<()>>; 5],
    pub(super) completed_write_epoch: u64,
}

pub(super) struct SqliteWriteAdmission {
    pub(super) state: Mutex<SqliteWriteAdmissionState>,
    pub(super) checkpoint_policy: Arc<super::connection_policy::ImportCheckpointPolicy>,
    ready: Condvar,
}

impl SqliteWriteAdmission {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(SqliteWriteAdmissionState::default()),
            checkpoint_policy: Arc::default(),
            ready: Condvar::new(),
        }
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn acquire(self: &Arc<Self>, lane: LibraryChangeLane) -> SqliteWritePermit {
        self.acquire_priority(sqlite_write_priority(lane), None)
    }

    pub(super) fn reserve(self: &Arc<Self>, lane: LibraryChangeLane) -> ReservedWrite {
        let (pending, preempt) = self.register(sqlite_write_priority(lane));
        if let Some(preempt) = preempt {
            preempt();
        }
        #[cfg(test)]
        tests::after_registration();
        ReservedWrite { pending }
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn acquire_preemptible(
        self: &Arc<Self>,
        lane: LibraryChangeLane,
        preempt: SqliteWritePreemptCallback,
    ) -> SqliteWritePermit {
        self.acquire_priority(sqlite_write_priority(lane), Some(preempt))
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn acquire_user_interactive(self: &Arc<Self>) -> Option<SqliteWritePermit> {
        self.acquire_user_interactive_for(SQLITE_USER_INTERACTIVE_ADMISSION_TIMEOUT)
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn acquire_user_interactive_for(
        self: &Arc<Self>,
        timeout: Duration,
    ) -> Option<SqliteWritePermit> {
        self.acquire_priority_for(SQLITE_USER_INTERACTIVE_PRIORITY, timeout)
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn acquire_for(
        self: &Arc<Self>,
        lane: LibraryChangeLane,
        timeout: Duration,
    ) -> Option<SqliteWritePermit> {
        self.acquire_priority_for(sqlite_write_priority(lane), timeout)
    }

    #[cfg_attr(test, track_caller)]
    fn acquire_priority_for(
        self: &Arc<Self>,
        priority: usize,
        timeout: Duration,
    ) -> Option<SqliteWritePermit> {
        let deadline = Instant::now() + timeout;
        let (mut pending, preempt) = self.register(priority);
        if let Some(preempt) = preempt {
            preempt();
        }
        #[cfg(test)]
        tests::after_registration();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        loop {
            if pending.can_admit(&state) {
                return Some(pending.admit(&mut state, None));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(state);
                return None;
            }
            let (next, _) = self
                .ready
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
        }
    }

    #[cfg_attr(test, track_caller)]
    fn acquire_priority(
        self: &Arc<Self>,
        priority: usize,
        active_preempt: Option<SqliteWritePreemptCallback>,
    ) -> SqliteWritePermit {
        let (mut pending, preempt) = self.register(priority);
        if let Some(preempt) = preempt {
            preempt();
        }
        #[cfg(test)]
        tests::after_registration();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        while !pending.can_admit(&state) {
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }
        pending.admit(&mut state, active_preempt)
    }

    fn register(
        self: &Arc<Self>,
        priority: usize,
    ) -> (PendingWrite, Option<SqliteWritePreemptCallback>) {
        let token = Arc::new(());
        let pending = PendingWrite {
            admission: Arc::clone(self),
            priority,
            token: Some(Arc::clone(&token)),
        };
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.waiting[priority].push_back(token);
        let preempt = preemption_callback(&state, priority);
        (pending, preempt)
    }

    pub(super) fn completed_write_epoch(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .completed_write_epoch
    }

    pub(super) fn wait_for_completed_write_after(
        &self,
        observed_epoch: u64,
        timeout: Duration,
    ) -> u64 {
        let deadline = Instant::now() + timeout;
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        while state.completed_write_epoch == observed_epoch {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let (next, wait_result) = self
                .ready
                .wait_timeout(state, remaining)
                .unwrap_or_else(|error| error.into_inner());
            state = next;
            if wait_result.timed_out() {
                break;
            }
        }
        state.completed_write_epoch
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn try_acquire(
        self: &Arc<Self>,
        lane: LibraryChangeLane,
    ) -> Option<SqliteWritePermit> {
        self.try_acquire_priority(sqlite_write_priority(lane), None)
    }

    #[cfg_attr(test, track_caller)]
    pub(super) fn try_acquire_preemptible_maintenance(
        self: &Arc<Self>,
        preempt: SqliteWritePreemptCallback,
    ) -> Option<SqliteWritePermit> {
        self.try_acquire_priority(SQLITE_MAINTENANCE_PRIORITY, Some(preempt))
    }

    #[cfg_attr(test, track_caller)]
    fn try_acquire_priority(
        self: &Arc<Self>,
        priority: usize,
        preempt: Option<SqliteWritePreemptCallback>,
    ) -> Option<SqliteWritePermit> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if state.is_active
            || state.waiting[..=priority]
                .iter()
                .any(|queue| !queue.is_empty())
        {
            return None;
        }
        state.is_active = true;
        state.active_priority = priority;
        state.active_preempt = preempt;
        Some(SqliteWritePermit::new(Arc::clone(self), priority))
    }
}

struct PendingWrite {
    admission: Arc<SqliteWriteAdmission>,
    priority: usize,
    token: Option<Arc<()>>,
}

pub(super) struct ReservedWrite {
    pending: PendingWrite,
}

impl ReservedWrite {
    #[cfg_attr(test, track_caller)]
    pub(super) fn try_acquire(&mut self) -> Option<SqliteWritePermit> {
        let mut state = self
            .pending
            .admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.pending.can_admit(&state) {
            return None;
        }
        // The position survives a rolled-back attempt, without retaining an active permit.
        state.is_active = true;
        state.active_priority = self.pending.priority;
        state.active_preempt = None;
        Some(SqliteWritePermit::new(
            Arc::clone(&self.pending.admission),
            self.pending.priority,
        ))
    }
}

impl PendingWrite {
    fn can_admit(&self, state: &SqliteWriteAdmissionState) -> bool {
        !state.is_active
            && state.waiting[..self.priority]
                .iter()
                .all(VecDeque::is_empty)
            && self.token.as_ref().is_some_and(|token| {
                state.waiting[self.priority]
                    .front()
                    .is_some_and(|first| Arc::ptr_eq(first, token))
            })
    }

    #[cfg_attr(test, track_caller)]
    fn admit(
        &mut self,
        state: &mut SqliteWriteAdmissionState,
        preempt: Option<SqliteWritePreemptCallback>,
    ) -> SqliteWritePermit {
        state.waiting[self.priority].pop_front();
        self.token = None;
        state.is_active = true;
        state.active_priority = self.priority;
        state.active_preempt = preempt;
        SqliteWritePermit::new(Arc::clone(&self.admission), self.priority)
    }
}

impl Drop for PendingWrite {
    fn drop(&mut self) {
        let _timer =
            super::operation_diagnostics::OperationTimer::start("writer_reservation_retirement");
        let Some(token) = self.token.take() else {
            return;
        };
        let mut state = self
            .admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.waiting[self.priority].retain(|waiting| !Arc::ptr_eq(waiting, &token));
        drop(state);
        self.admission.ready.notify_all();
    }
}

pub(super) struct SqliteWritePermit {
    pub(super) admission: Arc<SqliteWriteAdmission>,
    #[cfg(test)]
    _observation: diagnostics::HeldWriteObservation,
}

impl SqliteWritePermit {
    #[cfg_attr(test, track_caller)]
    fn new(admission: Arc<SqliteWriteAdmission>, priority: usize) -> Self {
        #[cfg(not(test))]
        let _ = priority;
        Self {
            admission,
            #[cfg(test)]
            _observation: diagnostics::HeldWriteObservation::start(priority),
        }
    }
}

impl Drop for SqliteWritePermit {
    fn drop(&mut self) {
        let _timer =
            super::operation_diagnostics::OperationTimer::start("writer_permit_retirement");
        let mut state = self
            .admission
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.is_active = false;
        state.active_priority = 0;
        state.active_preempt = None;
        state.completed_write_epoch = state.completed_write_epoch.wrapping_add(1);
        drop(state);
        self.admission.ready.notify_all();
    }
}

pub(super) const SQLITE_USER_INTERACTIVE_PRIORITY: usize = 0;
pub(super) const SQLITE_MAINTENANCE_PRIORITY: usize = 4;

fn preemption_callback(
    state: &SqliteWriteAdmissionState,
    waiting_priority: usize,
) -> Option<SqliteWritePreemptCallback> {
    (state.is_active && waiting_priority < state.active_priority)
        .then(|| state.active_preempt.clone())
        .flatten()
}

pub(super) fn sqlite_write_priority(lane: LibraryChangeLane) -> usize {
    match lane {
        LibraryChangeLane::Live => 1,
        LibraryChangeLane::Journal => 2,
        LibraryChangeLane::Recovery => 3,
    }
}
