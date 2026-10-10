use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn preview_preparation_namespace_refusal_never_creates_or_changes_cache_files() {
    for exists in [false, true] {
        let directory = tempdir().expect("owned storage");
        let root = directory.path().join("cache");
        if exists {
            fs::create_dir(&root).expect("existing directory");
            fs::write(root.join("foreign.jpg"), b"preserved bytes").expect("owned fixture");
        }
        let outcome = prepare_preview_root_with_namespace_probe(&root, |_| {
            Err(ScanError::new(
                "injected_namespace_unavailable",
                "unsupported fixture capability",
            ))
        })
        .expect("capability refusal is not catalog initialization failure");
        assert_eq!(outcome, PreviewRootPreparation::NamespaceUnavailable);
        assert_eq!(root.exists(), exists);
        if exists {
            assert_eq!(
                fs::read(root.join("foreign.jpg")).expect("untouched bytes"),
                b"preserved bytes"
            );
            assert_eq!(fs::read_dir(&root).expect("untouched directory").count(), 1);
        }
    }
}

#[test]
fn preview_preparation_releases_its_namespace_without_creating_an_active_store() {
    let directory = tempdir().expect("owned storage");
    let root = directory.path().join("cache");
    assert_eq!(
        prepare_preview_root(&root).expect("prepared"),
        PreviewRootPreparation::Ready
    );
    fs::rename(&root, directory.path().join("moved-cache")).expect("no retained handles");
}

#[cfg(windows)]
#[test]
fn preview_operation_namespace_failure_is_actionable_and_not_source_failure() {
    let error = open_preview_cache_operation_namespace(Path::new("relative-cache"))
        .err()
        .expect("invalid namespace");
    assert_eq!(error.code, "preview_cache_namespace_unavailable");
    assert!(error.message.contains("图库仍可浏览"));
    assert!(error.message.contains("NTFS"));
}
