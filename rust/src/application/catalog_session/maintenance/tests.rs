use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tempfile::tempdir;

use super::*;
use crate::application::catalog_session::{
    invalidate_catalog_session, open_catalog_reader, validated_catalog_session,
};

#[test]
fn validation_in_flight_defers_structural_maintenance_without_waiting() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    crate::application::catalog_session::validated_catalog_session_with(&path, None, |path| {
        assert!(matches!(
            with_catalog_maintenance::<()>(&path, || panic!("validation overlap"))
                .expect("validation owns admission"),
            CatalogMaintenanceAttempt::Busy
        ));
        crate::adapters::SqliteCatalogSession::validate(path)
    })
    .expect("validation continues independently");
    assert!(matches!(
        with_catalog_maintenance(&path, || Ok(CatalogMaintenanceAttempt::Completed(())))
            .expect("retired validation releases admission"),
        CatalogMaintenanceAttempt::Completed(())
    ));
}

#[test]
fn structural_attempt_outcomes_retire_proof_except_busy() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    for outcome in ["completed", "interrupted", "failed", "busy", "panic"] {
        let previous = validated_catalog_session(&path).expect("prime session");
        crate::adapters::reset_full_schema_validation_count(&path);
        let result = catch_unwind(AssertUnwindSafe(|| {
            with_catalog_maintenance(&path, || match outcome {
                "completed" => Ok(CatalogMaintenanceAttempt::Completed(())),
                "interrupted" => Ok(CatalogMaintenanceAttempt::Interrupted),
                "failed" => Err(ScanError::new("injected_failure", "injected")),
                "busy" => Ok(CatalogMaintenanceAttempt::Busy),
                _ => panic!("injected maintenance unwind"),
            })
        }));
        match outcome {
            "failed" => assert_eq!(
                result.expect("not panicked").err().expect("failure").code,
                "injected_failure"
            ),
            "panic" => assert!(result.is_err()),
            _ => {
                result.expect("not panicked").expect("maintenance outcome");
            }
        }
        let current = validated_catalog_session(&path).expect("session can reopen");
        assert_eq!(
            Arc::ptr_eq(&previous, &current),
            outcome == "busy",
            "{outcome}"
        );
        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            usize::from(outcome != "busy"),
            "{outcome}"
        );
        drop(open_catalog_reader(&path).expect("readable after each outcome"));
    }
}

#[test]
fn invalidation_during_busy_maintenance_cannot_restore_stale_proof() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let previous = validated_catalog_session(&path).expect("prime session");
    assert!(matches!(
        with_catalog_maintenance::<()>(&path, || {
            invalidate_catalog_session(&path)?;
            Ok(CatalogMaintenanceAttempt::Busy)
        })
        .expect("busy outcome"),
        CatalogMaintenanceAttempt::Busy
    ));
    let current = validated_catalog_session(&path).expect("renew invalidated session");
    assert!(!Arc::ptr_eq(&previous, &current));
}

#[test]
fn maintenance_waiters_resume_and_other_catalogs_remain_available() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let other = directory.path().join("other.sqlite3");
    let previous = validated_catalog_session(&path).expect("prime session");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    thread::scope(|scope| {
        let maintenance_path = &path;
        let worker = scope.spawn(move || {
            with_catalog_maintenance(maintenance_path, || {
                entered_tx.send(()).expect("entered");
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("release maintenance");
                Ok(CatalogMaintenanceAttempt::Completed(()))
            })
        });
        entered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("maintenance admitted");
        assert!(matches!(
            with_catalog_maintenance::<()>(&path, || panic!("overlap"))
                .expect("second maintenance defers"),
            CatalogMaintenanceAttempt::Busy
        ));
        drop(open_catalog_reader(&other).expect("unrelated catalog is not locked"));
        let entry = catalog_session_entry(&path).expect("entry");
        let flight = match &*entry.state.lock().expect("short state lock") {
            CatalogSessionState::Maintaining(flight) => Arc::clone(flight),
            _ => panic!("maintenance owns the transition"),
        };
        let waiter = scope.spawn(move || {
            flight.wait().expect("maintenance waiter wakes");
        });
        release_tx.send(()).expect("release");
        assert!(matches!(
            worker.join().expect("worker").expect("maintenance"),
            CatalogMaintenanceAttempt::Completed(())
        ));
        waiter.join().expect("waiter");
    });
    assert!(!Arc::ptr_eq(
        &previous,
        &validated_catalog_session(&path).expect("renew proof")
    ));
}
