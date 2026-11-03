use std::fs;
use std::io::Cursor;
use std::time::UNIX_EPOCH;

use crate::domain::classification::ClassificationRecordId;
use crate::domain::{AssetLocationView, IncrementalCatalogRoot, PreviewStatus};

use super::*;

fn fixture() -> (
    tempfile::TempDir,
    FingerprintCandidate,
    ClassificationContent,
) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    fs::create_dir(root.join("图片")).unwrap();
    let path = root.join("图片").join("content.wrong-extension");
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(16, 8)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    fs::write(&path, bytes.get_ref()).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let candidate = FingerprintCandidate {
        location: AssetLocationView {
            asset_id: "asset".into(),
            location_id: "location".into(),
            root_id: "root".into(),
            scan_id: "scan".into(),
            absolute_path: path.to_string_lossy().into_owned(),
            display_path: String::new(),
            relative_path: "图片/content.wrong-extension".into(),
            preview_path: String::new(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: metadata
                .modified()
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            file_identity: super::super::local_files::file_identity_evidence(&path).unwrap(),
            source_revision: None,
            source_generation: 1,
            width: 16,
            height: 8,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: String::new(),
            metadata_engine_version: String::new(),
            camera: Default::default(),
            capture_time: None,
        },
        root: IncrementalCatalogRoot {
            root_id: "root".into(),
            root_path: root.to_string_lossy().into_owned(),
            root_generation: crate::domain::LibraryRootGeneration::initial(),
            active_scan_id: Some("scan".into()),
            has_running_scan: false,
            catalog_revision: 1,
            last_consistency_audit_unix_ms: None,
            publication_root_identity: super::super::local_files::file_identity_evidence(&root)
                .unwrap(),
        },
    };
    let content = ClassificationContent {
        scheme: ClassificationRecordId::new(EXACT_FINGERPRINT_SCHEME.into()).unwrap(),
        digest: *blake3::hash(bytes.get_ref()).as_bytes(),
        byte_length: metadata.len(),
    };
    (directory, candidate, content)
}

#[test]
fn original_and_namespace_remain_read_only_until_the_analysis_lease_drops() {
    let (directory, candidate, content) = fixture();
    let path = Path::new(&candidate.location.absolute_path);
    let original = fs::read(path).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut lease = GuardedClassificationImageReader
        .open(&candidate, &content, &cancelled)
        .unwrap();
    assert!(fs::OpenOptions::new().write(true).open(path).is_err());
    assert!(fs::rename(path, path.with_extension("moved")).is_err());
    assert!(
        fs::rename(
            directory.path().join("图片"),
            directory.path().join("changed")
        )
        .is_err()
    );
    assert_eq!(
        lease.prepare_pixels(&cancelled).unwrap().len(),
        3 * 224 * 224
    );
    lease.revalidate().unwrap();
    drop(lease);
    assert_eq!(fs::read(path).unwrap(), original);
    fs::rename(path, path.with_extension("moved")).unwrap();
}

#[test]
fn uncertain_revision_requires_exact_content_and_rejects_wrong_roots() {
    let (_directory, mut candidate, mut content) = fixture();
    let cancelled = AtomicBool::new(false);
    content.digest[0] ^= 1;
    let error = GuardedClassificationImageReader
        .open(&candidate, &content, &cancelled)
        .err()
        .unwrap();
    assert_eq!(error.code, "classification_source_changed");
    content.digest[0] ^= 1;
    candidate
        .root
        .publication_root_identity
        .as_mut()
        .unwrap()
        .value = "different".into();
    assert!(
        GuardedClassificationImageReader
            .open(&candidate, &content, &cancelled)
            .is_err()
    );
}
