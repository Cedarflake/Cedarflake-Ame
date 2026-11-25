use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use super::*;

struct Fixture {
    root: tempfile::TempDir,
    process: Option<HandoffProcess>,
}

impl Fixture {
    fn new() -> Self {
        let scratch = Path::new(env!("CARGO_MANIFEST_DIR")).join("../build/test/derived/temporary");
        fs::create_dir_all(&scratch).unwrap();
        let root = tempfile::tempdir_in(&scratch).unwrap();
        let directory = tempfile::tempdir_in(root.path()).unwrap();
        let executable = directory.path().join("waiting-helper.exe");
        let system = std::env::var_os("SystemRoot").unwrap();
        fs::copy(Path::new(&system).join("System32/cmd.exe"), &executable).unwrap();
        let child = super::super::hidden_command(&executable)
            .args(["/d", "/q"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            root,
            process: Some(HandoffProcess::new(child, directory)),
        }
    }

    fn process(&mut self) -> &mut HandoffProcess {
        self.process.as_mut().unwrap()
    }

    fn acknowledge(&mut self, purpose: Purpose) {
        let process = self.process();
        fs::write(process.directory().join("ready"), b"ready").unwrap();
        process.acknowledge(purpose).unwrap();
    }

    fn directory(&mut self) -> PathBuf {
        self.process().directory().to_owned()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(process) = self.process.as_mut() {
            process.authority = Authority::Retired;
        }
        drop(self.process.take());
    }
}

#[test]
fn failed_prepare_retires_helper_before_removing_its_directory() {
    let mut fixture = Fixture::new();
    let directory = fixture.directory();
    drop(fixture.process.take());
    assert!(!directory.exists());
}

#[test]
fn acknowledged_helper_survives_disposal_without_authorizing_replacement() {
    let mut fixture = Fixture::new();
    fixture.acknowledge(Purpose::Update);
    let directory = fixture.directory();
    let mut input = fixture.process().child.stdin.take().unwrap();
    drop(fixture.process.take());
    assert!(directory.is_dir());
    assert!(!directory.join("commit").exists());
    // The pipe remains writable only while the detached child is still alive.
    input.write_all(b"exit\r\n").unwrap();
    drop(input);
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::remove_file(directory.join("waiting-helper.exe")).is_err() {
        assert!(Instant::now() < deadline, "Detached child did not retire");
        std::thread::sleep(Duration::from_millis(20));
    }
    fs::remove_dir_all(&directory).unwrap();
    assert!(fixture.root.path().is_dir());
}

#[test]
fn activation_rejects_dead_helper_without_publishing_commit() {
    let mut fixture = Fixture::new();
    fixture.acknowledge(Purpose::Update);
    retire(&mut fixture.process().child);
    assert!(matches!(
        fixture.process().activate().unwrap(),
        Activation::RecoveryRequired
    ));
    assert!(!fixture.directory().join("commit").exists());
}

#[test]
fn recovery_replacement_cannot_activate_a_package_and_retires_dead_preparation() {
    let mut original = Fixture::new();
    original.acknowledge(Purpose::Update);
    let abandoned = original.directory();
    retire(&mut original.process().child);
    let mut recovery = Fixture::new();
    recovery.acknowledge(Purpose::Recovery);
    original
        .process()
        .replace_with_recovery(recovery.process.take().unwrap());
    assert!(!abandoned.exists());
    assert!(matches!(
        original.process().activate().unwrap(),
        Activation::RecoveryArmed
    ));
    assert!(!original.directory().join("commit").exists());
    // Retire the child before the fixture owning its containing root is dropped.
    original.process().authority = Authority::Retired;
    drop(original.process.take());
}

#[test]
fn activation_publishes_complete_commit_once_for_live_helper() {
    let mut fixture = Fixture::new();
    fixture.acknowledge(Purpose::Update);
    assert!(matches!(
        fixture.process().activate().unwrap(),
        Activation::Committed
    ));
    assert!(matches!(
        fixture.process().activate().unwrap(),
        Activation::Committed
    ));
    assert_eq!(
        fs::read(fixture.directory().join("commit")).unwrap(),
        b"apply"
    );
    assert!(fixture.process().child.try_wait().unwrap().is_none());
}
