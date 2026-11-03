#![cfg(all(windows, feature = "classification-worker"))]

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;

use rust_lib_cedarflake_ame::classification_worker::{
    EncoderKind, RuntimeError, WorkerConfiguration, WorkerSession,
};

#[test]
fn missing_artifacts_return_a_structured_failure_and_retire() {
    let models = tempfile::tempdir().unwrap();
    let runtime = tempfile::tempdir().unwrap();
    let result = WorkerSession::start(
        WorkerConfiguration {
            executable: env!("CARGO_BIN_EXE_cedarflake_ame_classification_worker").into(),
            model_directory: models.path().into(),
            runtime_directory: runtime.path().into(),
            encoder: EncoderKind::Image,
        },
        &AtomicBool::new(false),
    );
    assert!(matches!(result, Err(RuntimeError::ArtifactMismatch)));
}

#[test]
fn cancelled_start_never_launches_the_requested_executable() {
    let result = WorkerSession::start(
        WorkerConfiguration {
            executable: "missing.exe".into(),
            model_directory: "missing-models".into(),
            runtime_directory: "missing-runtime".into(),
            encoder: EncoderKind::Text,
        },
        &AtomicBool::new(true),
    );
    assert!(matches!(result, Err(RuntimeError::Cancelled)));
}

#[test]
fn worker_retires_on_parent_disconnect_without_loading_native_code() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cedarflake_ame_classification_worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take();
    assert!(child.wait().unwrap().success());
}

#[test]
fn malformed_start_retires_without_reading_unbounded_payload() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cedarflake_ame_classification_worker"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(&u32::MAX.to_le_bytes()).unwrap();
    input.write_all(&0_u32.to_le_bytes()).unwrap();
    drop(input);
    assert!(!child.wait().unwrap().success());
}
