use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;

use tempfile::tempdir;

use super::*;
use crate::adapters::local_files::{
    reset_source_content_open_instrumentation, source_content_open_count,
};
use crate::adapters::media_inspector::LocalMediaInspector;
use crate::domain::ExpectedFileState;
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

#[test]
fn first_content_read_preserves_revision_for_known_and_sniffed_extensions() {
    for aged in [false, true] {
        for name in ["image.png", "image.data"] {
            let root = tempdir().expect("isolated source");
            let path = root.path().join(name);
            let bytes =
                encode_rgb_quadrants(MediaFixtureFormat::Png, 32, 24).expect("valid pixels");
            fs::write(&path, &bytes).expect("owned fixture");
            if aged {
                let earlier =
                    std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 60 * 60);
                let file = OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .expect("owned timestamp fixture");
                file.set_times(
                    fs::FileTimes::new()
                        .set_accessed(earlier)
                        .set_modified(earlier),
                )
                .expect("age only the owned source fixture");
            }
            let before = file_source_evidence(&path).expect("before discovery");
            let discovery =
                FileDiscovery::new(&root.path().to_string_lossy()).expect("production discovery");
            let FileVisitOutcome::File(file) = discovery.visit_relative_path(name).outcome else {
                panic!("both names must admit the same valid PNG");
            };
            let after_discovery = file_source_evidence(&path).expect("after discovery");
            let inspection = LocalMediaInspector::new()
                .inspect_with_discovery(&discovery, &file)
                .expect("production pinned source inspection");
            let after_inspection = file_source_evidence(&path).expect("after inspection");
            eprintln!(
                "controlled first content read name={name} aged={aged} before={} discovered={} inspected={}",
                before.1.value, after_discovery.1.value, after_inspection.1.value,
            );
            assert_eq!((inspection.width, inspection.height), (32, 24));
            assert_eq!(after_discovery, before, "discovery source evidence");
            assert_eq!(after_inspection, before, "inspection source evidence");
            assert_eq!(file.source_revision.as_ref(), Some(&before.1));
            discovery
                .revalidate_relative_file_state(
                    name,
                    &ExpectedFileState {
                        absolute_path: file.absolute_path,
                        file_size: file.file_size,
                        modified_unix_ms: file.modified_unix_ms,
                        file_identity: file.file_identity,
                        source_revision: file.source_revision,
                    },
                )
                .expect("unchanged source passes final publication revalidation");
            assert_eq!(fs::read(&path).expect("source bytes"), bytes);
        }
    }
}
