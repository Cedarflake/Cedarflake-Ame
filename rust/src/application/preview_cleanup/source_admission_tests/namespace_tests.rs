use super::*;

#[test]
fn retired_cleanup_absent_namespace_never_deletes_files_created_at_started() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    fs::remove_file(&fixture.artifact).expect("arrange absent generated cache");
    fs::remove_dir(&fixture.retired_root).expect("remove empty generated cache");
    let mut created = false;
    let mut completed = false;
    clear_preview_scope(
        "retired-absent-namespace".to_owned(),
        |event| {
            if let PreviewCleanupEvent::Started {
                total_files,
                total_bytes,
                ..
            } = event
            {
                assert_eq!((total_files, total_bytes), (0, 0));
                fs::create_dir(&fixture.retired_root).expect("new namespace after admission");
                fs::write(&fixture.artifact, &fixture.bytes).expect("new generated source bytes");
                created = true;
            }
            completed |= matches!(
                event,
                PreviewCleanupEvent::Completed {
                    removed_files: 0,
                    removed_bytes: 0,
                    issue_count: 0,
                    ..
                }
            );
            true
        },
        fixture.scope(),
    )
    .expect("missing-root cleanup has no filesystem deletion authority");
    assert!(created && completed);
    assert_eq!(
        fs::read(&fixture.artifact).expect("later source survives"),
        fixture.bytes
    );
    assert_eq!(
        fs::read_dir(&fixture.retired_root)
            .expect("new source entries")
            .count(),
        1
    );
}

#[test]
fn retired_cleanup_namespace_releases_on_cancel_detach_and_unwind() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    for outcome in ["cancel", "detach", "unwind"] {
        let fixture = Fixture::new();
        let moved = fixture.directory.path().join("released-cache");
        let mut started = false;
        let result = catch_unwind(AssertUnwindSafe(|| {
            clear_preview_scope(
                "retired-namespace-release".to_owned(),
                |event| {
                    if matches!(event, PreviewCleanupEvent::Started { .. }) {
                        started = true;
                        let error = fs::rename(&fixture.retired_root, &moved)
                            .expect_err("namespace remains guarded during the callback");
                        assert_eq!(error.raw_os_error(), Some(32));
                        match outcome {
                            "cancel" => {
                                assert!(cancel_preview_cleanup("retired-namespace-release"))
                            }
                            "detach" => return false,
                            _ => panic!("injected observer unwind"),
                        }
                    }
                    true
                },
                fixture.scope(),
            )
        }));
        assert!(started);
        if outcome == "unwind" {
            assert!(result.is_err());
        } else {
            result
                .expect("observer does not panic")
                .expect("controlled cleanup exit");
        }
        fs::rename(&fixture.retired_root, &moved)
            .expect("all terminal outcomes release the namespace");
        assert_eq!(
            fs::read(moved.join(fixture.artifact.file_name().expect("leaf")))
                .expect("cancelled source bytes remain"),
            fixture.bytes
        );
        assert!(!cancel_preview_cleanup("retired-namespace-release"));
    }
}

#[test]
fn retired_cleanup_cannot_delete_a_source_moved_into_its_checked_namespace() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = Fixture::new();
    let source_root = fixture.directory.path().join("published-source");
    let displaced_cache = fixture.directory.path().join("displaced-cache");
    fs::create_dir(&source_root).expect("isolated source root");
    let leaf = fixture.artifact.file_name().expect("managed leaf");
    let source = source_root.join(leaf);
    fs::write(&source, &fixture.bytes).expect("generated source fixture");
    fixture.import("source-before-namespace-change", &source_root, 1);
    assert_eq!(fixture.catalog_counts(), (1, 1));
    let mut attempted = false;
    let mut replaced = false;
    let result = clear_preview_scope(
        "retired-namespace-change".to_owned(),
        |event| {
            if matches!(event, PreviewCleanupEvent::Started { .. }) {
                attempted = true;
                match fs::rename(&fixture.retired_root, &displaced_cache) {
                    Ok(()) => {
                        fs::rename(&source_root, &fixture.retired_root)
                            .expect("simulate external replacement using disposable fixtures");
                        replaced = true;
                    }
                    Err(error) => assert_eq!(
                        error.raw_os_error(),
                        Some(32),
                        "only a held Windows sharing guard may prevent the test interleaving"
                    ),
                }
            }
            true
        },
        fixture.scope(),
    );
    assert!(
        attempted,
        "cleanup must reach the real event boundary: {result:?}"
    );
    let remaining_source = if replaced {
        fixture.retired_root.join(leaf)
    } else {
        source
    };
    assert_eq!(
        fs::read(&remaining_source).expect("cleanup must not delete a replacement source image"),
        fixture.bytes
    );
    if !replaced {
        result.expect("pinned original derived cache can be cleaned");
        fs::rename(&fixture.retired_root, &displaced_cache)
            .expect("cleanup must release its namespace guard");
    }
}
