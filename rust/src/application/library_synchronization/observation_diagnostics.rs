#[cfg(debug_assertions)]
use std::io::Write;
#[cfg(any(debug_assertions, test))]
use std::time::{Duration, Instant};

#[cfg(test)]
pub(super) mod capture;

pub(super) fn measure_observation<T>(stage: &'static str, operation: impl FnOnce() -> T) -> T {
    #[cfg(any(debug_assertions, test))]
    let started = Instant::now();
    let result = operation();
    #[cfg(test)]
    capture::record(stage, started.elapsed());
    #[cfg(debug_assertions)]
    {
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(100) {
            let _ = writeln!(
                std::io::stderr().lock(),
                "[Ame sync observation] stage={stage} elapsed_ms={} thread={:?}",
                elapsed.as_millis(),
                std::thread::current().id(),
            );
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    result
}
