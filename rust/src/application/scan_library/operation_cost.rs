use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Operation {
    SourceDiscovery,
    PriorSelection,
    MediaInspection,
    LocationStaging,
    DirectoryPersistence,
    CheckpointPersistence,
}

const OPERATIONS: [Operation; 6] = [
    Operation::SourceDiscovery,
    Operation::PriorSelection,
    Operation::MediaInspection,
    Operation::LocationStaging,
    Operation::DirectoryPersistence,
    Operation::CheckpointPersistence,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Measurement {
    calls: u64,
    elapsed: Duration,
}

#[derive(Default)]
struct Accumulator {
    measurements: [Measurement; OPERATIONS.len()],
    active_operation: Option<Operation>,
}

thread_local! {
    static CURRENT: RefCell<Option<Rc<RefCell<Accumulator>>>> = const { RefCell::new(None) };
}

/// Only a synchronous, thread-local scan scope can receive operation costs.
pub(super) struct OperationCostScope {
    accumulator: Rc<RefCell<Accumulator>>,
}

pub(super) struct OperationTimer {
    active: Option<(Rc<RefCell<Accumulator>>, Operation, Instant)>,
}

pub(super) struct DiscoveryCostIterator<I> {
    inner: I,
}

impl<I> DiscoveryCostIterator<I> {
    pub(super) fn new(inner: I) -> Self {
        Self { inner }
    }
}

impl<I: Iterator> Iterator for DiscoveryCostIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let _measurement = OperationTimer::start(Operation::SourceDiscovery);
        self.inner.next()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct OperationCostReport {
    measurements: [Measurement; OPERATIONS.len()],
    accounted: Duration,
    unaccounted: Duration,
}

impl OperationCostScope {
    pub(super) fn start() -> Self {
        let accumulator = Rc::new(RefCell::new(Accumulator::default()));
        CURRENT.with(|current| {
            let mut current = current.borrow_mut();
            assert!(current.is_none(), "overlapping scan cost scopes");
            *current = Some(Rc::clone(&accumulator));
        });
        Self { accumulator }
    }

    pub(super) fn finish(self, elapsed: Duration) -> Result<OperationCostReport, &'static str> {
        let accumulator = self.accumulator.borrow();
        if accumulator.active_operation.is_some() {
            return Err("scan cost scope has an unfinished operation");
        }
        let accounted = accumulator
            .measurements
            .iter()
            .map(|measurement| measurement.elapsed)
            .sum();
        let unaccounted = elapsed
            .checked_sub(accounted)
            .ok_or("operation costs exceed the scan duration")?;
        Ok(OperationCostReport {
            measurements: accumulator.measurements,
            accounted,
            unaccounted,
        })
    }
}

impl Drop for OperationCostScope {
    fn drop(&mut self) {
        CURRENT.with(|current| {
            let mut current = current.borrow_mut();
            if current
                .as_ref()
                .is_some_and(|owner| Rc::ptr_eq(owner, &self.accumulator))
            {
                *current = None;
            }
        });
    }
}

impl OperationTimer {
    pub(super) fn start(operation: Operation) -> Self {
        let active = CURRENT.with(|current| {
            let accumulator = current.borrow().as_ref().cloned()?;
            {
                let mut state = accumulator.borrow_mut();
                assert!(
                    state.active_operation.is_none(),
                    "overlapping scan cost operations"
                );
                state.active_operation = Some(operation);
            }
            Some((accumulator, operation, Instant::now()))
        });
        Self { active }
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        if let Some((accumulator, operation, started)) = &self.active {
            let elapsed = started.elapsed();
            let mut state = accumulator.borrow_mut();
            let measurement = &mut state.measurements[*operation as usize];
            measurement.calls += 1;
            measurement.elapsed += elapsed;
            state.active_operation = None;
        }
    }
}

impl OperationCostReport {
    #[cfg(windows)]
    pub(super) fn print(&self, scan: &str) {
        for operation in OPERATIONS {
            let measurement = self.measurements[operation as usize];
            println!(
                "AME_SYNTHETIC_SCAN_OPERATION scan={scan} operation={operation:?} calls={} elapsed_us={}",
                measurement.calls,
                measurement.elapsed.as_micros(),
            );
        }
        println!(
            "AME_SYNTHETIC_SCAN_ACCOUNTING scan={scan} accounted_us={} unaccounted_us={}",
            self.accounted.as_micros(),
            self.unaccounted.as_micros(),
        );
    }
}

#[cfg(test)]
mod tests;
