use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::{open_catalog_identity_guard, read_catalog_identity};

fn assert_identity_matches_existing_contract(path: &Path) {
    let observed = read_catalog_identity(path).expect("fresh catalog identity");
    let (guard, expected_id) = open_catalog_identity_guard(path).expect("held read guard");
    assert_eq!(observed.canonical_path, fs::canonicalize(path).unwrap());
    assert_eq!(observed.file_identity, expected_id);
    assert!(observed.file_identity.is_some());
    drop(guard);
}

#[test]
fn catalog_identity_matches_held_guard_and_canonical_path_for_unicode_and_long_paths() {
    let directory = tempdir().unwrap();
    let mut parent = directory.path().join("图库 空格");
    for depth in 0..9 {
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("目录.sqlite3");
        fs::write(&path, b"catalog identity fixture").unwrap();
        assert_identity_matches_existing_contract(&path);
        parent = parent.join(format!("目录层级_{depth}_{}", "segment".repeat(8)));
    }
    assert!(parent.as_os_str().len() > 512);
}

#[test]
fn catalog_identity_reopens_the_current_object_after_replacement() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite3");
    fs::write(&path, b"original").unwrap();
    let original = read_catalog_identity(&path).unwrap();
    fs::rename(&path, directory.path().join("retired.sqlite3")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    let replacement = read_catalog_identity(&path).unwrap();
    assert_eq!(original.canonical_path, replacement.canonical_path);
    assert_ne!(original.file_identity, replacement.file_identity);
    assert_identity_matches_existing_contract(&path);
}

#[test]
fn catalog_identity_does_not_create_a_missing_catalog_or_keep_a_temporary_guard() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("missing.sqlite3");
    let error = read_catalog_identity(&path).err().expect("missing file");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    assert!(!path.exists());
    fs::write(&path, b"held").unwrap();
    read_catalog_identity(&path).unwrap();
    fs::remove_file(&path).expect("attribute-only lookup has released its handle");
}
