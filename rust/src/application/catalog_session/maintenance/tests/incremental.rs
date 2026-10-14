use super::*;

#[test]
fn incremental_attempts_preserve_proof_on_busy_or_interruption_only() {
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
        let retains_proof = matches!(outcome, "busy" | "interrupted");
        assert_eq!(Arc::ptr_eq(&previous, &current), retains_proof);
        assert_eq!(
            crate::adapters::full_schema_validation_count(&path),
            usize::from(!retains_proof)
        );
        drop(open_catalog_reader(&path).expect("readable after each outcome"));
    }
}

#[test]
fn completed_or_interrupted_incremental_attempt_cannot_restore_invalidated_proof() {
    let directory = tempdir().expect("catalog directory");
    let path = directory.path().join("catalog.sqlite3");
    for interrupted in [false, true] {
        let previous = validated_catalog_session(&path).expect("prime session");
        with_reserved_catalog_maintenance(
            &path,
            CatalogMaintenanceKind::IncrementalReclamation,
            || {
                invalidate_catalog_session(&path)?;
                if interrupted {
                    Ok(CatalogMaintenanceAttempt::Interrupted)
                } else {
                    Ok(CatalogMaintenanceAttempt::Completed(()))
                }
            },
        )
        .expect("maintenance outcome");
        let current = validated_catalog_session(&path).expect("renew invalidated session");
        assert!(!Arc::ptr_eq(&previous, &current));
    }
}
