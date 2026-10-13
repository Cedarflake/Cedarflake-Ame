pub(super) struct OperationTimer {
    #[cfg(test)]
    started: std::time::Instant,
    #[cfg(test)]
    stage: &'static str,
}

impl OperationTimer {
    pub(super) fn start(stage: &'static str) -> Self {
        #[cfg(not(test))]
        let _ = stage;
        Self {
            #[cfg(test)]
            started: std::time::Instant::now(),
            #[cfg(test)]
            stage,
        }
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        #[cfg(test)]
        {
            use std::io::Write;

            let elapsed = self.started.elapsed();
            if elapsed >= std::time::Duration::from_millis(100) {
                // Preserve slow successful operations in hosted output, including unwinding.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "[Ame catalog operation] stage={} elapsed_ms={} thread={:?}",
                    self.stage,
                    elapsed.as_millis(),
                    std::thread::current().id(),
                );
            }
        }
    }
}

pub(super) fn measure<T>(stage: &'static str, operation: impl FnOnce() -> T) -> T {
    let _timer = OperationTimer::start(stage);
    operation()
}
