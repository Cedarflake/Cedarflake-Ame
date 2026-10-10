use super::*;

#[test]
fn path_reconcile_with_previous_path_completes_without_widening_terminal_authority() {
    let directory = tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let mut change = intent("root-a", LibraryChangeIntentKind::Reconcile, "current.png");
    change.previous_relative_path = Some("previous.png".into());
    let lease = lease_change(&mut catalog, change);
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .unwrap()
        .unwrap();
    let mut batch = CatalogDeltaBatch {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        expected_catalog_revision: root.catalog_revision,
        mutations: Vec::new(),
        terminal_media_evidence: vec![TerminalMediaEvidenceUpdate {
            change_id: lease.change.id,
            evidence: TerminalMediaEvidence {
                relative_path: "previous.png".into(),
                file_size: 12,
                modified_unix_ms: 1_500,
                file_identity: None,
                source_revision: None,
                source_generation: 0,
                inspection_engine_id: "ame-image-inspection".into(),
                inspection_engine_version: 1,
                issue: LibraryChangeFailure {
                    code: "image_decode_invalid".into(),
                    message: "truncated image".into(),
                },
            },
        }],
        completions: vec![LibraryChangeCompletion {
            change_id: lease.change.id,
            lease_generation: lease.lease_generation,
            issue: None,
        }],
    };
    let error = catalog.publish_catalog_delta(&batch, 2_000).unwrap_err();
    assert_eq!(error.code, "catalog_terminal_media_evidence_path_mismatch");
    assert!(
        catalog
            .load_terminal_media_evidence_by_relative_paths("root-a", &["previous.png".into()])
            .unwrap()
            .is_empty()
    );
    batch.terminal_media_evidence[0].evidence.relative_path = "current.png".into();
    assert_eq!(
        catalog.publish_catalog_delta(&batch, 2_000).unwrap().status,
        CatalogDeltaPublicationStatus::Applied
    );
    assert_eq!(
        catalog
            .load_terminal_media_evidence_by_relative_paths("root-a", &["current.png".into()])
            .unwrap()
            .len(),
        1
    );
}
