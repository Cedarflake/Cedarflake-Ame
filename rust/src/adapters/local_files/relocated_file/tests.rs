use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn lookup_is_metadata_only_and_stays_inside_the_pinned_root() {
    let fixture = tempdir().unwrap();
    let root = fixture.path().join("library");
    fs::create_dir(&root).unwrap();
    let original = root.join("original.png");
    fs::write(&original, b"unchanged source bytes").unwrap();
    let identity = super::super::super::file_identity_evidence(&original)
        .unwrap()
        .unwrap();
    let source = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &root.to_string_lossy(),
        None,
    )
    .unwrap();
    let destination = root.join("子目录").join("移动.png");
    fs::create_dir(destination.parent().unwrap()).unwrap();
    fs::rename(&original, &destination).unwrap();
    assert_eq!(
        locate_relative_file(&source, &identity).unwrap().as_deref(),
        Some("子目录/移动.png")
    );
    assert_eq!(fs::read(&destination).unwrap(), b"unchanged source bytes");
    let outside = fixture.path().join("outside.png");
    fs::rename(&destination, &outside).unwrap();
    assert_eq!(locate_relative_file(&source, &identity).unwrap(), None);
    fs::remove_file(&outside).unwrap();
    assert_eq!(locate_relative_file(&source, &identity).unwrap(), None);
}

#[test]
fn lookup_rejects_other_volumes_directories_and_unknown_identity_schemes() {
    let fixture = tempdir().unwrap();
    let source = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &fixture.path().to_string_lossy(),
        None,
    )
    .unwrap();
    let mut identity = source.metadata_inventory_root_identity().unwrap().unwrap();
    assert_eq!(locate_relative_file(&source, &identity).unwrap(), None);
    identity.value.replace_range(..16, "0000000000000000");
    assert_eq!(locate_relative_file(&source, &identity).unwrap(), None);
    identity.scheme = "unsupported".into();
    assert_eq!(locate_relative_file(&source, &identity).unwrap(), None);
}
