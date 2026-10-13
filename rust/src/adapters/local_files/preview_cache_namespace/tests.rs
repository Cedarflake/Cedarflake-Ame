use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn preview_cleanup_namespace_pins_root_and_ancestor_until_release() {
    let directory = tempdir().expect("owned namespace");
    let ancestor = directory.path().join("ancestor");
    let root = ancestor.join("cache");
    fs::create_dir_all(&root).expect("cache namespace");
    let bytes = b"owned fixture";
    fs::write(root.join("entry"), bytes).expect("fixture bytes");
    let namespace = PreviewCacheNamespace::open(&root).expect("guarded namespace");
    assert_eq!(
        namespace.root().expect("bound root"),
        fs::canonicalize(&root).expect("physical root")
    );
    for (from, to) in [
        (root.clone(), ancestor.join("moved-cache")),
        (ancestor.clone(), directory.path().join("moved-ancestor")),
    ] {
        let error = fs::rename(&from, &to).expect_err("owned namespace remains pinned");
        assert_eq!(error.raw_os_error(), Some(32));
    }
    drop(namespace);
    let moved_root = ancestor.join("moved-cache");
    fs::rename(&root, &moved_root).expect("released root may move");
    fs::rename(&moved_root, &root).expect("restore owned fixture root");
    let moved_ancestor = directory.path().join("moved-ancestor");
    fs::rename(&ancestor, &moved_ancestor).expect("released ancestor may move");
    fs::rename(&moved_ancestor, &ancestor).expect("restore owned ancestor");
    assert_eq!(
        fs::read(root.join("entry")).expect("preserved bytes"),
        bytes
    );
}

#[test]
fn preview_cleanup_namespace_absence_never_acquires_later_created_files() {
    let directory = tempdir().expect("owned namespace");
    let root = directory.path().join("absent");
    let namespace = PreviewCacheNamespace::open(&root).expect("observed missing namespace");
    assert!(namespace.root().is_none());
    fs::create_dir(&root).expect("later directory");
    fs::write(root.join("entry"), b"later bytes").expect("later file");
    assert!(namespace.root().is_none());
    drop(namespace);
    assert_eq!(
        fs::read(root.join("entry")).expect("untouched file"),
        b"later bytes"
    );
}

#[test]
fn preview_cleanup_namespace_rejects_non_directory_and_device_paths() {
    let directory = tempdir().expect("owned namespace");
    let file = directory.path().join("file");
    fs::write(&file, b"not a directory").expect("fixture");
    for path in [
        file.as_path(),
        Path::new("relative-cache"),
        Path::new(r"\\.\C:"),
    ] {
        let error = PreviewCacheNamespace::open(path)
            .err()
            .expect("unsupported namespace");
        assert_eq!(error.code, "preview_cleanup_namespace_unavailable");
    }
    assert_eq!(fs::read(file).expect("preserved bytes"), b"not a directory");
}

#[test]
fn preview_cache_namespace_preserves_volume_root_rejection_for_final_paths() {
    for (path, direct_volume_parent) in [
        (Path::new(r"D:\NewCache"), true),
        (Path::new(r"D:\A\B"), false),
        (Path::new(r"\\?\D:\NewCache"), true),
        (Path::new(r"\\?\D:\A\B"), false),
    ] {
        let (volume, components) = publication_namespace_path(path).expect("valid child namespace");
        assert_eq!(volume, Path::new(r"D:\"));
        assert_eq!(components.len() == 1, direct_volume_parent);
        let error = PreviewCacheNamespace::for_operation(&volume)
            .err()
            .expect("volume is only a parent anchor, never a final cache");
        assert_eq!(error.code, "preview_cleanup_namespace_unavailable");
    }
    assert!(PreviewCacheNamespace::for_operation(Path::new(r"\\?\D:\")).is_err());
}

#[test]
fn preview_cache_namespace_creates_missing_ancestors_and_releases_them() {
    let directory = tempdir().expect("owned namespace");
    let ancestor = directory.path().join("missing-parent");
    let root = ancestor.join("nested").join("cache");
    let namespace = PreviewCacheNamespace::for_operation(&root).expect("create bound cache");
    assert!(namespace.admits(&root));
    assert!(namespace.identity().is_some());
    assert_eq!(fs::read_dir(&root).expect("empty cache").count(), 0);
    let moved = directory.path().join("moved-parent");
    let error = fs::rename(&ancestor, &moved).expect_err("creation retained ancestor guards");
    assert_eq!(error.raw_os_error(), Some(32));
    drop(namespace);
    fs::rename(&ancestor, &moved).expect("operation releases the created namespace");
}
