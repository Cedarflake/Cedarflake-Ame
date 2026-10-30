use std::sync::Arc;

use super::*;

fn unresponsive_worker() -> Transport {
    let shell = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut command = Command::new(shell);
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$null = [Console]::In.ReadToEnd()",
    ]);
    Transport::launch(command).unwrap()
}

fn request() -> Frame<Request> {
    Frame {
        message: Request::Image {
            session: 1,
            request: 1,
        },
        payload: vec![0; super::super::manifest::IMAGE_VALUES * 4],
    }
}

#[test]
fn cancellation_can_retire_a_worker_while_pipe_write_is_blocked() {
    let mut transport = unresponsive_worker();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);
    let cancellation = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::Release);
    });
    let result = transport.request(request(), Duration::from_secs(10), &cancelled);
    assert!(matches!(result, Err(RuntimeError::Cancelled)));
    transport.retire().unwrap();
    cancellation.join().unwrap();
    assert!(transport.child.try_wait().unwrap().is_some());
    assert!(transport.io_thread.is_none());
    assert!(transport.is_retired());
    assert!(matches!(
        transport.request(request(), Duration::from_secs(1), &AtomicBool::new(false)),
        Err(RuntimeError::Exited)
    ));
}

#[test]
fn unresponsive_worker_timeout_retains_ownership_until_joined_retirement() {
    let mut transport = unresponsive_worker();
    let result = transport.request(
        request(),
        Duration::from_millis(30),
        &AtomicBool::new(false),
    );
    assert!(matches!(result, Err(RuntimeError::TimedOut)));
    assert!(!transport.is_retired());
    transport.retire().unwrap();
    assert!(transport.child.try_wait().unwrap().is_some());
    transport.retire().unwrap();
}
