use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;

use tempfile::tempdir;

use super::*;
use crate::adapters::local_files::{
    reset_source_content_open_instrumentation, source_content_open_count,
};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};

#[test]
fn locked_discovery_admission_retains_available_file_identity_for_exact_retry() {
    for name in ["image.data", "image"] {
        let root = tempdir().expect("isolated source");
        let path = root.path().join(name);
        let bytes = encode_rgb_quadrants(MediaFixtureFormat::Png, 32, 24).expect("valid pixels");
        fs::write(&path, &bytes).expect("owned fixture");
        let modified = fs::metadata(&path)
            .expect("source metadata")
            .modified()
            .expect("mtime");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .expect("exclusive read lock");
        let visit = discovery.visit_relative_path(name);
        let FileVisitOutcome::RetryableFile { file, issue } = visit.outcome else {
            panic!("an available unreadable file needs a typed retry owner");
        };
        assert_eq!(visit.relative_path, name);
        assert_eq!(file.relative_path, name);
        assert_eq!(file.file_size, bytes.len() as u64);
        assert!(file.file_identity.is_some());
        assert_eq!(issue.code, "media_signature_unreadable");
        drop(lock);
        assert!(matches!(
            discovery.visit_relative_path(name).outcome,
            FileVisitOutcome::File(_)
        ));
        assert_eq!(
            fs::metadata(&path)
                .expect("source metadata")
                .modified()
                .expect("mtime"),
            modified
        );
        assert_eq!(fs::read(&path).expect("source bytes"), bytes);
    }
}

#[test]
fn locked_discovery_admission_does_not_probe_or_retry_cloud_placeholders() {
    let root = tempdir().expect("isolated source");
    let path = root.path().join("cloud.data");
    let bytes = b"fixture bytes must never be opened for content";
    fs::write(&path, bytes).expect("owned fixture");
    let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
    let root_path = root.path().to_string_lossy();
    reset_source_content_open_instrumentation(&root_path);
    let visit = discovery.visit_relative_path_with_metadata(
        "cloud.data".to_owned(),
        path.clone(),
        fs::metadata(&path).expect("metadata only"),
        ReparseKind::None,
        MetadataInventoryPlaceholderState::Offline,
        None,
    );
    assert!(
        matches!(visit.outcome, FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped")
    );
    assert_eq!(source_content_open_count(&root_path), 0);
    assert_eq!(fs::read(&path).expect("source bytes"), bytes);
}
