use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::*;

#[derive(Default)]
struct Observer {
    stop: AtomicBool,
    received: AtomicU64,
    stop_after: Option<u64>,
}

impl ApplicationDownloadObserver for Observer {
    fn cancelled(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    fn received(&self, bytes: u64) {
        self.received.store(bytes, Ordering::Relaxed);
        if self.stop_after.is_some_and(|limit| bytes >= limit) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }
    fn verifying(&self) {}
}

fn package(directory: &Path) -> RetainedPackage {
    RetainedPackage {
        asset: PackageAsset {
            file_name: "release.msi".into(),
            size: 7,
            sha256: format!("{:x}", Sha256::digest(b"package")),
        },
        path: directory.join("release.msi"),
        url: "https://github.com/Cedarflake/Cedarflake-Ame/invalid-fixture".into(),
        transport: UpdateTransport::new().unwrap(),
    }
}

#[test]
fn retained_package_is_rehashed_and_leased_against_writers() {
    let directory = tempfile::tempdir().unwrap();
    let package = package(directory.path());
    fs::write(&package.path, b"package").unwrap();
    let observer = Observer::default();
    package.download(&observer).unwrap();
    assert_eq!(observer.received.load(Ordering::Relaxed), 7);
    let lease = package.verified_file(&observer).unwrap();
    assert!(OpenOptions::new().write(true).open(&package.path).is_err());
    assert!(fs::remove_file(&package.path).is_err());
    drop(lease);
    fs::write(&package.path, b"changed").unwrap();
    assert_eq!(
        package.verified_file(&observer).unwrap_err().code,
        ApplicationUpdateFailureCode::DamagedDownload
    );
}

#[test]
fn interrupted_complete_partial_is_verified_before_atomic_publication() {
    let directory = tempfile::tempdir().unwrap();
    let package = package(directory.path());
    let partial = package.path.with_extension("partial");
    fs::write(&partial, b"package").unwrap();
    let observer = Observer::default();
    observer.stop.store(true, Ordering::Relaxed);
    assert_eq!(
        package.download(&observer).unwrap_err().code,
        ApplicationUpdateFailureCode::Cancelled
    );
    assert!(!package.path.exists());
    assert_eq!(fs::read(&partial).unwrap(), b"package");
    observer.stop.store(false, Ordering::Relaxed);
    package.download(&observer).unwrap();
    assert_eq!(fs::read(&package.path).unwrap(), b"package");
    assert!(!partial.exists());
}

#[test]
fn corrupt_partial_is_retired_without_publishing_ready_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let package = package(directory.path());
    let partial = package.path.with_extension("partial");
    fs::write(&partial, b"changed").unwrap();
    assert_eq!(
        package.download(&Observer::default()).unwrap_err().code,
        ApplicationUpdateFailureCode::DamagedDownload
    );
    assert!(!package.path.exists());
    assert!(!partial.exists());
}

#[test]
fn an_existing_partial_hardlink_cannot_modify_its_unrelated_file() {
    let directory = tempfile::tempdir().unwrap();
    let package = package(directory.path());
    let unrelated = directory.path().join("retained-original.bin");
    fs::write(&unrelated, b"irreplaceable original content").unwrap();
    fs::hard_link(&unrelated, package.path.with_extension("partial")).unwrap();
    assert_eq!(
        package.download(&Observer::default()).unwrap_err().code,
        ApplicationUpdateFailureCode::Storage
    );
    assert_eq!(
        fs::read(unrelated).unwrap(),
        b"irreplaceable original content"
    );
    assert!(!package.path.exists());
}

#[test]
fn active_partial_holds_its_file_and_parent_namespace_until_publication() {
    let directory = tempfile::tempdir().unwrap();
    let cache = directory.path().join("cache");
    fs::create_dir(&cache).unwrap();
    let partial = cache.join("release.partial");
    let (file, namespace) = storage_lease::open_partial(&partial).unwrap();
    assert!(fs::rename(&cache, directory.path().join("replaced")).is_err());
    assert!(OpenOptions::new().write(true).open(&partial).is_err());
    drop(file);
    drop(namespace);
    fs::rename(&cache, directory.path().join("replaced")).unwrap();
}

#[path = "http_fixture.rs"]
mod http_fixture;

#[test]
fn interrupted_transfer_resumes_exact_bytes_before_publication() {
    let directory = tempfile::tempdir().unwrap();
    let body = vec![42; 256 * 1024];
    let server = http_fixture::Server::start(
        body.clone(),
        vec![
            http_fixture::Reply::Interrupt(8192),
            http_fixture::Reply::Range,
        ],
    );
    let mut package = package(directory.path());
    package.asset.size = body.len() as u64;
    package.asset.sha256 = format!("{:x}", Sha256::digest(&body));
    package.transport = UpdateTransport::fixture(server.address);
    let observer = Observer::default();
    assert_eq!(
        package.download(&observer).unwrap_err().code,
        ApplicationUpdateFailureCode::Network
    );
    assert!(!package.path.exists());
    let partial = package.path.with_extension("partial");
    assert_eq!(fs::metadata(&partial).unwrap().len(), 8192);
    package.download(&observer).unwrap();
    assert_eq!(fs::read(&package.path).unwrap(), body);
    assert!(!partial.exists());
    assert_eq!(server.join(), vec![None, Some(8192)]);
}

#[test]
fn cancelled_transfer_keeps_resume_bytes_and_accepts_a_full_server_restart() {
    let directory = tempfile::tempdir().unwrap();
    let body = vec![7; 2 * 1024 * 1024];
    let server = http_fixture::Server::start(
        body.clone(),
        vec![http_fixture::Reply::Full, http_fixture::Reply::Full],
    );
    let mut package = package(directory.path());
    package.asset.size = body.len() as u64;
    package.asset.sha256 = format!("{:x}", Sha256::digest(&body));
    package.transport = UpdateTransport::fixture(server.address);
    let observer = Observer {
        stop_after: Some(1),
        ..Observer::default()
    };
    assert_eq!(
        package.download(&observer).unwrap_err().code,
        ApplicationUpdateFailureCode::Cancelled
    );
    assert!(!package.path.exists());
    let length = fs::metadata(package.path.with_extension("partial"))
        .unwrap()
        .len();
    assert!(length > 0 && length < body.len() as u64);
    package.download(&Observer::default()).unwrap();
    assert_eq!(fs::read(&package.path).unwrap(), body);
    assert_eq!(server.join(), vec![None, Some(length)]);
}

#[test]
fn wrong_range_cannot_overwrite_retained_resume_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let server =
        http_fixture::Server::start(b"package".to_vec(), vec![http_fixture::Reply::WrongRange]);
    let mut package = package(directory.path());
    package.transport = UpdateTransport::fixture(server.address);
    let partial = package.path.with_extension("partial");
    fs::write(&partial, b"pack").unwrap();
    assert_eq!(
        package.download(&Observer::default()).unwrap_err().code,
        ApplicationUpdateFailureCode::InvalidRelease
    );
    assert_eq!(fs::read(&partial).unwrap(), b"pack");
    assert!(!package.path.exists());
    assert_eq!(server.join(), vec![Some(4)]);
}
