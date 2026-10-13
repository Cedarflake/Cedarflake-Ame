use super::*;

#[test]
fn unsuccessful_incremental_attempts_retire_proof_except_busy() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    for outcome in ["interrupted", "failed", "busy", "panic"] {
        let previous = validated_catalog_session(&path).expect("prime session");
        crate::adapters::reset_full_schema_validation_count(&path);
        let result = catch_unwind(AssertUnwindSafe(|| {
            with_reserved_catalog_maintenance::<()>(
                &path,
                CatalogMaintenanceKind::IncrementalReclamation,
                || match outcome {
                    "interrupted" => Ok(CatalogMaintenanceAttempt::Interrupted),
                    "failed" => Err(ScanError::new("injected_failure", "injected")),
                    "busy" => Ok(CatalogMaintenanceAttempt::Busy),
                    _ => panic!("injected maintenance unwind"),
                },
            )
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
        assert_eq!(Arc::ptr_eq(&previous, &current), outcome == "busy");
        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            usize::from(outcome != "busy")
        );
        drop(open_catalog_reader(&path).expect("readable after each outcome"));
    }
}

#[test]
fn successful_incremental_attempt_cannot_restore_an_invalidated_proof() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    let previous = validated_catalog_session(&path).expect("prime session");
    assert!(matches!(
        with_reserved_catalog_maintenance(
            &path,
            CatalogMaintenanceKind::IncrementalReclamation,
            || {
                invalidate_catalog_session(&path)?;
                Ok(CatalogMaintenanceAttempt::Completed(()))
            }
        )
        .expect("completed outcome"),
        CatalogMaintenanceAttempt::Completed(())
    ));
    let current = validated_catalog_session(&path).expect("renew invalidated session");
    assert!(!Arc::ptr_eq(&previous, &current));
}
