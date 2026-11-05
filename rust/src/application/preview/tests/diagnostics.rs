use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::super::{PreviewStageTimings, log_preview_diagnostic};
use crate::domain::ScanError;

#[test]
fn preview_failure_survives_disconnected_diagnostic_pipe() {
    const CHILD_FLAG: &str = "AME_PREVIEW_DIAGNOSTIC_PIPE_CHILD";
    if std::env::var_os(CHILD_FLAG).is_some() {
        std::io::stdin()
            .read_exact(&mut [0])
            .expect("parent signal");
        std::io::stderr()
            .write_all(b"disconnected diagnostic probe\n")
            .expect_err("parent closed the diagnostic pipe");
        let result = Err(ScanError::new(
            "preview_source_missing",
            "Source is unavailable",
        ));
        log_preview_diagnostic(
            "fixture-location",
            false,
            Duration::ZERO,
            &PreviewStageTimings::default(),
            &result,
        );
        let failure = result.expect_err("source failure is retained");
        assert_eq!(failure.code, "preview_source_missing");
        assert_eq!(failure.message, "Source is unavailable");
        return;
    }

    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "application::preview::tests::diagnostics::preview_failure_survives_disconnected_diagnostic_pipe",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_FLAG, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("diagnostic child");
    drop(child.stderr.take());
    child
        .stdin
        .take()
        .expect("child input")
        .write_all(&[1])
        .expect("release child");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().expect("child status") {
            assert!(
                status.success(),
                "preview diagnostic changed the operation result: {status}"
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("diagnostic child did not finish within its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
