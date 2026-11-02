use std::io::Cursor;
use std::sync::atomic::Ordering;

use super::*;

const FIRST: Artifact = Artifact {
    name: "first.onnx",
    length: 3,
    digest: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
};
const SECOND: Artifact = Artifact {
    name: "second.onnx",
    ..FIRST
};

struct FixtureDownload {
    calls: Vec<String>,
    bytes: &'static [u8],
    fail_second: bool,
}

impl ModelDownload for FixtureDownload {
    fn open(
        &mut self,
        artifact: &Artifact,
        offset: u64,
    ) -> Result<download::DownloadBody, ScanError> {
        self.calls.push(artifact.name.to_owned());
        if self.fail_second && artifact.name == SECOND.name {
            return Err(ScanError::new(
                "fixture_offline",
                "Connection is unavailable",
            ));
        }
        Ok(download::DownloadBody {
            reader: Box::new(Cursor::new(&self.bytes[offset as usize..])),
            offset,
        })
    }
}

fn fixture() -> FixtureDownload {
    FixtureDownload {
        calls: vec![],
        bytes: b"abc",
        fail_second: false,
    }
}

#[test]
fn complete_installation_keeps_files_and_namespaces_pinned_until_publication() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("模型");
    let mut downloader = fixture();
    let mut updates = Vec::new();
    let installation = prepare_with(
        &models,
        directory.path(),
        &[&FIRST, &SECOND],
        &[],
        &mut downloader,
        &AtomicBool::new(false),
        &mut |progress| updates.push(progress),
    )
    .unwrap();
    assert_eq!(downloader.calls, [FIRST.name, SECOND.name]);
    assert_eq!(
        updates.last().unwrap().phase,
        ModelInstallationPhase::Prepared
    );
    assert_eq!(updates.last().unwrap().completed_bytes, 6);
    assert!(
        updates
            .windows(2)
            .all(|pair| pair[0].completed_bytes <= pair[1].completed_bytes)
    );
    assert!(std::fs::write(models.join(FIRST.name), b"def").is_err());
    assert!(std::fs::remove_file(models.join(SECOND.name)).is_err());
    assert!(std::fs::rename(&models, directory.path().join("moved")).is_err());
    assert_eq!(
        std::fs::read(models.join("NOTICE.txt")).unwrap(),
        NOTICES[1].1
    );
    drop(installation);
    std::fs::write(models.join(FIRST.name), b"def").unwrap();
}

#[test]
fn retry_reuses_only_complete_verified_files_and_preserves_unrelated_files() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("模型");
    let mut downloader = fixture();
    downloader.fail_second = true;
    let cancelled = AtomicBool::new(false);
    assert!(
        prepare_with(
            &models,
            directory.path(),
            &[&FIRST, &SECOND],
            &[],
            &mut downloader,
            &cancelled,
            &mut |_| {}
        )
        .is_err()
    );
    assert!(models.join(FIRST.name).is_file());
    assert!(!models.join(SECOND.name).exists());
    std::fs::write(models.join("retained.txt"), b"retain").unwrap();
    downloader.calls.clear();
    downloader.fail_second = false;
    let prepared = prepare_with(
        &models,
        directory.path(),
        &[&FIRST, &SECOND],
        &[],
        &mut downloader,
        &cancelled,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(downloader.calls, [SECOND.name]);
    assert_eq!(
        std::fs::read(models.join("retained.txt")).unwrap(),
        b"retain"
    );
    drop(prepared);
}

#[test]
fn truncation_tampering_and_oversize_never_publish_a_partial_artifact() {
    for bytes in [b"ab".as_slice(), b"abd", b"abcd"] {
        let directory = tempfile::tempdir().unwrap();
        let models = directory.path().join("models");
        let mut downloader = fixture();
        downloader.bytes = bytes;
        let error = prepare_with(
            &models,
            directory.path(),
            &[&FIRST],
            &[],
            &mut downloader,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .err()
        .unwrap();
        let expected = if bytes.len() < 3 {
            "classification_download_interrupted"
        } else {
            "classification_download_mismatch"
        };
        assert_eq!(error.code, expected);
        assert!(!models.join(FIRST.name).exists());
        assert_eq!(std::fs::read_dir(models).unwrap().count(), 2);
    }
}

#[test]
fn cancellation_during_transfer_preserves_owned_partial_and_allows_retry() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    let cancelled = AtomicBool::new(false);
    let error = prepare_with(
        &models,
        directory.path(),
        &[&FIRST],
        &[],
        &mut fixture(),
        &cancelled,
        &mut |progress| {
            if progress.phase == ModelInstallationPhase::Downloading
                && progress.completed_bytes != 0
            {
                cancelled.store(true, Ordering::Release);
            }
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "classification_cancelled");
    assert_eq!(std::fs::read_dir(&models).unwrap().count(), 2);
    cancelled.store(false, Ordering::Release);
    assert!(
        prepare_with(
            &models,
            directory.path(),
            &[&FIRST],
            &[],
            &mut fixture(),
            &cancelled,
            &mut |_| {}
        )
        .is_ok()
    );
}

#[test]
fn damaged_existing_model_and_bad_runtime_are_rejected_without_network_or_overwrite() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    std::fs::create_dir(&models).unwrap();
    std::fs::write(models.join(FIRST.name), b"bad").unwrap();
    let mut downloader = fixture();
    assert!(
        prepare_with(
            &models,
            directory.path(),
            &[&FIRST],
            &[],
            &mut downloader,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    assert_eq!(std::fs::read(models.join(FIRST.name)).unwrap(), b"bad");
    assert!(downloader.calls.is_empty());
    assert!(
        prepare_with(
            &models,
            directory.path(),
            &[&SECOND],
            &[&FIRST],
            &mut downloader,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    assert!(downloader.calls.is_empty());
}

#[test]
fn cancellation_before_start_and_during_cached_verification_prevents_preparation() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    let cancelled = AtomicBool::new(true);
    let mut downloader = fixture();
    assert!(
        prepare_with(
            &models,
            directory.path(),
            &[&FIRST],
            &[],
            &mut downloader,
            &cancelled,
            &mut |_| {}
        )
        .is_err()
    );
    assert!(!models.exists());
    std::fs::create_dir(&models).unwrap();
    std::fs::write(models.join(FIRST.name), b"abc").unwrap();
    assert!(matches!(
        verify_file_cancellable(&models.join(FIRST.name), &FIRST, &cancelled),
        Err(RuntimeError::Cancelled)
    ));
    assert!(downloader.calls.is_empty());
}
