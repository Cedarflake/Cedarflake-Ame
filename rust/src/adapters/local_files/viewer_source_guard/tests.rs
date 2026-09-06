use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
};
use windows_sys::Win32::System::SystemServices::{
    IO_REPARSE_TAG_APPEXECLINK, IO_REPARSE_TAG_MOUNT_POINT, IO_REPARSE_TAG_SYMLINK,
};

use super::*;

#[test]
fn viewer_directory_admits_only_available_ordinary_or_official_cloud_tags() {
    assert_eq!(
        classify_viewer_directory(FILE_ATTRIBUTE_DIRECTORY, 0).expect("ordinary"),
        ViewerDirectoryKind::Ordinary
    );
    let attributes = FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT;
    for variant in 0..=15 {
        let tag = IO_REPARSE_TAG_CLOUD | (variant << 12);
        // CLOUD variants encode placeholder state; family membership alone is
        // not availability. The native value-only API marks these variants partial.
        if variant & 2 == 0 {
            assert!(classify_viewer_directory(attributes, tag).is_err());
        } else {
            assert_eq!(
                classify_viewer_directory(attributes, tag).expect("available Cloud directory"),
                ViewerDirectoryKind::HydratedCloud
            );
        }
        for unavailable in [
            FILE_ATTRIBUTE_OFFLINE,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        ] {
            assert!(classify_viewer_directory(attributes | unavailable, tag).is_err());
        }
    }
    for tag in [
        0,
        IO_REPARSE_TAG_SYMLINK,
        IO_REPARSE_TAG_MOUNT_POINT,
        IO_REPARSE_TAG_APPEXECLINK,
        IO_REPARSE_TAG_CLOUD | 0x2000_0000,
        IO_REPARSE_TAG_CLOUD | 0x0001_0000,
    ] {
        assert!(
            classify_viewer_directory(attributes, tag).is_err(),
            "unsupported tag {tag:x}"
        );
    }
    assert!(classify_viewer_directory(FILE_ATTRIBUTE_REPARSE_POINT, IO_REPARSE_TAG_CLOUD).is_err());
    assert!(classify_viewer_directory(FILE_ATTRIBUTE_DIRECTORY, IO_REPARSE_TAG_CLOUD).is_err());
}

#[test]
fn viewer_directory_requires_exact_volume_and_normal_source_components() {
    let directory = tempfile::tempdir().expect("fixture");
    let root = canonical_source_root_path(directory.path()).expect("canonical root");
    let opened = open_publication_namespace_guard(&root).expect("directory handle");
    let volume = raw_file_id_info(&opened)
        .expect("volume")
        .VolumeSerialNumber;
    validate_viewer_directory(&opened, volume).expect("exact volume");
    assert!(validate_viewer_directory(&opened, volume.wrapping_add(1)).is_err());
    let root_identity = file_identity_evidence(&root)
        .expect("root identity query")
        .expect("root identity");
    let expected = ExpectedFileState {
        absolute_path: root.join("image.png").to_string_lossy().into_owned(),
        file_size: 1,
        modified_unix_ms: 0,
        file_identity: None,
        source_revision: None,
    };
    for relative in ["", "../image.png", "C:\\image.png", "\\image.png"] {
        assert!(
            open_viewer_source_guard(&expected, &root, Path::new(relative), &root_identity)
                .is_err()
        );
    }
}
