use super::*;

#[test]
fn changed_or_unavailable_sources_wait_without_exhausting_model_retries() {
    for code in [
        "viewer_source_guard_failed",
        "source_became_unavailable",
        "source_changed_during_scan",
        "source_replaced_during_scan",
        "source_revision_changed_during_scan",
        "source_identity_unavailable",
        "source_revision_unavailable",
        "source_revalidation_failed",
        "classification_source_read_failed",
    ] {
        assert!(matches!(
            execution_error(ScanError::new(code, "Source is unavailable")),
            ClassificationExecutionError::Work {
                kind: ClassificationWorkFailure::Unavailable,
                ..
            }
        ));
    }
}

#[test]
fn model_setup_and_unknown_failures_stop_the_run_without_classifying_images_as_bad() {
    for code in [
        "classification_artifact_rejected",
        "classification_runtime_unavailable",
        "unexpected_failure",
    ] {
        match execution_error(ScanError::new(code, "Preparation failed")) {
            ClassificationExecutionError::Blocked(error) => assert_eq!(error.code, code),
            _ => panic!("a setup failure must retain its run-level outcome"),
        }
    }
    assert!(matches!(
        execution_error(ScanError::new(
            "classification_worker_exited",
            "Worker exited"
        )),
        ClassificationExecutionError::Work {
            kind: ClassificationWorkFailure::Transient,
            ..
        }
    ));
    assert!(matches!(
        execution_error(ScanError::new(
            "classification_image_decode_failed",
            "Image is invalid"
        )),
        ClassificationExecutionError::Work {
            kind: ClassificationWorkFailure::Permanent,
            ..
        }
    ));
}
