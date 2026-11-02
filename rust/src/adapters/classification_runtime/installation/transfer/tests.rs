use std::io::{Cursor, Error, ErrorKind};

use super::super::download::DownloadBody;
use super::*;

const ARTIFACT: Artifact = Artifact {
    name: "测试.onnx",
    length: 3,
    digest: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
};

struct InterruptedBody {
    delivered: bool,
}
impl Read for InterruptedBody {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.delivered {
            return Err(Error::new(
                ErrorKind::ConnectionReset,
                "Fixture interruption",
            ));
        }
        bytes[..2].copy_from_slice(b"ab");
        self.delivered = true;
        Ok(2)
    }
}

struct Downloads {
    offsets: Vec<u64>,
    interrupt: bool,
    ignore_range: bool,
}
impl ModelDownload for Downloads {
    fn open(&mut self, _: &Artifact, offset: u64) -> Result<DownloadBody, ScanError> {
        self.offsets.push(offset);
        if self.interrupt {
            return Ok(DownloadBody {
                reader: Box::new(InterruptedBody { delivered: false }),
                offset,
            });
        }
        let offset = if self.ignore_range { 0 } else { offset };
        Ok(DownloadBody {
            reader: Box::new(Cursor::new(&b"abc"[offset as usize..])),
            offset,
        })
    }
}

#[test]
fn interruption_reopens_same_partial_and_requests_only_the_missing_range() {
    let root = tempfile::tempdir().unwrap();
    let cancelled = AtomicBool::new(false);
    let mut download = Downloads {
        offsets: vec![],
        interrupt: true,
        ignore_range: false,
    };
    assert_eq!(
        download_artifact(
            root.path(),
            &ARTIFACT,
            &mut download,
            &cancelled,
            &mut |_| {}
        )
        .unwrap_err()
        .code,
        "classification_download_interrupted"
    );
    assert_eq!(
        std::fs::read(partial_path(root.path(), &ARTIFACT)).unwrap(),
        b"ab"
    );
    download.interrupt = false;
    let mut progress = vec![];
    download_artifact(
        root.path(),
        &ARTIFACT,
        &mut download,
        &cancelled,
        &mut |bytes| progress.push(bytes),
    )
    .unwrap();
    assert_eq!(download.offsets, [0, 2]);
    assert_eq!(progress.first(), Some(&2));
    assert_eq!(
        std::fs::read(root.path().join(ARTIFACT.name)).unwrap(),
        b"abc"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn server_ignoring_range_restarts_only_this_partial_and_rechecks_complete_hash() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(partial_path(root.path(), &ARTIFACT), b"ab").unwrap();
    std::fs::write(root.path().join("retained.onnx"), b"keep").unwrap();
    let mut download = Downloads {
        offsets: vec![],
        interrupt: false,
        ignore_range: true,
    };
    download_artifact(
        root.path(),
        &ARTIFACT,
        &mut download,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(download.offsets, [2]);
    assert_eq!(
        std::fs::read(root.path().join(ARTIFACT.name)).unwrap(),
        b"abc"
    );
    assert_eq!(
        std::fs::read(root.path().join("retained.onnx")).unwrap(),
        b"keep"
    );
}

#[test]
fn complete_staging_is_verified_without_network_and_invalid_staging_is_restarted() {
    for bytes in [b"abc".as_slice(), b"abd", b"oversized"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(partial_path(root.path(), &ARTIFACT), bytes).unwrap();
        let mut download = Downloads {
            offsets: vec![],
            interrupt: false,
            ignore_range: false,
        };
        download_artifact(
            root.path(),
            &ARTIFACT,
            &mut download,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        if bytes == b"abc" {
            assert!(download.offsets.is_empty());
        } else {
            assert_eq!(download.offsets, [0]);
        }
        assert_eq!(
            std::fs::read(root.path().join(ARTIFACT.name)).unwrap(),
            b"abc"
        );
    }
}

#[test]
fn resumed_corrupt_prefix_cannot_publish_and_destination_is_never_replaced() {
    let root = tempfile::tempdir().unwrap();
    let partial = partial_path(root.path(), &ARTIFACT);
    std::fs::write(&partial, b"zz").unwrap();
    let mut download = Downloads {
        offsets: vec![],
        interrupt: false,
        ignore_range: false,
    };
    assert_eq!(
        download_artifact(
            root.path(),
            &ARTIFACT,
            &mut download,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .unwrap_err()
        .code,
        "classification_download_mismatch"
    );
    assert!(!root.path().join(ARTIFACT.name).exists());
    assert_eq!(std::fs::metadata(&partial).unwrap().len(), 0);
    std::fs::write(root.path().join(ARTIFACT.name), b"old").unwrap();
    assert!(
        download_artifact(
            root.path(),
            &ARTIFACT,
            &mut download,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    assert_eq!(
        std::fs::read(root.path().join(ARTIFACT.name)).unwrap(),
        b"old"
    );
    assert_eq!(std::fs::read(partial).unwrap(), b"abc");
}
