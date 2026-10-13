use std::fs;

use tempfile::tempdir;

use super::*;
use crate::adapters::local_files::FileDiscovery;

#[test]
fn directory_entries_keep_physical_identity_after_rename_without_a_file_revision() {
    let source = tempdir().expect("source directory");
    fs::create_dir(source.path().join("old")).expect("empty directory fixture");
    let discovery = FileDiscovery::new(&source.path().to_string_lossy()).expect("discovery");
    let before = discovery
        .metadata_inventory_entry("old")
        .expect("original directory evidence");
    fs::rename(source.path().join("old"), source.path().join("new"))
        .expect("rename directory fixture");
    let after = discovery
        .metadata_inventory_entry("new")
        .expect("renamed directory evidence");

    assert_eq!(after.kind, MetadataInventoryEntryKind::Directory);
    assert!(before.file_identity.is_some());
    assert_eq!(after.file_identity, before.file_identity);
    assert!(after.file_size.is_none());
    assert!(after.source_revision.is_none());
}

#[test]
fn unavailable_or_reparse_directories_do_not_publish_identity_evidence() {
    let source = tempdir().expect("source directory");
    let path = source.path().join("opaque");
    fs::create_dir(&path).expect("directory fixture");
    for (reparse_kind, placeholder_state) in [
        (
            ReparseKind::None,
            MetadataInventoryPlaceholderState::RecallOnDataAccess,
        ),
        (
            ReparseKind::Other,
            MetadataInventoryPlaceholderState::Available,
        ),
    ] {
        let entry = CheckedDirectoryEntry {
            relative_path: "opaque".to_owned(),
            metadata: path.symlink_metadata().expect("directory attributes"),
            reparse_kind,
            placeholder_state,
        };
        let evidence = metadata_inventory_entry_from_checked_directory_entry(entry, &path)
            .expect("opaque directory evidence");

        assert_eq!(evidence.kind, MetadataInventoryEntryKind::Other);
        assert!(evidence.file_identity.is_none());
        assert!(evidence.source_revision.is_none());
    }
}
