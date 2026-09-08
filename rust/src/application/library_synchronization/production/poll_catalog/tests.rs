use std::panic::{AssertUnwindSafe, catch_unwind};

use super::*;

#[test]
fn poll_catalog_reuses_one_connection_and_returns_it_after_error_and_unwind() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let owner = PollCatalogOwner::default();
    for _ in 0..100 {
        drop(owner.checkout(&path).unwrap());
    }
    let failed: Result<(), ScanError> = (|| {
        let _checkout = owner.checkout(&path)?;
        Err(ScanError::new("controlled_error", "Controlled poll error"))
    })();
    assert!(failed.is_err());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _checkout = owner.checkout(&path).unwrap();
            panic!("controlled poll unwind");
        }))
        .is_err()
    );
    drop(owner.checkout(&path).unwrap());
    assert_eq!(owner.connection_counts(), (1, 0));
    owner.retire().unwrap();
    owner.retire().unwrap();
    assert_eq!(owner.connection_counts(), (1, 1));
    assert!(owner.checkout(&path).is_err());
}

#[test]
fn poll_catalog_rejects_duplicate_checkout_and_retirement_of_an_active_lease() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let owner = PollCatalogOwner::default();
    let checkout = owner.checkout(&path).unwrap();
    assert!(owner.checkout(&path).is_err());
    assert!(owner.retire().is_err());
    assert_eq!(owner.connection_counts(), (1, 0));
    drop(checkout);
    drop(owner.checkout(&path).unwrap());
    owner.retire().unwrap();
    assert_eq!(owner.connection_counts(), (1, 1));
}

#[test]
fn poll_catalog_process_proof_revocation_reopens_even_without_header_changes() {
    use crate::application::catalog_session::with_catalog_maintenance;
    use crate::ports::CatalogMaintenanceAttempt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let owner = PollCatalogOwner::default();
    drop(owner.checkout(&path).unwrap());
    with_catalog_maintenance::<()>(&path, || Ok(CatalogMaintenanceAttempt::Busy)).unwrap();
    drop(owner.checkout(&path).unwrap());
    assert_eq!(owner.connection_counts(), (1, 0));
    with_catalog_maintenance(&path, || Ok(CatalogMaintenanceAttempt::Completed(()))).unwrap();
    drop(owner.checkout(&path).unwrap());
    assert_eq!(owner.connection_counts(), (2, 1));
    owner.retire().unwrap();
}

#[test]
fn poll_catalog_failed_checkout_keeps_its_epoch_catalog() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let other = directory.path().join("other.sqlite3");
    let owner = PollCatalogOwner::default();
    drop(owner.checkout(&path).unwrap());
    let error = match owner.checkout(&other) {
        Err(error) => error,
        Ok(_) => panic!("a runtime cannot change its catalog"),
    };
    assert_eq!(error.code, "library_synchronization_catalog_changed");
    assert!(!other.exists());
    drop(owner.checkout(&path).unwrap());
    assert_eq!(owner.connection_counts(), (1, 0));
    owner.retire().unwrap();
}

#[test]
fn poll_catalog_failed_replacement_preserves_the_epoch_path_binding() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let other = directory.path().join("other.sqlite3");
    let owner = PollCatalogOwner::default();
    drop(owner.checkout(&path).unwrap());
    crate::application::catalog_session::invalidate_catalog_session(&path).unwrap();
    let damaged_path = path.clone();
    owner.before_close(move || {
        rusqlite::Connection::open(damaged_path)
            .unwrap()
            .execute_batch("DROP TABLE schema_info")
            .unwrap();
    });
    assert!(owner.checkout(&path).is_err());
    assert_eq!(owner.connection_counts(), (1, 1));
    let error = match owner.checkout(&other) {
        Err(error) => error,
        Ok(_) => panic!("failed replacement must not erase the epoch binding"),
    };
    assert_eq!(error.code, "library_synchronization_catalog_changed");
    assert!(!other.exists());
    owner.retire().unwrap();
}

#[test]
fn poll_catalog_windows_handle_blocks_replacement_only_until_explicit_retirement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ame.sqlite3");
    let moved = directory.path().join("retired.sqlite3");
    let owner = PollCatalogOwner::default();
    drop(owner.checkout(&path).unwrap());
    assert!(std::fs::rename(&path, &moved).is_err());
    assert!(path.exists());
    owner.retire().unwrap();
    std::fs::rename(&path, &moved).unwrap();
    SqliteCatalogSession::validate(path.clone()).unwrap();
    let next = PollCatalogOwner::default();
    drop(next.checkout(&path).unwrap());
    next.retire().unwrap();
    assert!(moved.exists());
    assert!(path.exists());
}
