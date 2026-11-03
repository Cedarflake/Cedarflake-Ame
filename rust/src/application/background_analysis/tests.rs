use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use super::*;

fn wait_for_waiters(gate: &BackgroundAnalysisGate, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if gate.state.lock().unwrap().waiting.len() == expected {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "background request did not arrive"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn an_already_waiting_task_runs_before_the_previous_consumer_reacquires() {
    let gate = BackgroundAnalysisGate::default();
    let cancelled = AtomicBool::new(false);
    let first = gate.acquire(&cancelled).unwrap().unwrap();
    let (entered, observed) = mpsc::channel();
    let (release, retained) = mpsc::channel();
    thread::scope(|scope| {
        let gate_ref = &gate;
        let cancelled_ref = &cancelled;
        let second_entered = entered.clone();
        let second = scope.spawn(move || {
            let _lease = gate_ref.acquire(cancelled_ref).unwrap().unwrap();
            second_entered.send("classification").unwrap();
            retained.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        wait_for_waiters(&gate, 1);
        let third = scope.spawn(|| {
            let _lease = gate.acquire(&cancelled).unwrap().unwrap();
            entered.send("next exact source").unwrap();
        });
        wait_for_waiters(&gate, 2);
        assert!(observed.try_recv().is_err());
        drop(first);
        assert_eq!(
            observed.recv_timeout(Duration::from_secs(5)).unwrap(),
            "classification"
        );
        assert!(observed.try_recv().is_err());
        release.send(()).unwrap();
        assert_eq!(
            observed.recv_timeout(Duration::from_secs(5)).unwrap(),
            "next exact source"
        );
        second.join().unwrap();
        third.join().unwrap();
    });
    assert!(gate.acquire(&cancelled).unwrap().is_some());
}

#[test]
fn cancelled_waiter_retires_without_releasing_another_tasks_source() {
    let gate = BackgroundAnalysisGate::default();
    let active_cancelled = AtomicBool::new(false);
    let first = gate.acquire(&active_cancelled).unwrap().unwrap();
    let cancelled = AtomicBool::new(false);
    thread::scope(|scope| {
        let waiter = scope.spawn(|| gate.acquire(&cancelled).unwrap().is_none());
        wait_for_waiters(&gate, 1);
        cancelled.store(true, Ordering::Release);
        assert!(waiter.join().unwrap());
    });
    let state = gate.state.lock().unwrap();
    assert!(state.waiting.is_empty());
    assert_eq!(state.active, Some(first.ticket));
    drop(state);
    drop(first);
    assert!(gate.acquire(&active_cancelled).unwrap().is_some());
}

#[test]
fn pending_admission_is_bounded_and_cancellation_drains_all_waiters() {
    let gate = BackgroundAnalysisGate::default();
    let active_cancelled = AtomicBool::new(false);
    let _first = gate.acquire(&active_cancelled).unwrap().unwrap();
    let cancelled = AtomicBool::new(false);
    thread::scope(|scope| {
        let mut waiters = Vec::new();
        for _ in 0..MAX_WAITERS {
            waiters.push(scope.spawn(|| gate.acquire(&cancelled).unwrap().is_none()));
        }
        wait_for_waiters(&gate, MAX_WAITERS);
        assert_eq!(
            gate.acquire(&active_cancelled).err().unwrap().code,
            "background_analysis_busy"
        );
        cancelled.store(true, Ordering::Release);
        for waiter in waiters {
            assert!(waiter.join().unwrap());
        }
    });
    assert!(gate.state.lock().unwrap().waiting.is_empty());
}

#[test]
fn cancellation_before_admission_and_failed_work_leave_no_reservation() {
    let gate = BackgroundAnalysisGate::default();
    assert!(gate.acquire(&AtomicBool::new(true)).unwrap().is_none());
    let cancelled = AtomicBool::new(false);
    let result = (|| {
        let _lease = gate.acquire(&cancelled)?.unwrap();
        Err::<(), _>(ScanError::new("source_changed", "The source changed"))
    })();
    assert_eq!(result.unwrap_err().code, "source_changed");
    assert!(gate.acquire(&cancelled).unwrap().is_some());
}
