use std::cell::{Cell, RefCell};

use crate::domain::CatalogSpaceUsage;
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl};

use super::*;

struct Maintenance {
    usage: CatalogSpaceUsage,
    calls: RefCell<Vec<&'static str>>,
}

impl CatalogSpaceRepository for Maintenance {
    fn try_discard_retired_scan_staging(
        &self,
        _: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<usize>, ScanError> {
        self.calls.borrow_mut().push("discard");
        Ok(CatalogMaintenanceAttempt::Completed(0))
    }

    fn inspect_catalog_space(
        &self,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        self.calls.borrow_mut().push("inspect");
        Ok(CatalogMaintenanceAttempt::Completed(self.usage))
    }

    fn try_convert_to_incremental(
        &self,
        _: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        self.calls.borrow_mut().push("convert");
        Ok(CatalogMaintenanceAttempt::Completed(CatalogSpaceUsage {
            auto_vacuum: CatalogAutoVacuumMode::Incremental,
            ..self.usage
        }))
    }

    fn try_reclaim_incremental(
        &self,
        _: u32,
        _: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        panic!("These capacity fixtures must not reach incremental page release")
    }

    fn try_checkpoint_wal(
        &self,
        _: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<()>, ScanError> {
        self.calls.borrow_mut().push("checkpoint");
        Ok(CatalogMaintenanceAttempt::Completed(()))
    }
}

struct Capacity {
    result: Result<u64, ScanError>,
    calls: Cell<usize>,
}

impl CatalogCapacity for Capacity {
    fn available_for(&self, _: &Path) -> Result<u64, ScanError> {
        self.calls.set(self.calls.get() + 1);
        self.result.clone()
    }
}

fn legacy_catalog() -> Maintenance {
    Maintenance {
        usage: CatalogSpaceUsage {
            page_size: 4096,
            page_count: 65_536,
            freelist_count: 16_384,
            auto_vacuum: CatalogAutoVacuumMode::None,
        },
        calls: RefCell::new(Vec::new()),
    }
}

#[test]
fn insufficient_capacity_never_enters_conversion_or_checkpoint() {
    let maintenance = legacy_catalog();
    let capacity = Capacity {
        result: Ok(0),
        calls: Cell::new(0),
    };
    let operation = CatalogReclamationOperation::new("insufficient".to_owned());
    let error = run(
        Path::new("catalog.sqlite3"),
        &operation,
        &maintenance,
        &capacity,
    )
    .err()
    .expect("insufficient capacity");
    assert_eq!(error.code, "catalog_reclamation_space_insufficient");
    assert_eq!(*maintenance.calls.borrow(), ["discard", "inspect"]);
    assert_eq!(capacity.calls.get(), 1);
    let snapshot = operation.snapshot().unwrap();
    assert_eq!(snapshot.available_temporary_bytes, Some(0));
    assert_eq!(snapshot.required_temporary_bytes, Some(576 * 1024 * 1024));
}

#[test]
fn capacity_probe_failure_preserves_its_error_without_conversion() {
    let maintenance = legacy_catalog();
    let capacity = Capacity {
        result: Err(ScanError::new(
            "capacity_fixture_failed",
            "Capacity unavailable",
        )),
        calls: Cell::new(0),
    };
    let operation = CatalogReclamationOperation::new("capacity-failure".to_owned());
    let error = run(
        Path::new("catalog.sqlite3"),
        &operation,
        &maintenance,
        &capacity,
    )
    .err()
    .expect("capacity failure");
    assert_eq!(error.code, "capacity_fixture_failed");
    assert_eq!(error.message, "Capacity unavailable");
    assert_eq!(*maintenance.calls.borrow(), ["discard", "inspect"]);
    assert_eq!(capacity.calls.get(), 1);
}

#[test]
fn cancelled_work_does_not_probe_or_mutate_storage() {
    let maintenance = legacy_catalog();
    let capacity = Capacity {
        result: Ok(u64::MAX),
        calls: Cell::new(0),
    };
    let operation = CatalogReclamationOperation::new("cancelled".to_owned());
    operation.cancel();
    assert!(matches!(
        run(
            Path::new("catalog.sqlite3"),
            &operation,
            &maintenance,
            &capacity
        )
        .unwrap(),
        ReclamationCompletion::Cancelled,
    ));
    assert!(maintenance.calls.borrow().is_empty());
    assert_eq!(capacity.calls.get(), 0);
}

#[test]
fn full_mode_conversion_does_not_request_legacy_rewrite_capacity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    let maintenance = Maintenance {
        usage: CatalogSpaceUsage {
            page_size: 4096,
            page_count: 100,
            freelist_count: 0,
            auto_vacuum: CatalogAutoVacuumMode::Full,
        },
        calls: RefCell::new(Vec::new()),
    };
    let capacity = Capacity {
        result: Err(ScanError::new(
            "capacity_not_needed",
            "Full mode needs no rewrite",
        )),
        calls: Cell::new(0),
    };
    let operation = CatalogReclamationOperation::new("full-mode".to_owned());
    assert!(matches!(
        run(&path, &operation, &maintenance, &capacity).unwrap(),
        ReclamationCompletion::Completed,
    ));
    assert_eq!(
        *maintenance.calls.borrow(),
        ["discard", "inspect", "convert", "checkpoint"]
    );
    assert_eq!(capacity.calls.get(), 0);
}
