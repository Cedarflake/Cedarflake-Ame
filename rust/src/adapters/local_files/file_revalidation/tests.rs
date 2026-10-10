use std::fs;

use tempfile::tempdir;

use super::*;
use crate::adapters::local_files::{
    file_source_evidence, path_text, reset_source_content_open_instrumentation,
    source_content_open_count,
};

fn source_state(path: &Path) -> (ExpectedFileState, CheckedDirectoryEntry) {
    let metadata = path.symlink_metadata().expect("generated source metadata");
    let (identity, revision) = file_source_evidence(path).expect("generated source evidence");
    let expected = ExpectedFileState {
        absolute_path: path_text(path),
        file_size: metadata.len(),
        modified_unix_ms: modified_unix_ms(&metadata),
        file_identity: identity,
        source_revision: Some(revision),
    };
    let entry = checked_directory_entry_from_metadata(String::new(), path, metadata)
        .expect("generated source admission");
    (expected, entry)
}

#[test]
fn revalidation_accepts_unchanged_evidence_without_reading_source_content() {
    let directory = tempdir().expect("generated source directory");
    let source = directory.path().join("图片.bin");
    fs::write(&source, b"retained source bytes").expect("generated source");
    let (expected, _) = source_state(&source);
    let root = path_text(directory.path());
    reset_source_content_open_instrumentation(&root);

    revalidate_file_state(&expected).expect("unchanged source evidence");

    assert_eq!(source_content_open_count(&root), 0);
    assert_eq!(fs::read(&source).unwrap(), b"retained source bytes");
}

#[test]
fn revalidation_checks_current_handle_metadata_after_the_path_precheck() {
    let directory = tempdir().expect("generated source directory");
    let source = directory.path().join("source.bin");
    fs::write(&source, b"before").expect("generated source");
    let (mut expected, entry) = source_state(&source);
    expected.file_identity = None;
    expected.source_revision = None;
    fs::write(&source, b"changed after the path precheck").expect("generated source edit");

    let error = revalidate_file_state_with_metadata(&expected, &source, &entry)
        .expect_err("stale path metadata must not admit a changed source");

    assert_eq!(error.code, "source_changed_during_scan");
    assert_eq!(
        fs::read(&source).unwrap(),
        b"changed after the path precheck"
    );
}

#[test]
fn revalidation_preserves_identity_and_revision_open_failure_classification() {
    let directory = tempdir().expect("generated source directory");
    let source = directory.path().join("source.bin");
    fs::write(&source, b"before").expect("generated source");
    let (mut expected, entry) = source_state(&source);
    fs::remove_file(&source).expect("remove only the generated fixture");

    let with_identity = revalidate_file_state_with_metadata(&expected, &source, &entry)
        .expect_err("missing source identity");
    assert_eq!(with_identity.code, "source_identity_unavailable");
    expected.file_identity = None;
    let without_identity = revalidate_file_state_with_metadata(&expected, &source, &entry)
        .expect_err("missing source revision");
    assert_eq!(without_identity.code, "source_revision_unavailable");
}

#[test]
fn revalidation_rejects_placeholder_evidence_before_opening_the_source() {
    let directory = tempdir().expect("generated source directory");
    let source = directory.path().join("source.bin");
    fs::write(&source, b"before").expect("generated source");
    let (expected, mut entry) = source_state(&source);
    entry.placeholder_state = MetadataInventoryPlaceholderState::RecallOnDataAccess;
    fs::remove_file(&source).expect("remove only the generated fixture");

    let error = revalidate_file_state_with_metadata(&expected, &source, &entry)
        .expect_err("placeholder state takes precedence over the inaccessible path");

    assert_eq!(error.code, "source_became_unavailable");
}
