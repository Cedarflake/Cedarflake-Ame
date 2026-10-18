use std::io::Write;
use std::panic::Location;
use std::time::{Duration, Instant};

pub(super) struct HeldWriteObservation {
    started: Instant,
    priority: usize,
    caller: &'static Location<'static>,
}

impl HeldWriteObservation {
    #[track_caller]
    pub(super) fn start(priority: usize) -> Self {
        Self {
            started: Instant::now(),
            priority,
            caller: Location::caller(),
        }
    }
}

impl Drop for HeldWriteObservation {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        if elapsed < Duration::from_millis(100) {
            return;
        }
        // The permit releases its admission mutex before this field is dropped.
        let _ = writeln!(
            std::io::stderr().lock(),
            "[Ame held writer] priority={} elapsed_ms={} caller={} thread={:?}",
            self.priority,
            elapsed.as_millis(),
            self.caller,
            std::thread::current().id(),
        );
    }
}
