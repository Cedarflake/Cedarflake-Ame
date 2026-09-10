use std::cell::RefCell;
use std::thread;
use std::time::{Duration, Instant};

const DART_DURATION_MAXIMUM_MILLISECONDS: u64 = 9_223_372_036_854_775;
const LIBRARY_SYNCHRONIZATION_POLL_INTERVAL_MS_SOURCE: &str =
    include_str!("../../tool/library_synchronization_poll_interval_ms.txt");

thread_local! {
    static CAPTURED_WAIT_INTERVALS: RefCell<Option<Vec<Duration>>> = const { RefCell::new(None) };
}

struct WaitIntervalCaptureGuard;

impl Drop for WaitIntervalCaptureGuard {
    fn drop(&mut self) {
        CAPTURED_WAIT_INTERVALS.with(|captured| {
            captured.borrow_mut().take();
        });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ProductionSynchronizationCadence(Duration);

impl ProductionSynchronizationCadence {
    pub(super) fn from_shared_policy() -> Self {
        Self::try_from_policy_source(LIBRARY_SYNCHRONIZATION_POLL_INTERVAL_MS_SOURCE)
            .expect("tracked production library synchronization poll interval")
    }

    pub(super) fn try_from_policy_source(source: &str) -> Result<Self, String> {
        let digits = source
            .strip_suffix('\n')
            .ok_or_else(|| "production synchronization cadence must end with LF".to_owned())?;
        if digits.is_empty() {
            return Err("production synchronization cadence must not be empty".to_owned());
        }
        if digits.starts_with('0') {
            return Err(
                "production synchronization cadence must be positive without leading zeroes"
                    .to_owned(),
            );
        }
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(
                "production synchronization cadence must contain only decimal digits".to_owned(),
            );
        }
        let milliseconds = digits
            .parse::<u64>()
            .map_err(|_| "production synchronization cadence exceeds u64".to_owned())?;
        if milliseconds > DART_DURATION_MAXIMUM_MILLISECONDS {
            return Err(
                "production synchronization cadence exceeds the Dart Duration microsecond range"
                    .to_owned(),
            );
        }
        Ok(Self(Duration::from_millis(milliseconds)))
    }

    pub(super) fn wait_until(&self, timeout: Duration, mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + timeout;
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "bounded production synchronization cadence wait timed out"
            );
            sleep_for_production_cadence(self.0);
        }
    }

    const fn interval(self) -> Duration {
        self.0
    }
}

pub(super) fn capture_wait_intervals_for_contract<T>(
    action: impl FnOnce() -> T,
) -> (T, Vec<Duration>) {
    CAPTURED_WAIT_INTERVALS.with(|captured| {
        let mut captured = captured.borrow_mut();
        assert!(
            captured.is_none(),
            "production cadence wait capture must not be nested"
        );
        *captured = Some(Vec::new());
    });
    let guard = WaitIntervalCaptureGuard;
    let result = action();
    let intervals = CAPTURED_WAIT_INTERVALS.with(|captured| {
        captured
            .borrow_mut()
            .take()
            .expect("production cadence wait capture")
    });
    drop(guard);
    (result, intervals)
}

fn sleep_for_production_cadence(interval: Duration) {
    let was_captured = CAPTURED_WAIT_INTERVALS.with(|captured| {
        let mut captured = captured.borrow_mut();
        if let Some(intervals) = captured.as_mut() {
            intervals.push(interval);
            true
        } else {
            false
        }
    });
    if !was_captured {
        thread::sleep(interval);
    }
}

#[test]
fn shared_production_cadence_reads_the_tracked_policy() {
    assert_eq!(
        ProductionSynchronizationCadence::from_shared_policy().interval(),
        Duration::from_millis(250)
    );
}

#[test]
fn shared_production_cadence_enforces_canonical_text_and_dart_microsecond_range() {
    assert_eq!(
        ProductionSynchronizationCadence::try_from_policy_source("9223372036854775\n")
            .map(ProductionSynchronizationCadence::interval),
        Ok(Duration::from_millis(9_223_372_036_854_775))
    );
    for invalid in [
        "",
        "0\n",
        "01\n",
        "+1\n",
        "-1\n",
        "1ms\n",
        " 1\n",
        "1 \n",
        "1\r\n",
        "1\n2\n",
        "1",
        "\u{feff}1\n",
        "9223372036854776\n",
        "18446744073709551615\n",
        "18446744073709551616\n",
    ] {
        assert!(
            ProductionSynchronizationCadence::try_from_policy_source(invalid).is_err(),
            "accepted invalid production cadence policy {invalid:?}"
        );
    }
}

#[test]
fn shared_production_cadence_wait_uses_the_parsed_interval() {
    let cadence = ProductionSynchronizationCadence::try_from_policy_source("875\n")
        .expect("shared 875 ms production cadence policy");
    let mut attempts = 0;
    let (_, intervals) = capture_wait_intervals_for_contract(|| {
        cadence.wait_until(Duration::from_secs(1), || {
            attempts += 1;
            attempts == 2
        });
    });

    assert_eq!(intervals, [Duration::from_millis(875)]);
}
