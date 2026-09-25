use super::*;

#[test]
fn bounded_subtree_completes_one_terminal_child() {
    reconcile_terminal_children(1);
}

#[test]
fn bounded_subtree_completes_multiple_terminal_children() {
    reconcile_terminal_children(2);
}

#[test]
fn subtree_terminal_retirement_preserves_neighbors_and_rolls_back_with_publication() {
    let source = tempdir().expect("source");
    let storage = tempdir().expect("storage");
    for directory in ["album_%", "album_%2"] {
        fs::create_dir(source.path().join(directory)).unwrap();
        write_png(
            &source.path().join(directory).join("ready.png"),
            [10, 20, 30, 255],
        );
    }
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "terminal-retirement-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).unwrap();
    let root = only_root(&catalog);
    for (index, directory) in ["album_%", "album_%2"].into_iter().enumerate() {
        fs::write(
            source.path().join(directory).join("broken.png"),
            b"broken PNG",
        )
        .unwrap();
        let clock = 2_000 + index as i64 * 1_000;
        enqueue_intent(
            &mut catalog,
            subtree_intent(&root, LibraryChangeIntentKind::Reconcile, directory, None),
            clock,
        );
        process_ready_authoritative_library_change(
            &mut catalog,
            &root.root_id,
            root.root_generation,
            clock,
            immediate_queue_policy(),
            fixture_recovery_policy(),
        )
        .unwrap();
    }
    let old_paths = ["album_%/broken.png".into(), "album_%2/broken.png".into()];
    assert_eq!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(&root.root_id, &old_paths)
            .unwrap()
            .len(),
        2
    );
    fs::rename(source.path().join("album_%"), source.path().join("renamed")).unwrap();
    enqueue_intent(
        &mut catalog,
        subtree_intent(
            &root,
            LibraryChangeIntentKind::RenameCandidate,
            "renamed",
            Some("album_%"),
        ),
        4_000,
    );
    let hook = crate::adapters::set_before_catalog_delta_commit_hook(&root.root_id, || {
        Err(ScanError::new(
            "fixture_commit_rejected",
            "reject after mutation",
        ))
    });
    let error = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        4_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .unwrap_err();
    assert_eq!(error.code, "fixture_commit_rejected");
    drop(hook);
    assert_eq!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(&root.root_id, &old_paths)
            .unwrap()
            .len(),
        2
    );
    assert!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(
                &root.root_id,
                &["renamed/broken.png".into()]
            )
            .unwrap()
            .is_empty()
    );
    process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        6_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .unwrap();
    let recovered = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        6_100,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .unwrap();
    assert_eq!(recovered.incremental.completed_count, 1);
    let old = catalog
        .load_terminal_media_evidence_by_relative_paths(&root.root_id, &old_paths)
        .unwrap();
    assert_eq!(old.len(), 1);
    assert_eq!(old[0].relative_path, "album_%2/broken.png");
    assert_eq!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(
                &root.root_id,
                &["renamed/broken.png".into()]
            )
            .unwrap()
            .len(),
        1
    );
    fs::remove_file(source.path().join("renamed/broken.png")).unwrap();
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "renamed", None),
        7_000,
    );
    process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        7_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .unwrap();
    assert!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(
                &root.root_id,
                &["renamed/broken.png".into()]
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read(source.path().join("album_%2/broken.png")).unwrap(),
        b"broken PNG"
    );
    drop(catalog);
    SqliteCatalog::open(paths.catalog_path).expect("complete catalog proof after retirement");
}

fn reconcile_terminal_children(count: usize) {
    let source = tempdir().expect("source");
    let storage = tempdir().expect("storage");
    let album = source.path().join("album");
    fs::create_dir(&album).expect("album");
    write_png(&album.join("ready.png"), [10, 20, 30, 255]);
    let paths = fixture_storage(&storage);
    publish_initial_scan(&source, paths.clone(), "terminal-subtree-baseline");
    let mut catalog = SqliteCatalog::open(paths.catalog_path.clone()).expect("catalog");
    let root = only_root(&catalog);
    let relative_paths = (0..count)
        .map(|index| format!("album/incomplete-{index}.png"))
        .collect::<Vec<_>>();
    for path in &relative_paths {
        fs::write(source.path().join(path), b"incomplete screenshot").expect("owned fixture");
    }
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        2_000,
    );
    let report = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        2_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("terminal children do not block their directory lease");
    assert_eq!(report.incremental.completed_count, 1);
    assert_eq!(report.incremental.retried_count, 0);
    let evidence = catalog
        .load_terminal_media_evidence_by_relative_paths(&root.root_id, &relative_paths)
        .expect("terminal evidence");
    assert_eq!(evidence.len(), count);
    for path in &relative_paths {
        assert_eq!(
            fs::read(source.path().join(path)).expect("source remains unchanged"),
            b"incomplete screenshot",
        );
    }
    for path in &relative_paths {
        write_png(&source.path().join(path), [30, 40, 50, 255]);
    }
    enqueue_intent(
        &mut catalog,
        subtree_intent(&root, LibraryChangeIntentKind::Reconcile, "album", None),
        3_000,
    );
    let repaired = process_ready_authoritative_library_change(
        &mut catalog,
        &root.root_id,
        root.root_generation,
        3_000,
        immediate_queue_policy(),
        fixture_recovery_policy(),
    )
    .expect("completed screenshot writes converge");
    assert_eq!(repaired.incremental.completed_count, 1);
    assert!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(&root.root_id, &relative_paths)
            .expect("replaced negative evidence")
            .is_empty()
    );
}
