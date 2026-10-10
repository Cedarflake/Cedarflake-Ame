use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Default)]
struct Measurement {
    calls: u64,
    elapsed: Duration,
    maximum: Duration,
}

thread_local! {
    static CAPTURE: RefCell<Option<BTreeMap<&'static str, Measurement>>> = const {
        RefCell::new(None)
    };
}

pub(in crate::application::library_synchronization) struct ObservationCapture;

impl ObservationCapture {
    pub(in crate::application::library_synchronization) fn start() -> Self {
        CAPTURE.with_borrow_mut(|capture| {
            assert!(capture.is_none(), "observation captures must not overlap");
            *capture = Some(BTreeMap::new());
        });
        Self
    }
}

pub(in crate::application::library_synchronization) fn record(
    stage: &'static str,
    elapsed: Duration,
) {
    CAPTURE.with_borrow_mut(|capture| {
        if let Some(capture) = capture {
            let measurement = capture.entry(stage).or_default();
            measurement.calls += 1;
            measurement.elapsed += elapsed;
            measurement.maximum = measurement.maximum.max(elapsed);
        }
    });
}

impl Drop for ObservationCapture {
    fn drop(&mut self) {
        let capture = CAPTURE
            .with_borrow_mut(Option::take)
            .expect("owned observation capture");
        for (stage, measurement) in capture {
            eprintln!(
                "controlled observation totals stage={stage} calls={} total_us={} max_us={}",
                measurement.calls,
                measurement.elapsed.as_micros(),
                measurement.maximum.as_micros(),
            );
        }
    }
}
