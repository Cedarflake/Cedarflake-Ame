use std::fs;

use tempfile::tempdir;

use super::*;

#[test]
fn partial_stream_cannot_finish_or_retain_its_publication_namespace_after_drop() {
    use super::super::super::PublicationGuardedFileDiscovery;

    let directory = tempfile::tempdir_in(std::env::current_dir().expect("current directory"))
        .expect("owned publication directory");
    let root = directory.path().join("root");
    let moved = directory.path().join("moved");
    fs::create_dir(&root).expect("owned root");
    fs::write(root.join("photo.jpg"), b"unread fixture").expect("owned source");
    let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &root.to_string_lossy(),
        None,
    )
    .expect("publication namespace");
    let mut entries = guard
        .streaming_metadata_inventory_entries_in_directory("")
        .expect("owned cursor");
    drop(guard);
    assert_eq!(
        fs::rename(&root, &moved)
            .expect_err("cursor pins root")
            .raw_os_error(),
        Some(32)
    );
    assert!(entries.next().expect("one entry").is_ok());
    let error = entries
        .finish()
        .expect_err("terminal boundary has not been read");
    assert_eq!(error.code, "directory_entry_unreadable");
    fs::rename(&root, &moved).expect("failed finish releases the cursor and namespace");
    assert_eq!(
        fs::read(moved.join("photo.jpg")).expect("retained source"),
        b"unread fixture"
    );
}

#[test]
fn handle_directory_open_enumerates_a_nested_nonreparse_directory() {
    let directory = tempdir().expect("temporary directory");
    let album = directory.path().join("album");
    fs::create_dir_all(&album).expect("nested directory");
    fs::write(album.join("inside.png"), b"inside").expect("nested source");
    let discovery =
        FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");

    let mut opened = discovery
        .streaming_checked_entry_paths_in_directory("album")
        .expect("root-relative nested directory");
    let entry = opened
        .next()
        .expect("nested entry")
        .expect("valid nested entry");
    assert_eq!(entry.relative_path, "album/inside.png");
    assert!(opened.next().is_none());
    assert!(opened.finish().expect("finish nested directory").is_some());
}

#[test]
fn handle_directory_open_completes_an_empty_directory_without_a_false_failure() {
    let directory = tempdir().expect("temporary directory");
    let empty = directory.path().join("empty");
    fs::create_dir(&empty).expect("empty directory");
    let discovery =
        FileDiscovery::new(&directory.path().to_string_lossy()).expect("file discovery");

    let mut opened = discovery
        .streaming_checked_entry_paths_in_directory("empty")
        .expect("root-relative empty directory");

    assert!(opened.next().is_none());
    assert!(opened.finish().expect("finish empty directory").is_some());
}

#[test]
fn handle_directory_open_rejects_an_ancestor_junction_swap_at_the_authorization_window() {
    use std::process::Command;

    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("root");
    let outside = directory.path().join("outside");
    let album = root.join("album");
    fs::create_dir_all(&album).expect("source directory");
    fs::create_dir_all(&outside).expect("outside directory");
    fs::write(album.join("inside.png"), b"inside").expect("inside source");
    fs::write(outside.join("outside.png"), b"outside").expect("outside source");
    let discovery = FileDiscovery::new(&root.to_string_lossy()).expect("file discovery");

    let opened = discovery.streaming_checked_entry_paths_in_directory_with_hook("album", || {
        fs::rename(&album, root.join("album-original")).expect("move authorized directory");
        let junction = Command::new("cmd.exe")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(&album)
            .arg(&outside)
            .status()
            .expect("junction command");
        assert!(junction.success());
    });
    fs::remove_dir(&album).expect("remove junction");

    let issue = match opened {
        Ok(_) => panic!("the external junction target must not be enumerated"),
        Err(issue) => issue,
    };
    assert_eq!(
        issue.code, "source_path_outside_root",
        "unexpected root-relative open failure: {}",
        issue.message
    );
}
