#[cfg(debug_assertions)]
use std::time::{Duration, Instant};

pub(super) fn measure_observation<T>(stage: &'static str, operation: impl FnOnce() -> T) -> T {
    #[cfg(debug_assertions)]
    let started = Instant::now();
    let result = operation();
    #[cfg(debug_assertions)]
    {
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(100) {
            eprintln!(
                "[Ame sync observation] stage={stage} elapsed_ms={}",
                elapsed.as_millis(),
            );
        }
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    result
}
