use std::sync::atomic::AtomicU64;

use super::*;

#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);

impl ClassificationDispatchClock for Clock {
    fn elapsed(&self) -> Duration {
        Duration::from_millis(self.unix_ms())
    }
    fn unix_ms(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }
}

impl Clock {
    fn set(&self, millis: u64) {
        self.0.store(millis, Ordering::Release);
    }
}

fn active(signals: &ClassificationDispatchSignals<Clock>) -> ActiveWork<Clock> {
    match signals.admission(true).unwrap() {
        Admission::Ready(active) => active,
        _ => panic!("expected ready work"),
    }
}

#[test]
fn cached_browsing_defers_initial_admission_without_interrupting_an_admitted_queue() {
    let clock = Clock::default();
    let signals = ClassificationDispatchSignals::new(clock.clone());
    clock.set(2_999);
    signals.browsing_activity().unwrap();
    clock.set(5_998);
    assert!(
        matches!(signals.admission(true).unwrap(), Admission::Wait { delay: Some(delay), retire: false }
        if delay == Duration::from_millis(1))
    );
    clock.set(5_999);
    let first = active(&signals);
    clock.set(6_000);
    signals.browsing_activity().unwrap();
    assert!(!first.cancelled.load(Ordering::Acquire));
    drop(first);
    let second = active(&signals);
    assert!(!second.cancelled.load(Ordering::Acquire));
}

#[test]
fn source_reads_cancel_active_work_and_require_quiet_after_the_read_finishes() {
    let clock = Clock::default();
    let signals = ClassificationDispatchSignals::new(clock.clone());
    clock.set(3_000);
    let work = active(&signals);
    signals
        .foreground_changed(ClassificationForegroundState {
            reading_sources: true,
            ..Default::default()
        })
        .unwrap();
    assert!(work.cancelled.load(Ordering::Acquire));
    drop(work);
    clock.set(20_000);
    assert!(matches!(
        signals.admission(true).unwrap(),
        Admission::Wait {
            delay: None,
            retire: true
        }
    ));
    signals
        .foreground_changed(ClassificationForegroundState::default())
        .unwrap();
    assert!(
        matches!(signals.admission(true).unwrap(), Admission::Wait { delay: Some(delay), retire: false }
        if delay == INITIAL_QUIET)
    );
    clock.set(23_000);
    drop(active(&signals));
}

#[test]
fn pressure_suspend_and_import_each_hold_admission_and_request_retirement() {
    for foreground in [
        ClassificationForegroundState {
            importing: true,
            ..Default::default()
        },
        ClassificationForegroundState {
            suspended: true,
            ..Default::default()
        },
        ClassificationForegroundState {
            memory_pressure: true,
            ..Default::default()
        },
    ] {
        let clock = Clock::default();
        let signals = ClassificationDispatchSignals::new(clock.clone());
        clock.set(3_000);
        let work = active(&signals);
        signals.foreground_changed(foreground).unwrap();
        assert!(work.cancelled.load(Ordering::Acquire));
        drop(work);
        assert!(matches!(
            signals.admission(true).unwrap(),
            Admission::Wait {
                delay: None,
                retire: true
            }
        ));
    }
}

#[test]
fn automatic_readiness_never_overrides_manual_pause() {
    let clock = Clock::default();
    let signals = ClassificationDispatchSignals::new(clock.clone());
    clock.set(100_000);
    signals.catalog_changed().unwrap();
    assert!(matches!(
        signals.admission(false).unwrap(),
        Admission::Wait {
            delay: None,
            retire: true
        }
    ));
    signals.dispatch_changed().unwrap();
    assert!(matches!(
        signals.admission(false).unwrap(),
        Admission::Wait {
            delay: None,
            retire: true
        }
    ));
    clock.set(103_000);
    drop(active(&signals));
}

#[test]
fn empty_queue_releases_browsing_admission_and_close_cannot_be_reversed() {
    let clock = Clock::default();
    let signals = ClassificationDispatchSignals::new(clock.clone());
    clock.set(3_000);
    let work = active(&signals);
    signals.idle(work.catalog_revision).unwrap();
    drop(work);
    clock.set(10_000);
    signals.browsing_activity().unwrap();
    assert!(matches!(
        signals.admission(true).unwrap(),
        Admission::Wait { delay: Some(_), .. }
    ));
    signals.close().unwrap();
    signals.dispatch_changed().unwrap();
    signals
        .foreground_changed(ClassificationForegroundState::default())
        .unwrap();
    assert!(matches!(
        signals.admission(true).unwrap(),
        Admission::Closed
    ));
}

#[test]
fn notification_between_check_and_wait_is_not_lost() {
    let signals = ClassificationDispatchSignals::new(Clock::default());
    let observed = signals.notification().unwrap();
    signals.catalog_changed().unwrap();
    signals.wait(observed, None).unwrap();
    let observed = signals.notification().unwrap();
    signals.close().unwrap();
    signals.wait(observed, None).unwrap();
}
