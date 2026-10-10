use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn locates_renamed_directory_and_pins_its_complete_namespace() {
    let fixture = tempdir().unwrap();
    let original = fixture.path().join("原始目录");
    let replacement = fixture.path().join("新位置");
    fs::create_dir(&original).unwrap();
    let identity = super::super::file_identity_evidence(&original)
        .unwrap()
        .unwrap();
    fs::rename(&original, &replacement).unwrap();
    let located = locate_library_root(&original.to_string_lossy(), &identity)
        .unwrap()
        .unwrap();
    assert_eq!(
        Path::new(located.path()).canonicalize().unwrap(),
        replacement.canonicalize().unwrap()
    );
    assert_eq!(
        located.guard().metadata_inventory_root_identity().unwrap(),
        Some(identity)
    );
    assert!(fs::rename(&replacement, &original).is_err());
    drop(located);
    fs::rename(&replacement, &original).unwrap();
}

#[test]
fn a_regular_file_identity_cannot_become_a_root() {
    let fixture = tempdir().unwrap();
    let file = fixture.path().join("source.png");
    fs::write(&file, b"generated content").unwrap();
    let identity = super::super::file_identity_evidence(&file)
        .unwrap()
        .unwrap();
    assert!(
        locate_library_root(&file.to_string_lossy(), &identity)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::read(file).unwrap(), b"generated content");
}

#[test]
fn other_volumes_and_unsupported_namespaces_do_not_search() {
    let fixture = tempdir().unwrap();
    let mut identity = super::super::file_identity_evidence(fixture.path())
        .unwrap()
        .unwrap();
    let (volume, _) = parse_identity(&identity).unwrap().unwrap();
    identity
        .value
        .replace_range(..16, &format!("{:016x}", volume ^ 1));
    assert!(
        locate_library_root(&fixture.path().to_string_lossy(), &identity)
            .unwrap()
            .is_none()
    );
    for path in [r"\\server\share\root", r"\\.\C:", r"C:relative", "relative"] {
        assert!(locate_library_root(path, &identity).unwrap().is_none());
    }
}

#[test]
fn malformed_and_unknown_identity_schemes_fail_closed() {
    for value in [
        "",
        "0:0",
        "0000000000000000:0000000000000000000000000000000Z",
    ] {
        let identity = FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".into(),
            value: value.into(),
        };
        assert_eq!(
            parse_identity(&identity).unwrap_err().code,
            "root_location_identity_invalid"
        );
    }
    let identity = FileIdentityEvidence {
        scheme: "future".into(),
        value: "anything".into(),
    };
    assert_eq!(parse_identity(&identity).unwrap(), None);
}
