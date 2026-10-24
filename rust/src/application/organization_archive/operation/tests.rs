use super::*;

#[test]
fn organization_archive_admission_lasts_until_the_cancelled_worker_retires() {
    let active = AtomicBool::new(false);
    let first = ArchiveTaskControl::default();
    let lease = first.admit_with(&active).unwrap();
    first.cancel();
    assert!(!first.should_continue());
    assert_eq!(
        ArchiveTaskControl::default()
            .admit_with(&active)
            .err()
            .unwrap()
            .code,
        "organization_archive_busy"
    );
    drop(lease);
    let second = ArchiveTaskControl::default();
    let lease = second.admit_with(&active).unwrap();
    first.cancel();
    assert!(second.should_continue());
    drop(lease);
    assert!(!active.load(Ordering::Acquire));
}

#[test]
fn organization_archive_cancel_before_dispatch_does_not_acquire_admission() {
    let active = AtomicBool::new(false);
    let task = ArchiveTaskControl::default();
    task.cancel();
    assert_eq!(
        task.admit_with(&active).err().unwrap().code,
        "organization_archive_cancelled"
    );
    assert!(!active.load(Ordering::Acquire));
}

#[test]
fn organization_archive_task_cannot_be_dispatched_twice() {
    let active = AtomicBool::new(false);
    let task = ArchiveTaskControl::default();
    drop(task.admit_with(&active).unwrap());
    assert_eq!(
        task.admit_with(&active).err().unwrap().code,
        "organization_archive_task_used"
    );
    assert!(!active.load(Ordering::Acquire));
}
