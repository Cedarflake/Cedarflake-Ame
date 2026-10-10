use super::*;

#[test]
fn metadata_warnings_do_not_consume_the_retained_rejection_path_bound_after_reopen() {
    let source = tempfile::tempdir().expect("owned source namespace");
    let storage = tempfile::tempdir().expect("isolated catalog");
    let path = storage.path().join("catalog.sqlite3");
    let root = source.path().to_string_lossy().into_owned();
    let request = ScanRequest {
        scan_id: "retained-filter".to_owned(),
        root_path: root.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    catalog
        .begin_authoritative_scan(&request, "root", &root)
        .expect("retained scan owner");
    for index in 0..=RETRY_PATH_LIMIT {
        catalog
            .record_issue(
                &request.scan_id,
                &ScanIssue {
                    path: Some(
                        source
                            .path()
                            .join("warning.png")
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    code: "orientation_read_failed".to_owned(),
                    message: format!("warning-{index}"),
                },
            )
            .expect("retain ordinary metadata warning");
    }
    catalog
        .record_issue(
            &request.scan_id,
            &ScanIssue {
                path: Some(
                    source
                        .path()
                        .join("rejected.data")
                        .to_string_lossy()
                        .into_owned(),
                ),
                code: "media_type_unsupported".to_owned(),
                message: "old inspection without retained version".to_owned(),
            },
        )
        .expect("retain one actual rejection");
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("reopen without TEMP observations");
    let mut foreground = FinalizationPlan::new(FinalizationMode::Foreground);
    assert!(
        !restore(
            &mut foreground,
            &catalog,
            &request.scan_id,
            source.path(),
            source.path()
        )
        .expect("warnings do not consume negative allowance")
    );
    assert_eq!(
        foreground.foreground_revalidation.paths,
        BTreeSet::from(["rejected.data".to_owned()])
    );
    assert!(
        !foreground.foreground_revalidation.overflowed,
        "no whole-root fallback"
    );
    let mut recovery = FinalizationPlan::new(FinalizationMode::AuthoritativeRecovery);
    assert!(
        restore(
            &mut recovery,
            &catalog,
            &request.scan_id,
            source.path(),
            source.path()
        )
        .expect("legacy authority filters warnings independently")
    );
    assert_eq!(
        recovery.authoritative_retry_paths(),
        vec!["rejected.data".to_owned()]
    );
}

#[test]
fn retained_terminal_paths_are_read_in_bounded_keyset_windows_without_warning_payloads() {
    let source = tempfile::tempdir().expect("owned source namespace");
    let storage = tempfile::tempdir().expect("isolated catalog");
    let root = source.path().to_string_lossy().into_owned();
    let request = ScanRequest {
        scan_id: "retained-window".to_owned(),
        root_path: root.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    catalog
        .begin_authoritative_scan(&request, "root", &root)
        .expect("scan owner");
    for index in 0..530 {
        for (code, name) in [
            ("metadata_read_failed", "warning"),
            ("image_decode_invalid", "terminal"),
        ] {
            catalog
                .record_issue(
                    &request.scan_id,
                    &ScanIssue {
                        path: Some(
                            source
                                .path()
                                .join(format!("{name}-{index:03}.bmp"))
                                .to_string_lossy()
                                .into_owned(),
                        ),
                        code: code.to_owned(),
                        message: "fixture issue".to_owned(),
                    },
                )
                .expect("interleaved persisted issue");
        }
    }
    let mut after_id = 0;
    let mut observed = 0;
    for expected in [256, 256, 18, 0] {
        let window = load_retained_scan_issue_window(
            &catalog,
            &request.scan_id,
            TERMINAL_CODES,
            after_id,
            256,
        )
        .expect("bounded filtered window");
        assert_eq!(window.len(), expected);
        for issue in window {
            assert!(issue.id > after_id);
            assert!(
                issue
                    .path
                    .as_deref()
                    .expect("terminal path")
                    .contains("terminal-")
            );
            after_id = issue.id;
            observed += 1;
        }
    }
    assert_eq!(observed, 530);
}
