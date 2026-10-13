use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use super::*;

#[test]
fn inactive_observation_does_not_resolve_a_source_root() {
    let counts = SourceContentOpenCounts::default();
    let resolutions = Cell::new(0);
    counts.record(|| {
        resolutions.set(resolutions.get() + 1);
        PathBuf::from("unobserved-root")
    });
    assert_eq!(
        resolutions.get(),
        0,
        "disabled observation must perform no root I/O"
    );
}

#[test]
fn enabled_observation_counts_the_resolved_root_once() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("canonical-root");
    counts.reset(root.clone());
    let resolutions = Cell::new(0);
    counts.record(|| {
        resolutions.set(resolutions.get() + 1);
        root.clone()
    });
    assert_eq!(resolutions.get(), 1);
    assert_eq!(counts.count(&root), 1);
}

#[test]
fn unrelated_roots_do_not_create_or_increment_counters() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("observed-root");
    let unrelated = PathBuf::from("unrelated-root");
    counts.reset(root.clone());
    counts.record(|| unrelated.clone());
    assert_eq!(counts.count(&root), 0);
    assert_eq!(counts.count(&unrelated), 0);
    assert_eq!(counts.roots.lock().expect("counts").len(), 1);
}

#[test]
fn resetting_one_root_preserves_other_observations() {
    let counts = SourceContentOpenCounts::default();
    let first = PathBuf::from("first-root");
    let second = PathBuf::from("second-root");
    counts.reset(first.clone());
    counts.reset(second.clone());
    counts.record(|| first.clone());
    counts.record(|| second.clone());
    counts.reset(first.clone());
    assert_eq!(counts.count(&first), 0);
    assert_eq!(counts.count(&second), 1);
}

#[test]
fn source_resolution_does_not_hold_the_counter_lock() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("observed-root");
    counts.reset(root.clone());
    counts.record(|| {
        assert!(counts.roots.try_lock().is_ok());
        root.clone()
    });
    assert_eq!(counts.count(&root), 1);
}

#[test]
fn resolution_failure_does_not_poison_or_increment_counts() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("observed-root");
    counts.reset(root.clone());
    assert!(
        catch_unwind(AssertUnwindSafe(
            || counts.record(|| panic!("resolution failure"))
        ))
        .is_err()
    );
    assert_eq!(counts.count(&root), 0);
    counts.record(|| root.clone());
    assert_eq!(counts.count(&root), 1);
}

#[test]
fn maximum_count_stays_saturated() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("observed-root");
    counts
        .roots
        .lock()
        .expect("counts")
        .insert(root.clone(), u64::MAX);
    counts.record(|| root.clone());
    assert_eq!(counts.count(&root), u64::MAX);
}

#[test]
fn concurrent_content_opens_are_all_counted() {
    let counts = SourceContentOpenCounts::default();
    let root = PathBuf::from("observed-root");
    counts.reset(root.clone());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..32 {
                    counts.record(|| root.clone());
                }
            });
        }
    });
    assert_eq!(counts.count(&root), 256);
}
