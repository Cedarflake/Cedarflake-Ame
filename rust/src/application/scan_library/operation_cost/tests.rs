use super::*;

#[test]
fn inactive_collection_does_not_create_a_scope_or_timer() {
    let timer = OperationTimer::start(Operation::MediaInspection);
    assert!(timer.active.is_none());
    drop(timer);
    CURRENT.with(|current| assert!(current.borrow().is_none()));
    let report = OperationCostScope::start().finish(Duration::ZERO).unwrap();
    assert_eq!(report.accounted, Duration::ZERO);
    assert!(
        report
            .measurements
            .iter()
            .all(|measurement| measurement.calls == 0)
    );
}

#[test]
fn every_operation_accounts_exactly_once_and_leaves_other_time_explicit() {
    let scope = OperationCostScope::start();
    for operation in OPERATIONS {
        drop(OperationTimer::start(operation));
    }
    let measured = scope.accumulator.borrow().measurements;
    let accounted: Duration = measured.iter().map(|measurement| measurement.elapsed).sum();
    let report = scope.finish(accounted + Duration::from_millis(7)).unwrap();
    assert!(
        report
            .measurements
            .iter()
            .all(|measurement| measurement.calls == 1)
    );
    assert_eq!(report.accounted, accounted);
    assert_eq!(report.unaccounted, Duration::from_millis(7));
}

#[test]
fn finishing_retires_the_scope_before_a_new_measurement() {
    let elapsed = Instant::now();
    let first = OperationCostScope::start();
    drop(OperationTimer::start(Operation::SourceDiscovery));
    let first = first.finish(elapsed.elapsed()).unwrap();
    let second = OperationCostScope::start().finish(Duration::ZERO).unwrap();
    assert_eq!(
        first.measurements[Operation::SourceDiscovery as usize].calls,
        1
    );
    assert_eq!(second.accounted, Duration::ZERO);
}

#[test]
fn an_operation_from_a_retired_scope_cannot_charge_the_next_scope() {
    let first = OperationCostScope::start();
    let old_timer = OperationTimer::start(Operation::LocationStaging);
    assert_eq!(
        first.finish(Duration::ZERO),
        Err("scan cost scope has an unfinished operation")
    );
    let elapsed = Instant::now();
    let second = OperationCostScope::start();
    let current_timer = OperationTimer::start(Operation::PriorSelection);
    drop(old_timer);
    assert_eq!(
        second.accumulator.borrow().active_operation,
        Some(Operation::PriorSelection)
    );
    drop(current_timer);
    let report = second.finish(elapsed.elapsed()).unwrap();
    assert_eq!(
        report.measurements[Operation::LocationStaging as usize].calls,
        0
    );
    assert_eq!(
        report.measurements[Operation::PriorSelection as usize].calls,
        1
    );
}

#[test]
fn another_thread_cannot_enter_the_current_scope() {
    let scope = OperationCostScope::start();
    std::thread::spawn(|| {
        let timer = OperationTimer::start(Operation::CheckpointPersistence);
        assert!(timer.active.is_none());
    })
    .join()
    .unwrap();
    assert_eq!(
        scope.finish(Duration::ZERO).unwrap().accounted,
        Duration::ZERO
    );
}

#[test]
fn unwind_retires_operation_and_scope() {
    let result = std::panic::catch_unwind(|| {
        let _scope = OperationCostScope::start();
        let _timer = OperationTimer::start(Operation::MediaInspection);
        panic!("controlled operation failure");
    });
    assert!(result.is_err());
    assert_eq!(
        OperationCostScope::start()
            .finish(Duration::ZERO)
            .unwrap()
            .accounted,
        Duration::ZERO
    );
}

#[test]
fn overlapping_operations_are_rejected_without_losing_the_original() {
    let elapsed = Instant::now();
    let scope = OperationCostScope::start();
    let original = OperationTimer::start(Operation::DirectoryPersistence);
    assert!(
        std::panic::catch_unwind(|| OperationTimer::start(Operation::LocationStaging)).is_err()
    );
    drop(original);
    let report = scope.finish(elapsed.elapsed()).unwrap();
    assert_eq!(
        report.measurements[Operation::DirectoryPersistence as usize].calls,
        1
    );
    assert_eq!(
        report.measurements[Operation::LocationStaging as usize].calls,
        0
    );
}

#[test]
fn overlapping_scopes_are_rejected_without_replacing_the_original() {
    let scope = OperationCostScope::start();
    assert!(std::panic::catch_unwind(OperationCostScope::start).is_err());
    CURRENT.with(|current| {
        assert!(Rc::ptr_eq(
            current.borrow().as_ref().unwrap(),
            &scope.accumulator
        ));
    });
    assert_eq!(
        scope.finish(Duration::ZERO).unwrap().accounted,
        Duration::ZERO
    );
}

#[test]
fn inconsistent_elapsed_time_rejects_the_report_and_retires_the_scope() {
    let scope = OperationCostScope::start();
    scope.accumulator.borrow_mut().measurements[Operation::SourceDiscovery as usize] =
        Measurement {
            calls: 1,
            elapsed: Duration::from_millis(1),
        };
    assert_eq!(
        scope.finish(Duration::ZERO),
        Err("operation costs exceed the scan duration")
    );
    OperationCostScope::start().finish(Duration::ZERO).unwrap();
}

#[test]
fn expression_instrumentation_preserves_single_evaluation_and_error_return() {
    fn failing_operation(calls: &mut u32) -> Result<(), &'static str> {
        measure_scan_operation!(PriorSelection, {
            *calls += 1;
            Err::<(), _>("original error")
        })?;
        Ok(())
    }
    let elapsed = Instant::now();
    let scope = OperationCostScope::start();
    let mut calls = 0;
    assert_eq!(failing_operation(&mut calls), Err("original error"));
    assert_eq!(calls, 1);
    let report = scope.finish(elapsed.elapsed()).unwrap();
    assert_eq!(
        report.measurements[Operation::PriorSelection as usize].calls,
        1
    );
}

#[test]
fn generated_scan_reaches_each_owning_operation_without_altering_membership() {
    use super::super::{StoragePaths, run_scan_with_storage};
    use crate::domain::{ScanEvent, ScanRequest};
    use image::{ImageFormat, Rgba, RgbaImage};

    let source = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    for filename in ["first.png", "second.png"] {
        RgbaImage::from_pixel(2, 2, Rgba([20, 40, 60, 255]))
            .save_with_format(source.path().join(filename), ImageFormat::Png)
            .unwrap();
    }
    let elapsed = Instant::now();
    let scope = OperationCostScope::start();
    let mut completed = false;
    run_scan_with_storage(
        ScanRequest {
            scan_id: "operation-cost-fixture".to_owned(),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        },
        |event| {
            if let ScanEvent::Completed {
                asset_count,
                issue_count,
                ..
            } = event
            {
                assert_eq!(asset_count, 2);
                assert_eq!(issue_count, 0);
                completed = true;
            }
            true
        },
        StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        },
    )
    .unwrap();
    assert!(completed);
    let report = scope.finish(elapsed.elapsed()).unwrap();
    assert_eq!(
        report.measurements[Operation::SourceDiscovery as usize].calls,
        6
    );
    for operation in [
        Operation::PriorSelection,
        Operation::MediaInspection,
        Operation::LocationStaging,
    ] {
        assert_eq!(report.measurements[operation as usize].calls, 2);
    }
    assert!(report.measurements[Operation::DirectoryPersistence as usize].calls > 0);
    assert!(report.measurements[Operation::CheckpointPersistence as usize].calls > 0);
    assert_eq!(source.path().read_dir().unwrap().count(), 2);
}

struct CountedDirectoryEntries {
    next_calls: Rc<std::cell::Cell<u32>>,
    drops: Rc<std::cell::Cell<u32>>,
    fail_on_next: bool,
}

impl Iterator for CountedDirectoryEntries {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next_calls.get();
        self.next_calls.set(index + 1);
        assert!(!self.fail_on_next, "controlled iterator failure");
        (index < 3).then_some(index)
    }
}

impl Drop for CountedDirectoryEntries {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

#[test]
fn discovery_iterator_is_lazy_and_charges_each_next_including_exhaustion() {
    let next_calls = Rc::new(std::cell::Cell::new(0));
    let drops = Rc::new(std::cell::Cell::new(0));
    let elapsed = Instant::now();
    let scope = OperationCostScope::start();
    let entries = DiscoveryCostIterator::new(CountedDirectoryEntries {
        next_calls: Rc::clone(&next_calls),
        drops: Rc::clone(&drops),
        fail_on_next: false,
    });
    assert_eq!(next_calls.get(), 0);
    assert_eq!(
        scope.accumulator.borrow().measurements[Operation::SourceDiscovery as usize].calls,
        0
    );
    assert_eq!(entries.collect::<Vec<_>>(), vec![0, 1, 2]);
    assert_eq!(next_calls.get(), 4);
    assert_eq!(drops.get(), 1);
    assert_eq!(
        scope.finish(elapsed.elapsed()).unwrap().measurements[Operation::SourceDiscovery as usize]
            .calls,
        4
    );
}

#[test]
fn discovery_early_exit_releases_iterator_without_an_extra_read() {
    let next_calls = Rc::new(std::cell::Cell::new(0));
    let drops = Rc::new(std::cell::Cell::new(0));
    let elapsed = Instant::now();
    let scope = OperationCostScope::start();
    let entries = DiscoveryCostIterator::new(CountedDirectoryEntries {
        next_calls: Rc::clone(&next_calls),
        drops: Rc::clone(&drops),
        fail_on_next: false,
    });
    for entry in entries {
        if entry == 0 {
            break;
        }
    }
    assert_eq!(next_calls.get(), 1);
    assert_eq!(drops.get(), 1);
    assert_eq!(
        scope.finish(elapsed.elapsed()).unwrap().measurements[Operation::SourceDiscovery as usize]
            .calls,
        1
    );
}

#[test]
fn failed_discovery_releases_iterator_and_does_not_retain_measurement_authority() {
    let next_calls = Rc::new(std::cell::Cell::new(0));
    let drops = Rc::new(std::cell::Cell::new(0));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _scope = OperationCostScope::start();
        let mut entries = DiscoveryCostIterator::new(CountedDirectoryEntries {
            next_calls: Rc::clone(&next_calls),
            drops: Rc::clone(&drops),
            fail_on_next: true,
        });
        entries.next();
    }));
    assert!(result.is_err());
    assert_eq!(next_calls.get(), 1);
    assert_eq!(drops.get(), 1);
    OperationCostScope::start().finish(Duration::ZERO).unwrap();
}
