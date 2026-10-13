use tempfile::tempdir;

use super::*;
use crate::adapters::local_files::{
    file_identity_evidence, reset_source_content_open_instrumentation, source_content_open_count,
};

#[test]
fn complete_enumeration_retains_root_and_empty_directory_identities_without_media_reads() {
    let root = tempdir().expect("owned source");
    fs::create_dir(root.path().join("empty")).expect("empty child");
    fs::write(root.path().join("image.png"), b"unread source bytes").expect("owned image");
    let root_path = root.path().to_string_lossy().into_owned();
    let discovery = FileDiscovery::new(&root_path).expect("discovery");
    reset_source_content_open_instrumentation(&root_path);
    for relative_directory in ["", "empty"] {
        let mut entries = discovery
            .entry_paths_in_directory(relative_directory)
            .expect("directory enumeration");
        let paths = entries.by_ref().collect::<Vec<_>>();
        assert_eq!(
            paths.len(),
            if relative_directory.is_empty() { 2 } else { 0 }
        );
        let identity = entries.finish().expect("completed directory binding");
        assert_eq!(
            identity,
            file_identity_evidence(&root.path().join(relative_directory)).expect("identity oracle")
        );
        assert!(identity.is_some());
    }
    assert_eq!(source_content_open_count(&root_path), 0);
}

#[test]
fn partial_enumeration_cannot_be_retained_as_directory_coverage() {
    let root = tempdir().expect("owned source");
    fs::write(root.path().join("image.png"), b"fixture").expect("owned image");
    let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
    let mut entries = discovery.entry_paths_in_directory("").expect("enumeration");
    assert!(entries.next().is_some());
    let error = entries
        .finish()
        .expect_err("iterator has not reached its terminal boundary");
    assert_eq!(error.code, "directory_enumeration_incomplete");
}

#[test]
fn replaced_directory_does_not_publish_the_previous_owners_identity() {
    let root = tempdir().expect("owned source");
    let original = root.path().join("album");
    fs::create_dir(&original).expect("owned directory");
    let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
    let mut entries = discovery
        .entry_paths_in_directory("album")
        .expect("enumeration");
    assert!(entries.next().is_none());
    fs::rename(&original, root.path().join("moved")).expect("move the owned directory");
    fs::create_dir(&original).expect("different owner at the same path");
    let error = entries.finish().expect_err("changed directory binding");
    assert_eq!(error.code, "directory_identity_changed");
}
