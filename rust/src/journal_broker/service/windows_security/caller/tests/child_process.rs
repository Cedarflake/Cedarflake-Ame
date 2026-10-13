use std::fs;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

use super::open_directory_handle;

const CHILD_DIRECTORY: &str = "CEDARFLAKE_AME_CALLER_LIFETIME_FIXTURE";
const CHILD_TEST: &str = concat!(
    "journal_broker::service::windows_security::caller::tests::",
    "held_caller_rejects_operations_after_the_process_exits"
);
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const PARENT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) struct OwnedCallerChild {
    child: Child,
    directory: tempfile::TempDir,
}

impl OwnedCallerChild {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().expect("generated caller fixture");
        fs::create_dir(directory.path().join("root")).expect("generated caller root");
        let child = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", CHILD_TEST, "--test-threads=1"])
            .env(CHILD_DIRECTORY, directory.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("owned caller fixture process");
        Self { child, directory }
    }

    pub(super) fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn wait_for_root_handle(&mut self) -> u64 {
        let deadline = Instant::now() + PARENT_TIMEOUT;
        let ready = self.directory.path().join("ready");
        loop {
            assert!(
                self.child.try_wait().expect("child status").is_none(),
                "caller child exited before publishing its root handle"
            );
            match fs::read_to_string(&ready) {
                Ok(value) => return value.parse().expect("child root handle value"),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("read child root handle: {error}"),
            }
            assert!(Instant::now() < deadline, "caller child readiness deadline");
            thread::sleep(POLL_INTERVAL);
        }
    }

    pub(super) fn request_exit_and_wait(&mut self) {
        fs::write(self.directory.path().join("exit"), []).expect("request child exit");
        let deadline = Instant::now() + PARENT_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().expect("child retirement status") {
                assert!(status.success(), "caller child failed: {status}");
                return;
            }
            assert!(
                Instant::now() < deadline,
                "caller child retirement deadline"
            );
            thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for OwnedCallerChild {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = self.child.kill();
        let deadline = Instant::now() + PARENT_TIMEOUT;
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(POLL_INTERVAL);
        }
        if thread::panicking() {
            eprintln!("owned caller child did not retire before cleanup deadline");
        } else {
            panic!("owned caller child did not retire before cleanup deadline");
        }
    }
}

pub(super) fn run_child_if_requested() -> bool {
    let Some(directory) = std::env::var_os(CHILD_DIRECTORY) else {
        return false;
    };
    let directory = std::path::PathBuf::from(directory);
    let root = directory.join("root");
    let path = root.to_str().expect("UTF-8 generated root");
    let handle = open_directory_handle(&path.encode_utf16().collect::<Vec<_>>())
        .expect("child-owned root handle");
    let pending = directory.join("ready.tmp");
    fs::write(&pending, (handle.raw() as usize as u64).to_string()).expect("child ready record");
    fs::rename(pending, directory.join("ready")).expect("publish complete ready record");
    let deadline = Instant::now() + CHILD_TIMEOUT;
    let exit = directory.join("exit");
    loop {
        if exit.try_exists().expect("child exit request") {
            return true;
        }
        assert!(
            Instant::now() < deadline,
            "caller child self-retirement deadline"
        );
        thread::sleep(POLL_INTERVAL);
    }
}
