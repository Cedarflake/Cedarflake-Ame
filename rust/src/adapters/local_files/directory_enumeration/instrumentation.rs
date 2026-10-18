use std::collections::HashMap;
use std::sync::{Arc, Condvar, LazyLock, Mutex};

pub(super) static SOURCE_ENUMERATION_COUNTS: LazyLock<
    Mutex<HashMap<String, SourceEnumerationCounts>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Default)]
pub(super) struct SourceEnumerationCounts {
    pub(super) entry_reads: u64,
    pub(super) directory_opens: u64,
    pub(super) peak_staged_window: usize,
}

static SOURCE_ENUMERATION_GATES: LazyLock<Mutex<HashMap<String, Arc<SourceEnumerationGateState>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct SourceEnumerationGateState {
    control: Mutex<SourceEnumerationGateControl>,
    changed: Condvar,
}

struct SourceEnumerationGateControl {
    permits: usize,
    waiting: usize,
    released: bool,
}

pub(crate) struct SourceEnumerationGateGuard {
    root_path: String,
    state: Arc<SourceEnumerationGateState>,
}

impl SourceEnumerationGateGuard {
    pub(crate) fn allow_entries(&self, count: usize) {
        let mut control = self.state.control.lock().expect("source enumeration gate");
        control.permits = control.permits.saturating_add(count);
        self.state.changed.notify_all();
    }

    pub(crate) fn wait_until_blocked(&self, timeout: std::time::Duration) -> bool {
        let control = self.state.control.lock().expect("source enumeration gate");
        let (control, _) = self
            .state
            .changed
            .wait_timeout_while(control, timeout, |control| {
                control.waiting == 0 && !control.released
            })
            .expect("source enumeration gate wait");
        control.waiting > 0
    }

    pub(crate) fn release(&self) {
        let mut control = self.state.control.lock().expect("source enumeration gate");
        control.released = true;
        self.state.changed.notify_all();
    }
}

impl Drop for SourceEnumerationGateGuard {
    fn drop(&mut self) {
        self.release();
        let mut gates = SOURCE_ENUMERATION_GATES
            .lock()
            .expect("source enumeration gates");
        if gates
            .get(&self.root_path)
            .is_some_and(|state| Arc::ptr_eq(state, &self.state))
        {
            gates.remove(&self.root_path);
        }
    }
}

pub(crate) fn gate_source_enumeration(root_path: &str) -> SourceEnumerationGateGuard {
    let state = Arc::new(SourceEnumerationGateState {
        control: Mutex::new(SourceEnumerationGateControl {
            permits: 0,
            waiting: 0,
            released: false,
        }),
        changed: Condvar::new(),
    });
    SOURCE_ENUMERATION_GATES
        .lock()
        .expect("source enumeration gates")
        .insert(root_path.to_owned(), Arc::clone(&state));
    SourceEnumerationGateGuard {
        root_path: root_path.to_owned(),
        state,
    }
}

pub(super) fn wait_for_source_enumeration_permit(root_path: &str) {
    let state = SOURCE_ENUMERATION_GATES
        .lock()
        .expect("source enumeration gates")
        .get(root_path)
        .cloned();
    let Some(state) = state else {
        return;
    };
    let mut control = state.control.lock().expect("source enumeration gate");
    control.waiting = control.waiting.saturating_add(1);
    state.changed.notify_all();
    while control.permits == 0 && !control.released {
        control = state
            .changed
            .wait(control)
            .expect("source enumeration gate wait");
    }
    control.waiting = control.waiting.saturating_sub(1);
    if !control.released {
        control.permits = control.permits.saturating_sub(1);
    }
}

pub(crate) fn reset_source_enumeration_instrumentation(root_path: &str) {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .insert(root_path.to_owned(), SourceEnumerationCounts::default());
}

pub(crate) fn source_entry_read_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.entry_reads)
}

pub(crate) fn source_directory_open_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.directory_opens)
}

pub(crate) fn source_peak_staged_window(root_path: &str) -> usize {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get(root_path)
        .map_or(0, |counts| counts.peak_staged_window)
}

pub(crate) fn record_source_peak_staged_window(root_path: &str, staged: usize) {
    if let Some(counts) = SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source enumeration instrumentation")
        .get_mut(root_path)
    {
        counts.peak_staged_window = counts.peak_staged_window.max(staged);
    }
}
