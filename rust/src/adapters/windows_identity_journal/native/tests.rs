use std::fs;
use std::os::windows::fs::OpenOptionsExt;

use tempfile::tempdir;

use super::*;
use crate::journal_broker::describe_production_persistent_journal_root;

fn reference_for(path: &std::path::Path) -> JournalFileReference {
    let file = open_path(&path.to_string_lossy(), FILE_READ_ATTRIBUTES).unwrap();
    JournalFileReference::from_bytes(&file_info(&file).unwrap().FileId.Identifier).unwrap()
}

#[test]
fn single_link_identity_tracks_current_membership_after_moving_out() {
    let fixture = tempdir().unwrap();
    let source = fixture.path().join("library");
    fs::create_dir(&source).unwrap();
    let file = source.join("single.txt");
    fs::write(&file, b"generated metadata fixture").unwrap();
    let reference = reference_for(&file);
    let registration = describe_production_persistent_journal_root("root", 1, &source).unwrap();
    let root = NativeRoot::open(&registration.authorization).unwrap();

    let locations = root.locations(&reference).unwrap().unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].relative_path, "single.txt");
    fs::rename(&file, fixture.path().join("outside.txt")).unwrap();
    assert!(root.locations(&reference).unwrap().unwrap().is_empty());
}

#[test]
fn hardlinks_retain_every_root_alias_when_the_primary_name_is_outside() {
    let fixture = tempdir().unwrap();
    let source = fixture.path().join("library");
    fs::create_dir(&source).unwrap();
    let primary = fixture.path().join("primary.txt");
    fs::write(&primary, b"generated metadata fixture").unwrap();
    fs::hard_link(&primary, source.join("first.txt")).unwrap();
    fs::hard_link(&primary, source.join("second.txt")).unwrap();
    let reference = reference_for(&primary);
    let registration = describe_production_persistent_journal_root("root", 1, &source).unwrap();
    let root = NativeRoot::open(&registration.authorization).unwrap();

    let locations = root.locations(&reference).unwrap().unwrap();
    assert_eq!(
        locations
            .iter()
            .map(|location| location.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["first.txt", "second.txt"]
    );
    assert_eq!(locations[0].identity, locations[1].identity);
    fs::remove_file(source.join("first.txt")).unwrap();
    let remaining = root.locations(&reference).unwrap().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].relative_path, "second.txt");
    assert_eq!(remaining[0].identity, locations[0].identity);
}

#[test]
fn exclusive_content_lock_still_allows_attribute_only_identity_resolution() {
    let fixture = tempdir().unwrap();
    let path = fixture.path().join("locked.txt");
    fs::write(&path, b"generated metadata fixture").unwrap();
    let reference = reference_for(&path);
    let registration =
        describe_production_persistent_journal_root("root", 1, fixture.path()).unwrap();
    let root = NativeRoot::open(&registration.authorization).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    assert_eq!(fs::File::open(&path).unwrap_err().raw_os_error(), Some(32));
    let while_locked = root.locations(&reference).unwrap().unwrap();
    assert_eq!(while_locked.len(), 1);
    assert_eq!(while_locked[0].relative_path, "locked.txt");
    drop(held);
    let locations = root.locations(&reference).unwrap().unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].relative_path, "locked.txt");
    assert_eq!(locations[0].identity, while_locked[0].identity);
}
