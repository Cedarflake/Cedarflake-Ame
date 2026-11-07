use std::time::Duration;

const GIB: u64 = 1024 * 1024 * 1024;
const RECOVERY_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnalysisCapacity {
    pub processors: usize,
    pub physical_bytes: u64,
    pub available_bytes: u64,
    pub commit_headroom_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum AnalysisConcurrency {
    #[default]
    Serial,
    Parallel,
}

#[derive(Default)]
pub(crate) struct AnalysisCapacityPolicy {
    mode: AnalysisConcurrency,
    recovering: bool,
    healthy_since: Option<Duration>,
}

impl AnalysisCapacityPolicy {
    pub(crate) fn update(
        &mut self,
        capacity: Option<AnalysisCapacity>,
        gpu: bool,
        pressure: bool,
        now: Duration,
    ) -> AnalysisConcurrency {
        let eligible = capacity.filter(|capacity| {
            capacity.physical_bytes >= 12 * GIB && capacity.processors >= if gpu { 4 } else { 8 }
        });
        let Some(capacity) = eligible.filter(|_| !pressure) else {
            return self.serial();
        };
        let headroom = capacity.available_bytes.min(capacity.commit_headroom_bytes);
        if headroom < 3 * GIB / 2 {
            return self.serial();
        }
        if self.mode == AnalysisConcurrency::Parallel {
            return self.mode;
        }
        if headroom < 3 * GIB {
            self.healthy_since = None;
            return self.mode;
        }
        let healthy_since = *self.healthy_since.get_or_insert(now);
        if !self.recovering || now.saturating_sub(healthy_since) >= RECOVERY_INTERVAL {
            self.mode = AnalysisConcurrency::Parallel;
            self.recovering = false;
        }
        self.mode
    }

    fn serial(&mut self) -> AnalysisConcurrency {
        self.recovering |= self.mode == AnalysisConcurrency::Parallel;
        self.mode = AnalysisConcurrency::Serial;
        self.healthy_since = None;
        self.mode
    }
}

#[cfg(test)]
mod tests;
