use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use super::*;

thread_local! {
    static AFTER_REGISTRATION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
}

pub(super) fn after_registration() {
    let callback = AFTER_REGISTRATION.with(|slot| slot.borrow_mut().take());
    if let Some(callback) = callback {
        callback();
    }
}

#[test]
fn a_new_try_writer_cannot_barge_a_registered_same_priority_waiter() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (registered_tx, registered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let waiting_admission = Arc::clone(&admission);
    let waiter = thread::spawn(move || {
        AFTER_REGISTRATION.with(|slot| {
            *slot.borrow_mut() = Some(Box::new(move || {
                registered_tx.send(()).expect("registered waiter");
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("release registered waiter");
            }));
        });
        drop(waiting_admission.acquire(LibraryChangeLane::Recovery));
    });
    let registered = registered_rx.recv_timeout(Duration::from_secs(5));
    drop(owner);
    let barging = admission.try_acquire(LibraryChangeLane::Recovery);
    let barged = barging.is_some();
    drop(barging);
    let released = release_tx.send(());
    join_writer(waiter);
    registered.expect("older waiter reached the real admission queue");
    released.expect("release older waiter");
    assert!(
        !barged,
        "a new writer overtook a registered same-priority waiter"
    );
    assert!(admission.try_acquire(LibraryChangeLane::Recovery).is_some());
}

#[test]
fn blocking_writers_follow_registration_order_not_wakeup_order() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (order_tx, order_rx) = mpsc::channel();
    let first = GatedWriter::spawn(&admission, LibraryChangeLane::Recovery, 1, order_tx.clone());
    let second = GatedWriter::spawn(&admission, LibraryChangeLane::Recovery, 2, order_tx);
    drop(owner);
    second.release.send(()).expect("wake younger waiter first");
    let premature = order_rx.recv_timeout(Duration::from_millis(50));
    first.release.send(()).expect("wake older waiter");
    let first_result = order_rx.recv_timeout(Duration::from_secs(5));
    let second_result = order_rx.recv_timeout(Duration::from_secs(5));
    assert!(join_writer(first.worker));
    assert!(join_writer(second.worker));
    assert!(
        premature.is_err(),
        "wakeup order bypassed registration order"
    );
    assert_eq!(first_result.expect("first admitted writer"), 1);
    assert_eq!(second_result.expect("second admitted writer"), 2);
    assert_idle(&admission);
}

#[test]
fn higher_priority_try_writer_still_precedes_older_recovery_waiter() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (order_tx, order_rx) = mpsc::channel();
    let waiting = GatedWriter::spawn(&admission, LibraryChangeLane::Recovery, 1, order_tx);
    drop(owner);
    let live = admission.try_acquire(LibraryChangeLane::Live);
    let admitted = live.is_some();
    waiting.release.send(()).expect("release recovery waiter");
    let premature = order_rx.recv_timeout(Duration::from_millis(50));
    drop(live);
    let recovered = order_rx.recv_timeout(Duration::from_secs(5));
    assert!(join_writer(waiting.worker));
    assert!(
        admitted,
        "same-lane fairness must not reverse priority order"
    );
    assert!(premature.is_err(), "recovery cannot overlap a live writer");
    assert_eq!(recovered.expect("recovery completed after live"), 1);
    assert_idle(&admission);
}

#[test]
fn interactive_timeout_retires_only_its_own_waiter() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let epoch = admission.completed_write_epoch();
    let (order_tx, order_rx) = mpsc::channel();
    let waiting = GatedWriter::spawn(&admission, LibraryChangeLane::Recovery, 1, order_tx);
    assert!(
        admission
            .acquire_user_interactive_for(Duration::ZERO)
            .is_none()
    );
    {
        let state = admission.state.lock().expect("inspect waiters");
        assert!(state.waiting[SQLITE_USER_INTERACTIVE_PRIORITY].is_empty());
        assert_eq!(
            state.waiting[sqlite_write_priority(LibraryChangeLane::Recovery)].len(),
            1
        );
    }
    waiting.release.send(()).expect("release surviving waiter");
    assert_eq!(admission.completed_write_epoch(), epoch);
    drop(owner);
    let recovered = order_rx.recv_timeout(Duration::from_secs(5));
    assert!(join_writer(waiting.worker));
    assert_eq!(recovered.expect("recovery admitted after timeout"), 1);
    assert_idle(&admission);
}

#[test]
fn preemption_callback_unwind_retires_registration_without_revoking_active_writer() {
    for interactive in [false, true] {
        let admission = Arc::new(SqliteWriteAdmission::new());
        let owner = admission.acquire_preemptible(
            LibraryChangeLane::Recovery,
            Arc::new(|| {
                panic!("controlled preemption callback unwind");
            }),
        );
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if interactive {
                drop(admission.acquire_user_interactive());
            } else {
                drop(admission.acquire(LibraryChangeLane::Live));
            }
        }));
        assert!(result.is_err());
        assert_eq!(admission.completed_write_epoch(), 0);
        {
            let state = admission.state.lock().expect("unpoisoned admission state");
            assert!(state.is_active);
            assert!(state.waiting.iter().all(VecDeque::is_empty));
        }
        assert!(admission.try_acquire(LibraryChangeLane::Recovery).is_none());
        drop(owner);
        assert_eq!(admission.completed_write_epoch(), 1);
        assert_idle(&admission);
    }
}

#[test]
fn timed_out_middle_waiter_preserves_both_same_priority_neighbors() {
    let admission = Arc::new(SqliteWriteAdmission::new());
    let owner = admission.acquire(LibraryChangeLane::Recovery);
    let (order_tx, order_rx) = mpsc::channel();
    let first = GatedWriter::spawn_priority(
        &admission,
        SQLITE_USER_INTERACTIVE_PRIORITY,
        1,
        order_tx.clone(),
    );
    let middle = GatedWriter::spawn_mode(
        &admission,
        WaitingMode::Interactive(Duration::ZERO),
        9,
        order_tx.clone(),
    );
    let second =
        GatedWriter::spawn_priority(&admission, SQLITE_USER_INTERACTIVE_PRIORITY, 2, order_tx);
    let before_timeout = admission
        .state
        .lock()
        .expect("registered interactive queue")
        .waiting[SQLITE_USER_INTERACTIVE_PRIORITY]
        .len();
    middle
        .release
        .send(())
        .expect("expire the occupied middle position");
    let middle_admitted = join_writer(middle.worker);
    let after_timeout = admission
        .state
        .lock()
        .expect("queue after middle retirement")
        .waiting[SQLITE_USER_INTERACTIVE_PRIORITY]
        .len();
    let epoch_after_timeout = admission.completed_write_epoch();
    second
        .release
        .send(())
        .expect("wake younger interactive waiter");
    first
        .release
        .send(())
        .expect("wake older interactive waiter");
    drop(owner);
    let first_result = order_rx.recv_timeout(Duration::from_secs(5));
    let second_result = order_rx.recv_timeout(Duration::from_secs(5));
    assert!(join_writer(first.worker));
    assert!(join_writer(second.worker));
    assert!(!middle_admitted);
    assert_eq!((before_timeout, after_timeout), (3, 2));
    assert_eq!(epoch_after_timeout, 0);
    assert_eq!(first_result.expect("older interactive writer"), 1);
    assert_eq!(second_result.expect("younger interactive writer"), 2);
    assert_idle(&admission);
}

struct GatedWriter {
    release: mpsc::SyncSender<()>,
    worker: thread::JoinHandle<bool>,
}

enum WaitingMode {
    Priority(usize),
    Interactive(Duration),
}

impl GatedWriter {
    fn spawn(
        admission: &Arc<SqliteWriteAdmission>,
        lane: LibraryChangeLane,
        label: u8,
        order: mpsc::Sender<u8>,
    ) -> Self {
        Self::spawn_priority(admission, sqlite_write_priority(lane), label, order)
    }

    fn spawn_priority(
        admission: &Arc<SqliteWriteAdmission>,
        priority: usize,
        label: u8,
        order: mpsc::Sender<u8>,
    ) -> Self {
        Self::spawn_mode(admission, WaitingMode::Priority(priority), label, order)
    }

    fn spawn_mode(
        admission: &Arc<SqliteWriteAdmission>,
        mode: WaitingMode,
        label: u8,
        order: mpsc::Sender<u8>,
    ) -> Self {
        let admission = Arc::clone(admission);
        let (registered_tx, registered_rx) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            AFTER_REGISTRATION.with(|slot| {
                *slot.borrow_mut() = Some(Box::new(move || {
                    registered_tx
                        .send(())
                        .expect("report actual queue registration");
                    release_rx
                        .recv_timeout(Duration::from_secs(5))
                        .expect("release registered writer");
                }));
            });
            let permit = match mode {
                WaitingMode::Priority(priority) => Some(admission.acquire_priority(priority, None)),
                WaitingMode::Interactive(timeout) => {
                    admission.acquire_user_interactive_for(timeout)
                }
            };
            if permit.is_some() {
                order.send(label).expect("report owned permit");
            }
            permit.is_some()
        });
        registered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("writer registered before the next writer starts");
        Self { release, worker }
    }
}

fn join_writer<T>(worker: thread::JoinHandle<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !worker.is_finished() {
        assert!(
            Instant::now() < deadline,
            "owned admission writer retirement was not confirmed"
        );
        thread::sleep(Duration::from_millis(1));
    }
    worker
        .join()
        .expect("owned admission writer retired without panic")
}

fn assert_idle(admission: &Arc<SqliteWriteAdmission>) {
    {
        let state = admission.state.lock().expect("admission state");
        assert!(!state.is_active);
        assert!(state.waiting.iter().all(VecDeque::is_empty));
        assert!(state.active_preempt.is_none());
    }
    drop(
        admission
            .try_acquire(LibraryChangeLane::Recovery)
            .expect("no abandoned queue owner"),
    );
}
