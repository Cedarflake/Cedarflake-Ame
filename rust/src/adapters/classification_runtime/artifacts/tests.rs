use super::*;

const FIXTURE: Artifact = Artifact {
    name: "fixture.bin",
    length: 3,
    digest: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
};

#[test]
fn verified_file_cannot_be_changed_or_replaced_until_retirement() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(FIXTURE.name);
    std::fs::write(&path, b"abc").unwrap();
    let guard = verify_file(&path, &FIXTURE).unwrap();
    assert!(OpenOptions::new().write(true).open(&path).is_err());
    assert!(std::fs::remove_file(&path).is_err());
    drop(guard);
    std::fs::write(&path, b"def").unwrap();
    assert!(matches!(
        verify_file(&path, &FIXTURE),
        Err(RuntimeError::ArtifactMismatch)
    ));
    std::fs::write(&path, b"abc ").unwrap();
    assert!(matches!(
        verify_file(&path, &FIXTURE),
        Err(RuntimeError::ArtifactMismatch)
    ));
}

#[test]
fn held_parent_directory_prevents_verified_path_redirection() {
    let directory = tempfile::tempdir().unwrap();
    let parent = directory.path().join("模型");
    let nested = parent.join("encoder");
    std::fs::create_dir_all(&nested).unwrap();
    let guards = pin_directory(&nested).unwrap();
    let moved = directory.path().join("moved");
    assert!(std::fs::rename(&parent, &moved).is_err());
    drop(guards);
    std::fs::rename(parent, moved).unwrap();
}

#[test]
fn admission_rejects_network_relative_parent_and_placeholder_paths() {
    for path in [r"models", r"\\server\models", r"C:\models\..\other"] {
        assert!(matches!(
            pin_directory(Path::new(path)),
            Err(RuntimeError::UnsafeArtifact)
        ));
    }
    for attributes in [
        FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_OFFLINE,
        FILE_ATTRIBUTE_RECALL_ON_OPEN,
        FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
    ] {
        assert!(!admitted_attributes(attributes));
    }
    assert!(admitted_attributes(0));
}
