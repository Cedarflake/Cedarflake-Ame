use tempfile::tempdir;

use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, FileIdentityEvidence, IncrementalReconciliationOutcome,
    LibraryChangeCatchUpEvidence, LibraryChangeCompletion, LibraryChangeFailure, LibraryChangeId,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, PreviewArtifact, PreviewStatus,
    RetainedPreviewExpectation, ScanRequest, SourceRevisionEvidence, TerminalMediaEvidence,
    TerminalMediaEvidenceUpdate,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

use super::super::SqliteCatalog;

#[test]
fn persistent_journal_range_finalization_is_gated_by_owned_lineage() {
    let change_id = LibraryChangeId::new(1).expect("change ID");
    let mut evidence_by_change = std::collections::HashMap::new();
    assert!(!super::has_persistent_journal_lineage(&evidence_by_change));

    evidence_by_change.insert(
        change_id,
        vec![LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: "unrelated-watermark".to_owned(),
        }],
    );
    assert!(!super::has_persistent_journal_lineage(&evidence_by_change));

    evidence_by_change
        .get_mut(&change_id)
        .expect("evidence")
        .push(LibraryChangeCatchUpEvidence {
            source: super::PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
            watermark: "persistent-range".to_owned(),
        });
    assert!(super::has_persistent_journal_lineage(&evidence_by_change));
}

#[test]
fn incremental_path_window_is_bounded_and_root_scoped() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(
        &mut catalog,
        "root-a",
        "C:/source-a",
        &[
            location("asset-a", "location-a", "root-a", "a.jpg"),
            location("asset-c", "location-c", "root-a", "c.jpg"),
        ],
    );
    seed_catalog(
        &mut catalog,
        "root-b",
        "C:/source-b",
        &[location("asset-b", "location-b", "root-b", "a.jpg")],
    );

    assert!(
        catalog
            .load_incremental_locations_by_relative_paths("root-a", &[])
            .expect("load empty path window")
            .is_empty()
    );
    let locations = catalog
        .load_incremental_locations_by_relative_paths(
            "root-a",
            &[
                "missing.jpg".to_owned(),
                "c.jpg".to_owned(),
                "a.jpg".to_owned(),
            ],
        )
        .expect("load bounded path window");
    assert_eq!(
        locations
            .iter()
            .map(|location| (location.root_id.as_str(), location.relative_path.as_str()))
            .collect::<Vec<_>>(),
        vec![("root-a", "a.jpg"), ("root-a", "c.jpg")]
    );

    let oversized = vec!["a.jpg".to_owned(); 4_097];
    let error = catalog
        .load_incremental_locations_by_relative_paths("root-a", &oversized)
        .expect_err("reject oversized path window");
    assert_eq!(error.code, "catalog_relative_path_window_invalid");
}

#[test]
fn authoritative_subtree_window_keeps_case_distinct_siblings_outside_its_capacity() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(
        &mut catalog,
        "root-a",
        "C:/source",
        &[
            location("asset-upper", "location-upper", "root-a", "Album/upper.jpg"),
            location("asset-lower", "location-lower", "root-a", "album/lower.jpg"),
        ],
    );

    let locations = catalog
        .load_incremental_locations_in_subtree("root-a", "Album", 1)
        .expect("load exact-case authoritative subtree");

    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].relative_path, "Album/upper.jpg");
}

#[test]
fn publishes_a_location_and_completes_its_lease_at_one_revision() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish delta");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    assert_eq!(publication.catalog_revision, root.catalog_revision + 1);
    assert_eq!(publication.applied_mutation_count, 1);
    assert_eq!(publication.completed_change_count, 1);
    let stored = catalog
        .load_incremental_location_by_relative_path("root-a", "new.jpg")
        .expect("load location")
        .expect("stored location");
    assert_eq!(stored.asset_id, "asset-new");
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.completed_count, 1);
    assert_eq!(metrics.leased_count, 0);
}

#[test]
fn changed_hardlink_fans_out_revision_generation_and_preview_invalidation() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let identity = FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "0000000000000001:00000000000000000000000000000001".to_owned(),
    };
    let original_revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000001".to_owned(),
    };
    let changed_revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000002".to_owned(),
    };
    let mut first = location("asset-shared", "location-a", "root-a", "a.jpg");
    first.file_identity = Some(identity.clone());
    first.source_revision = Some(original_revision.clone());
    first.source_generation = 0;
    first.preview_status = PreviewStatus::Failed;
    first.preview_issue_code = Some("image_decode_failed".to_owned());
    first.preview_issue_message = Some("old failure".to_owned());
    let mut second = location("asset-shared", "location-b", "root-a", "b.jpg");
    second.file_identity = Some(identity);
    second.source_revision = Some(original_revision);
    second.source_generation = 0;
    second.preview_status = PreviewStatus::Failed;
    second.preview_issue_code = Some("image_decode_failed".to_owned());
    second.preview_issue_message = Some("old failure".to_owned());
    seed_catalog(
        &mut catalog,
        "root-a",
        "C:/source",
        &[first.clone(), second],
    );
    let original_generation = catalog
        .load_incremental_location_by_relative_path("root-a", "a.jpg")
        .expect("load original hardlink")
        .expect("original hardlink")
        .source_generation;
    let replacement_scan = ScanRequest {
        scan_id: "hardlink-replacement-scan".to_owned(),
        root_path: "C:/source".to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&replacement_scan, "root-a", "C:/source")
        .expect("begin hardlink replacement scan");
    for relative_path in ["a.jpg", "b.jpg"] {
        let staged = catalog
            .load_incremental_location_by_relative_path("root-a", relative_path)
            .expect("load hardlink for staging")
            .expect("active hardlink for staging");
        catalog
            .stage_location(&replacement_scan.scan_id, "root-a", &staged)
            .expect("stage hardlink before live change");
    }
    assert_eq!(
        catalog
            .count_staged_file_states(&replacement_scan.scan_id)
            .expect("flush staged hardlinks"),
        2
    );
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "a.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    first.source_revision = Some(changed_revision.clone());
    first.source_generation = 0;
    first.file_size = 2_048;
    first.created_unix_ms = Some(2_000);
    first.modified_unix_ms = 3_000;
    first.width = 512;
    first.height = 320;
    first.metadata_engine_id = "metadata-v2".to_owned();
    first.metadata_engine_version = "2".to_owned();
    first.preview_status = PreviewStatus::Pending;
    first.preview_issue_code = None;
    first.preview_issue_message = None;

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Modified,
                    evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(first),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish hardlink change");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    let first = catalog
        .load_incremental_location_by_relative_path("root-a", "a.jpg")
        .expect("load first hardlink")
        .expect("first hardlink");
    let second = catalog
        .load_incremental_location_by_relative_path("root-a", "b.jpg")
        .expect("load second hardlink")
        .expect("second hardlink");
    assert_eq!(first.source_generation, second.source_generation);
    assert_ne!(first.source_generation, original_generation);
    assert_eq!(first.source_revision, Some(changed_revision.clone()));
    assert_eq!(second.source_revision, Some(changed_revision));
    for location in [first, second] {
        assert_eq!(location.file_size, 2_048);
        assert_eq!(location.created_unix_ms, Some(2_000));
        assert_eq!(location.modified_unix_ms, 3_000);
        assert_eq!((location.width, location.height), (512, 320));
        assert_eq!(location.metadata_engine_id, "metadata-v2");
        assert_eq!(location.metadata_engine_version, "2");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
        assert!(location.preview_issue_code.is_none());
        assert!(location.preview_issue_message.is_none());
    }
    let staged_hardlinks = catalog
        .connection
        .prepare(
            "SELECT file_size, created_unix_ms, modified_unix_ms, width, height,
                    metadata_engine_id, metadata_engine_version,
                    source_revision_token, source_generation, preview_status
             FROM asset_locations
             WHERE scan_id = ?1 ORDER BY relative_path",
        )
        .expect("prepare staged hardlinks")
        .query_map([&replacement_scan.scan_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .expect("query staged hardlinks")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect staged hardlinks");
    assert_eq!(staged_hardlinks.len(), 2);
    for staged in staged_hardlinks {
        assert_eq!(staged.0, 2_048);
        assert_eq!(staged.1, Some(2_000));
        assert_eq!(staged.2, 3_000);
        assert_eq!((staged.3, staged.4), (512, 320));
        assert_eq!(staged.5, "metadata-v2");
        assert_eq!(staged.6, "2");
        assert_eq!(
            staged.7.as_deref(),
            Some("windows-file-change-time-100ns-v1:0000000000000002")
        );
        assert_eq!(
            u64::try_from(staged.8).expect("staged generation"),
            catalog
                .load_incremental_location_by_relative_path("root-a", "a.jpg")
                .expect("load active generation")
                .expect("active generation")
                .source_generation
        );
        assert_eq!(staged.9, "pending");
    }
    catalog
        .publish_scan(&replacement_scan.scan_id, "root-a", 2, 0)
        .expect("publish hardlink replacement scan");
}

#[test]
fn terminal_hardlink_change_updates_sibling_source_state_and_failure() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let identity = FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "0000000000000001:00000000000000000000000000000001".to_owned(),
    };
    let changed_revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000002".to_owned(),
    };
    let mut first = location("asset-shared", "location-a", "root-a", "a.jpg");
    first.file_identity = Some(identity.clone());
    first.source_generation = 0;
    let mut second = location("asset-shared", "location-b", "root-a", "b.jpg");
    second.file_identity = Some(identity.clone());
    second.source_generation = 0;
    seed_catalog(&mut catalog, "root-a", "C:/source", &[first, second]);
    let original_generation = catalog
        .load_incremental_location_by_relative_path("root-a", "b.jpg")
        .expect("load original sibling")
        .expect("original sibling")
        .source_generation;
    let replacement_scan = ScanRequest {
        scan_id: "terminal-hardlink-replacement-scan".to_owned(),
        root_path: "C:/source".to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&replacement_scan, "root-a", "C:/source")
        .expect("begin terminal hardlink replacement scan");
    for relative_path in ["a.jpg", "b.jpg"] {
        let staged = catalog
            .load_incremental_location_by_relative_path("root-a", relative_path)
            .expect("load terminal hardlink for staging")
            .expect("active terminal hardlink for staging");
        catalog
            .stage_location(&replacement_scan.scan_id, "root-a", &staged)
            .expect("stage hardlink before terminal live change");
    }
    assert_eq!(
        catalog
            .count_staged_file_states(&replacement_scan.scan_id)
            .expect("flush terminal staged hardlinks"),
        2
    );
    let mut terminal = catalog
        .load_incremental_location_by_relative_path("root-a", "a.jpg")
        .expect("load changed hardlink")
        .expect("changed hardlink");
    terminal.file_size = 2_048;
    terminal.modified_unix_ms = 3_000;
    terminal.source_revision = Some(changed_revision.clone());
    terminal.source_generation = 0;
    terminal.width = 0;
    terminal.height = 0;
    terminal.preview_path.clear();
    terminal.preview_status = PreviewStatus::Failed;
    terminal.preview_issue_code = Some("image_decode_invalid".to_owned());
    terminal.preview_issue_message = Some("malformed shared image".to_owned());
    terminal.metadata_engine_id = "ame-image-inspection".to_owned();
    terminal.metadata_engine_version = "2".to_owned();
    terminal.capture_time = None;
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "a.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    let failure = LibraryChangeFailure {
        code: "image_decode_invalid".to_owned(),
        message: "malformed shared image".to_owned(),
    };

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::TerminalIssue,
                    evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
                    remove_location_ids: vec!["location-a".to_owned()],
                    upsert_location: Some(terminal),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: vec![TerminalMediaEvidenceUpdate {
                    change_id: leased.change.id,
                    evidence: TerminalMediaEvidence {
                        relative_path: "a.jpg".to_owned(),
                        file_size: 2_048,
                        modified_unix_ms: 3_000,
                        file_identity: Some(identity),
                        source_revision: Some(changed_revision.clone()),
                        source_generation: 0,
                        inspection_engine_id: "ame-image-inspection".to_owned(),
                        inspection_engine_version: 2,
                        issue: failure.clone(),
                    },
                }],
                completions: vec![LibraryChangeCompletion {
                    change_id: leased.change.id,
                    lease_generation: leased.lease_generation,
                    issue: Some(failure),
                }],
            },
            2_000,
        )
        .expect("publish terminal hardlink change");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    let changed = catalog
        .load_incremental_location_by_relative_path("root-a", "a.jpg")
        .expect("load changed hardlink")
        .expect("changed hardlink placeholder");
    let sibling = catalog
        .load_incremental_location_by_relative_path("root-a", "b.jpg")
        .expect("load sibling hardlink")
        .expect("sibling hardlink");
    for location in [&changed, &sibling] {
        assert_eq!(location.file_size, 2_048);
        assert_eq!(location.modified_unix_ms, 3_000);
        assert_eq!(location.source_revision, Some(changed_revision.clone()));
        assert_ne!(location.source_generation, original_generation);
        assert!(matches!(location.preview_status, PreviewStatus::Failed));
        assert!(location.preview_path.is_empty());
        assert_eq!(
            location.preview_issue_code.as_deref(),
            Some("image_decode_invalid")
        );
        assert_eq!(
            location.preview_issue_message.as_deref(),
            Some("malformed shared image")
        );
    }
    assert_eq!(changed.source_generation, sibling.source_generation);
    let staged_failures = catalog
        .connection
        .prepare(
            "SELECT file_size, modified_unix_ms, source_revision_token,
                    source_generation, preview_status, preview_issue_code
             FROM asset_locations
             WHERE scan_id = ?1 ORDER BY relative_path",
        )
        .expect("prepare staged terminal hardlinks")
        .query_map([&replacement_scan.scan_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .expect("query staged terminal hardlinks")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect staged terminal hardlinks");
    assert_eq!(staged_failures.len(), 2);
    for staged in staged_failures {
        assert_eq!(staged.0, 2_048);
        assert_eq!(staged.1, 3_000);
        assert_eq!(
            staged.2.as_deref(),
            Some("windows-file-change-time-100ns-v1:0000000000000002")
        );
        assert_eq!(
            u64::try_from(staged.3).expect("staged terminal generation"),
            changed.source_generation
        );
        assert_eq!(staged.4, "failed");
        assert_eq!(staged.5.as_deref(), Some("image_decode_invalid"));
    }
    let terminal_evidence = catalog
        .load_terminal_media_evidence_by_relative_paths(
            "root-a",
            &["a.jpg".to_owned(), "b.jpg".to_owned()],
        )
        .expect("load hardlink terminal evidence");
    assert_eq!(terminal_evidence.len(), 2);

    let repaired_revision = SourceRevisionEvidence {
        scheme: "windows-file-change-time-100ns-v1".to_owned(),
        value: "0000000000000003".to_owned(),
    };
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "b.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load repaired root")
        .expect("repaired root");
    let mut repaired = sibling;
    repaired.file_size = 3_072;
    repaired.modified_unix_ms = 4_000;
    repaired.source_revision = Some(repaired_revision.clone());
    repaired.source_generation = 0;
    repaired.width = 640;
    repaired.height = 480;
    repaired.preview_status = PreviewStatus::Pending;
    repaired.preview_issue_code = None;
    repaired.preview_issue_message = None;
    repaired.metadata_engine_id = "metadata-v3".to_owned();
    repaired.metadata_engine_version = "3".to_owned();
    catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Modified,
                    evidence_disposition: DerivedEvidenceDisposition::InvalidateDerived,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(repaired),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            3_000,
        )
        .expect("publish repaired hardlink");
    let repaired_a = catalog
        .load_incremental_location_by_relative_path("root-a", "a.jpg")
        .expect("load repaired first alias")
        .expect("repaired first alias");
    let repaired_b = catalog
        .load_incremental_location_by_relative_path("root-a", "b.jpg")
        .expect("load repaired second alias")
        .expect("repaired second alias");
    for location in [repaired_a, repaired_b] {
        assert_eq!(location.source_revision, Some(repaired_revision.clone()));
        assert_eq!((location.width, location.height), (640, 480));
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_issue_code.is_none());
    }
    assert!(
        catalog
            .load_terminal_media_evidence_by_relative_paths(
                "root-a",
                &["a.jpg".to_owned(), "b.jpg".to_owned()],
            )
            .expect("reload hardlink terminal evidence")
            .is_empty()
    );
    catalog
        .publish_scan(&replacement_scan.scan_id, "root-a", 2, 0)
        .expect("publish repaired hardlink replacement scan");
}

#[test]
fn terminal_media_evidence_is_committed_with_lease_completion() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "broken.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    let failure = LibraryChangeFailure {
        code: "image_decode_invalid".to_owned(),
        message: "malformed JPEG".to_owned(),
    };

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: Vec::new(),
                terminal_media_evidence: vec![TerminalMediaEvidenceUpdate {
                    change_id: leased.change.id,
                    evidence: TerminalMediaEvidence {
                        relative_path: "broken.jpg".to_owned(),
                        file_size: 12,
                        modified_unix_ms: 1_500,
                        file_identity: Some(FileIdentityEvidence {
                            scheme: "windows-file-id-v1".to_owned(),
                            value: "volume:file".to_owned(),
                        }),
                        source_revision: None,
                        source_generation: 0,
                        inspection_engine_id: "ame-image-inspection".to_owned(),
                        inspection_engine_version: 1,
                        issue: failure.clone(),
                    },
                }],
                completions: vec![LibraryChangeCompletion {
                    change_id: leased.change.id,
                    lease_generation: leased.lease_generation,
                    issue: Some(failure),
                }],
            },
            2_000,
        )
        .expect("publish terminal evidence");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    assert_eq!(publication.catalog_revision, root.catalog_revision);
    let evidence = catalog
        .load_terminal_media_evidence_by_relative_paths("root-a", &["broken.jpg".to_owned()])
        .expect("load terminal evidence");
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].issue.code, "image_decode_invalid");
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.completed_count, 1);
    assert_eq!(metrics.retry_wait_count, 0);
}

#[test]
fn rejects_a_delta_with_inconsistent_reconciliation_evidence() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;

    let error = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Modified,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect_err("reject inconsistent evidence");

    assert_eq!(error.code, "catalog_delta_evidence_contract_invalid");
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "new.jpg")
            .expect("load location")
            .is_none()
    );
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.leased_count, 1);
}

#[test]
fn a_superseded_lease_cannot_publish_catalog_state() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "same.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    catalog
        .enqueue_library_change_intents(
            &[intent(
                "root-a",
                LibraryChangeIntentKind::Reconcile,
                "same.jpg",
            )],
            1_500,
            policy(),
        )
        .expect("enqueue newer evidence");

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-stale",
                        "location-stale",
                        "root-a",
                        "same.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("reject stale delta");

    assert_eq!(
        publication.status,
        CatalogDeltaPublicationStatus::StaleLease
    );
    assert_eq!(publication.catalog_revision, root.catalog_revision);
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "same.jpg")
            .expect("load location")
            .is_none()
    );
}

#[test]
fn a_changed_catalog_revision_rejects_the_complete_delta_batch() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source-a", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let stale_revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;
    seed_catalog(&mut catalog, "root-b", "C:/source-b", &[]);

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: stale_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-stale",
                        "location-stale",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("reject stale revision");

    assert_eq!(
        publication.status,
        CatalogDeltaPublicationStatus::StaleCatalogRevision
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "new.jpg")
            .expect("load location")
            .is_none()
    );
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.completed_count, 0);
}

#[test]
fn a_running_full_scan_preserves_and_mirrors_live_publication() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;
    let replacement_scan = ScanRequest {
        scan_id: "replacement-scan".to_owned(),
        root_path: "C:/source".to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&replacement_scan, "root-a", "C:/source")
        .expect("begin replacement scan");
    let leased_during_scan: (String, i64, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT status, lease_generation, authoritative_scan_id
             FROM library_change_queue WHERE id = ?1",
            [i64::try_from(leased.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("live lease during scan");
    assert_eq!(leased_during_scan.0, "leased");
    assert_eq!(
        leased_during_scan.1,
        i64::try_from(leased.lease_generation).expect("lease generation")
    );
    assert!(leased_during_scan.2.is_none());
    let blocked = catalog
        .publish_scan("replacement-scan", "root-a", 0, 0)
        .expect_err("unfinished live work must delay scan publication");
    assert_eq!(blocked.code, "catalog_scan_live_changes_pending");

    let mut new_location = location("asset-new", "location-new", "root-a", "new.jpg");
    new_location.file_identity = Some(FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "000000000000000100000000000000000000000000000001".to_owned(),
    });
    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(new_location),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish live delta during scan");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    assert_eq!(publication.catalog_revision, revision + 1);
    let active = catalog
        .load_incremental_location_by_relative_path("root-a", "new.jpg")
        .expect("load active location")
        .expect("active location");
    let staged: (String, i64, Option<String>) = catalog
        .connection
        .query_row(
            "SELECT relative_path, source_generation, source_revision_token
             FROM asset_locations
             WHERE scan_id = 'replacement-scan' AND location_id = 'location-new'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("mirrored staged location");
    assert_eq!(staged.0, "new.jpg");
    assert_eq!(
        staged.1,
        i64::try_from(active.source_generation).expect("source generation")
    );
    assert!(staged.2.is_some());
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.leased_count, 0);
    assert_eq!(metrics.completed_count, 1);

    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET source_generation = source_generation + 1
             WHERE scan_id = ?1 AND location_id = ?2",
            rusqlite::params![active.scan_id, active.location_id],
        )
        .expect("advance active source generation after mirror");
    catalog
        .connection
        .execute(
            "UPDATE catalog_state SET next_source_generation = next_source_generation + 1",
            [],
        )
        .expect("advance source generation allocator");
    let winning_active = catalog
        .load_incremental_location_by_relative_path("root-a", "new.jpg")
        .expect("load newer active location")
        .expect("newer active location");
    assert!(winning_active.source_generation > active.source_generation);

    catalog
        .publish_scan("replacement-scan", "root-a", 999, 0)
        .expect("publish replacement scan after live delta");
    let published_count: i64 = catalog
        .connection
        .query_row(
            "SELECT asset_count FROM scan_runs WHERE id = 'replacement-scan'",
            [],
            |row| row.get(0),
        )
        .expect("published staged count");
    assert_eq!(published_count, 1);
    assert_eq!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "new.jpg")
            .expect("load published location")
            .expect("published location")
            .source_generation,
        winning_active.source_generation
    );
}

#[test]
fn a_running_full_scan_mirrors_live_removal() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(
        &mut catalog,
        "root-a",
        "C:/source",
        &[location(
            "asset-removed",
            "location-removed",
            "root-a",
            "removed.jpg",
        )],
    );
    let active = catalog
        .load_incremental_location_by_relative_path("root-a", "removed.jpg")
        .expect("load active location")
        .expect("active location");
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "removed.jpg"),
    );
    let revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;
    let replacement_scan = ScanRequest {
        scan_id: "removal-replacement-scan".to_owned(),
        root_path: "C:/source".to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&replacement_scan, "root-a", "C:/source")
        .expect("begin replacement scan");
    catalog
        .stage_location(&replacement_scan.scan_id, "root-a", &active)
        .expect("stage pre-removal location");
    assert_eq!(
        catalog
            .count_staged_file_states(&replacement_scan.scan_id)
            .expect("flush staged location"),
        1
    );

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Removed,
                    evidence_disposition: DerivedEvidenceDisposition::RemoveFromCurrentProjection,
                    remove_location_ids: vec![active.location_id],
                    upsert_location: None,
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish live removal during scan");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "removed.jpg")
            .expect("load removed active path")
            .is_none()
    );
    assert_eq!(
        catalog
            .count_staged_file_states(&replacement_scan.scan_id)
            .expect("load mirrored staged removal"),
        0
    );
    catalog
        .publish_scan(&replacement_scan.scan_id, "root-a", 1, 0)
        .expect("publish replacement scan after live removal");
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "removed.jpg")
            .expect("load published removed path")
            .is_none()
    );
}

#[test]
fn a_running_full_scan_still_blocks_non_live_publication() {
    for (suffix, origin) in [
        ("journal", LibraryChangeOrigin::StartupCatchUp),
        ("recovery", LibraryChangeOrigin::MetadataInventory),
    ] {
        let directory = tempdir().expect("temporary directory");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
        seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
        let relative_path = format!("{suffix}.jpg");
        let mut non_live_intent =
            intent("root-a", LibraryChangeIntentKind::Reconcile, &relative_path);
        non_live_intent.origin = origin;
        let leased = lease_change(&mut catalog, non_live_intent);
        let revision = catalog
            .load_incremental_catalog_root("root-a")
            .expect("load root")
            .expect("root")
            .catalog_revision;
        let replacement_scan = ScanRequest {
            scan_id: format!("replacement-{suffix}-scan"),
            root_path: "C:/source".to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        catalog
            .begin_scan(&replacement_scan, "root-a", "C:/source")
            .expect("begin replacement scan");

        let publication = catalog
            .publish_catalog_delta(
                &CatalogDeltaBatch {
                    root_id: "root-a".to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    expected_catalog_revision: revision,
                    mutations: vec![CatalogDeltaMutation {
                        change_id: leased.change.id,
                        outcome: IncrementalReconciliationOutcome::Added,
                        evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                        remove_location_ids: Vec::new(),
                        upsert_location: Some(location(
                            &format!("asset-{suffix}"),
                            &format!("location-{suffix}"),
                            "root-a",
                            &relative_path,
                        )),
                        retained_preview_expectation: None,
                    }],
                    terminal_media_evidence: Vec::new(),
                    completions: vec![completion(&leased)],
                },
                2_000,
            )
            .expect("defer non-live publication during scan");

        assert_eq!(
            publication.status,
            CatalogDeltaPublicationStatus::RootScanInProgress
        );
        assert_eq!(publication.catalog_revision, revision);
    }
}

#[test]
fn a_retired_root_generation_cannot_publish_a_leased_delta() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;
    assert!(catalog.unregister_root("root-a").expect("retire root"));

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("reject retired root publication");

    assert_eq!(
        publication.status,
        CatalogDeltaPublicationStatus::RootGenerationChanged
    );
    assert_eq!(publication.applied_mutation_count, 0);
    assert_eq!(publication.completed_change_count, 0);
}

#[test]
fn queue_completion_failure_rolls_back_the_catalog_delta_and_revision() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let revision = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root")
        .catalog_revision;
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_delta_completion
             BEFORE UPDATE OF status ON library_change_queue
             WHEN NEW.status = 'completed'
             BEGIN
               SELECT RAISE(ABORT, 'injected completion failure');
             END;",
        )
        .expect("install failure trigger");

    let error = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect_err("injected transaction failure");
    assert_eq!(error.code, "catalog_database_error");
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "new.jpg")
            .expect("load location")
            .is_none()
    );
    assert_eq!(
        catalog
            .load_incremental_catalog_root("root-a")
            .expect("load root")
            .expect("root")
            .catalog_revision,
        revision
    );
    let metrics = catalog
        .load_library_change_queue_metrics(2_000, policy())
        .expect("queue metrics");
    assert_eq!(metrics.leased_count, 1);
    assert_eq!(metrics.completed_count, 0);
}

#[test]
fn identity_preserving_rename_moves_preview_ownership_atomically() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let mut old_location = location("asset-a", "location-old", "root-a", "old.jpg");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[old_location.clone()]);
    old_location = catalog
        .load_incremental_location_by_relative_path("root-a", "old.jpg")
        .expect("load seeded old location")
        .expect("seeded old location");
    let artifact = PreviewArtifact {
        artifact_key: "artifact-a".to_owned(),
        algorithm_id: "preview".to_owned(),
        algorithm_version: 1,
        orientation_contract: "orientation".to_owned(),
        size_bucket: 256,
        path: "C:/preview/artifact-a.jpg".to_owned(),
        byte_size: 128,
        encoded_width: 64,
        encoded_height: 64,
        width: 64,
        height: 64,
    };
    old_location.preview_path = artifact.path.clone();
    old_location.preview_status = PreviewStatus::Ready;
    catalog
        .update_active_preview(&old_location, Some(&artifact), None)
        .expect("publish preview");
    old_location = catalog
        .load_incremental_location_by_relative_path("root-a", "old.jpg")
        .expect("load old location")
        .expect("old location");
    let leased = lease_change(
        &mut catalog,
        LibraryChangeIntent {
            kind: LibraryChangeIntentKind::RenameCandidate,
            previous_relative_path: Some("old.jpg".to_owned()),
            relative_path: "new.jpg".to_owned(),
            ..intent(
                "root-a",
                LibraryChangeIntentKind::RenameCandidate,
                "new.jpg",
            )
        },
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    let mut new_location = old_location.clone();
    new_location.location_id = "location-new".to_owned();
    new_location.relative_path = "new.jpg".to_owned();
    new_location.absolute_path = "C:/source/new.jpg".to_owned();
    new_location.display_path = new_location.absolute_path.clone();

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::RenamedOrMoved,
                    evidence_disposition: DerivedEvidenceDisposition::RetainCompatible,
                    remove_location_ids: vec![old_location.location_id.clone()],
                    upsert_location: Some(new_location.clone()),
                    retained_preview_expectation: Some(RetainedPreviewExpectation {
                        location_id: old_location.location_id.clone(),
                        preview_path: old_location.preview_path.clone(),
                        preview_status: old_location.preview_status.clone(),
                        preview_issue_code: old_location.preview_issue_code.clone(),
                        preview_issue_message: old_location.preview_issue_message.clone(),
                    }),
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish rename");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "old.jpg")
            .expect("load old path")
            .is_none()
    );
    let stored = catalog
        .load_incremental_location_by_relative_path("root-a", "new.jpg")
        .expect("load new path")
        .expect("new path");
    assert_eq!(stored.asset_id, "asset-a");
    assert_eq!(stored.preview_path, artifact.path);
    let owners = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations
             WHERE artifact_key = 'artifact-a' AND location_id = 'location-new'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("preview owners");
    assert_eq!(owners, 1);
}

#[test]
fn preview_cleanup_invalidates_a_prepared_retained_preview_delta() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let mut old_location = location("asset-a", "location-old", "root-a", "old.jpg");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[old_location.clone()]);
    old_location = catalog
        .load_incremental_location_by_relative_path("root-a", "old.jpg")
        .expect("load seeded old location")
        .expect("seeded old location");
    let artifact = PreviewArtifact {
        artifact_key: "artifact-a".to_owned(),
        algorithm_id: "preview".to_owned(),
        algorithm_version: 1,
        orientation_contract: "orientation".to_owned(),
        size_bucket: 256,
        path: "C:/preview/artifact-a.jpg".to_owned(),
        byte_size: 128,
        encoded_width: 64,
        encoded_height: 64,
        width: 64,
        height: 64,
    };
    old_location.preview_path = artifact.path.clone();
    old_location.preview_status = PreviewStatus::Ready;
    catalog
        .update_active_preview(&old_location, Some(&artifact), None)
        .expect("publish preview");
    old_location = catalog
        .load_incremental_location_by_relative_path("root-a", "old.jpg")
        .expect("load old location")
        .expect("old location");
    let leased = lease_change(
        &mut catalog,
        LibraryChangeIntent {
            kind: LibraryChangeIntentKind::RenameCandidate,
            previous_relative_path: Some("old.jpg".to_owned()),
            relative_path: "new.jpg".to_owned(),
            ..intent(
                "root-a",
                LibraryChangeIntentKind::RenameCandidate,
                "new.jpg",
            )
        },
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");
    let mut new_location = old_location.clone();
    new_location.location_id = "location-new".to_owned();
    new_location.relative_path = "new.jpg".to_owned();
    new_location.absolute_path = "C:/source/new.jpg".to_owned();
    new_location.display_path = new_location.absolute_path.clone();
    catalog
        .reset_all_previews_for_cleanup()
        .expect("reset previews");

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::RenamedOrMoved,
                    evidence_disposition: DerivedEvidenceDisposition::RetainCompatible,
                    remove_location_ids: vec![old_location.location_id.clone()],
                    upsert_location: Some(new_location),
                    retained_preview_expectation: Some(RetainedPreviewExpectation {
                        location_id: old_location.location_id.clone(),
                        preview_path: old_location.preview_path.clone(),
                        preview_status: old_location.preview_status.clone(),
                        preview_issue_code: old_location.preview_issue_code.clone(),
                        preview_issue_message: old_location.preview_issue_message.clone(),
                    }),
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("reject stale preview delta");

    assert_eq!(
        publication.status,
        CatalogDeltaPublicationStatus::StalePreviewState
    );
    let stored = catalog
        .load_incremental_location_by_relative_path("root-a", "old.jpg")
        .expect("load old path")
        .expect("old path remains");
    assert!(matches!(stored.preview_status, PreviewStatus::Pending));
    assert!(stored.preview_path.is_empty());
    assert!(
        catalog
            .load_incremental_location_by_relative_path("root-a", "new.jpg")
            .expect("load new path")
            .is_none()
    );
}

#[test]
fn delta_maintenance_does_not_scan_or_rewrite_unaffected_global_state() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    seed_catalog(&mut catalog, "root-a", "C:/source", &[]);
    catalog
        .connection
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES ('unrelated-orphan', 1)",
            [],
        )
        .expect("insert unrelated orphan");
    catalog
        .connection
        .execute(
            "INSERT INTO preview_artifacts(
               artifact_key, source_file_size, source_modified_unix_ms,
               source_identity_scheme, source_identity_value,
               algorithm_id, algorithm_version, orientation_contract, size_bucket,
               encoded_width, encoded_height, artifact_path, byte_size,
               lifecycle_state, created_unix_ms, last_used_unix_ms
             ) VALUES (
               'unrelated-artifact', 1, 1, NULL, NULL,
               'preview', 1, 'orientation', 256, 1, 1,
               'C:/preview/unrelated.jpg', 1, 'ready', 1, 1
             )",
            [],
        )
        .expect("insert unrelated artifact");
    let leased = lease_change(
        &mut catalog,
        intent("root-a", LibraryChangeIntentKind::Reconcile, "new.jpg"),
    );
    let root = catalog
        .load_incremental_catalog_root("root-a")
        .expect("load root")
        .expect("root");

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: leased.change.id,
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("publish bounded delta");

    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
    let orphan_exists = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id = 'unrelated-orphan')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .expect("load unrelated orphan");
    assert!(orphan_exists);
    let lifecycle = catalog
        .connection
        .query_row(
            "SELECT lifecycle_state FROM preview_artifacts
             WHERE artifact_key = 'unrelated-artifact'",
            [],
            |row| row.get::<_, String>(0),
        )
        .expect("load unrelated artifact");
    assert_eq!(lifecycle, "ready");
    let asset_count = catalog
        .connection
        .query_row(
            "SELECT asset_count FROM scan_runs WHERE id = 'scan-root-a'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("load asset count");
    assert_eq!(asset_count, 1);
}

#[test]
fn terminal_lineage_cleanup_preserves_other_watermark_owners() {
    let directory = tempdir().expect("temporary directory");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO assets(id, created_unix_ms) VALUES ('asset-a', 1);
             INSERT INTO preview_artifacts(
               artifact_key, source_file_size, source_modified_unix_ms,
               source_identity_scheme, source_identity_value,
               algorithm_id, algorithm_version, orientation_contract, size_bucket,
               encoded_width, encoded_height, artifact_path, byte_size,
               lifecycle_state, created_unix_ms, last_used_unix_ms
             ) VALUES (
               'artifact-a', 1, 1, 'windows-file-id-128-v1', 'volume:file',
               'preview', 1, 'orientation', 256, 1, 1,
               'C:/preview/photo.jpg', 1, 'ready', 1, 1
             );
             INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               updated_unix_ms
             ) VALUES
             (
               'windows_usn_v1', 'watermark-1', 'windows-file-id-128-v1', 'volume:file',
               'asset-a', 'location-a', 'root-a', 'C:/source/photo.jpg', 'photo.jpg',
               'C:/preview/photo.jpg', 1, NULL, 1, 1, 1, 'ready', NULL, NULL,
               'metadata', '1', NULL, NULL, NULL, NULL, 1
             ),
             (
               'windows_usn_v1', 'watermark-2', 'windows-file-id-128-v1', 'volume:file',
               'asset-a', 'location-a', 'root-a', 'C:/source/photo.jpg', 'photo.jpg',
               'C:/preview/photo.jpg', 1, NULL, 1, 1, 1, 'ready', NULL, NULL,
               'metadata', '1', NULL, NULL, NULL, NULL, 2
             );",
        )
        .expect("seed handoff owners");

    let transaction = catalog
        .connection
        .transaction()
        .expect("cleanup transaction");
    super::cleanup_terminal_catch_up_handoffs(&transaction, "windows_usn_v1", "watermark-1")
        .expect("cleanup first watermark");
    transaction.commit().expect("commit first cleanup");

    let (asset_count, lifecycle, handoff_count) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets WHERE id = 'asset-a'),
               (SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = 'artifact-a'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("retained owner state");
    assert_eq!(
        (asset_count, lifecycle.as_str(), handoff_count),
        (1, "ready", 1)
    );

    let transaction = catalog.connection.transaction().expect("final transaction");
    super::cleanup_terminal_catch_up_handoffs(&transaction, "windows_usn_v1", "watermark-2")
        .expect("cleanup final watermark");
    transaction.commit().expect("commit final cleanup");
    let (asset_count, lifecycle, handoff_count) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM assets WHERE id = 'asset-a'),
               (SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = 'artifact-a'),
               (SELECT COUNT(*) FROM library_change_catch_up_handoffs)",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("released owner state");
    assert_eq!(
        (asset_count, lifecycle.as_str(), handoff_count),
        (0, "stale", 0)
    );
}

#[test]
fn incremental_identity_lookup_reads_a_normalized_scan_handoff_batch() {
    let directory = tempdir().expect("temporary directory");
    let catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_change_scan_handoff_batches(
               id, source_root_id, updated_unix_ms
             ) VALUES ('scan-source', 'root-source', 10);
             INSERT INTO library_change_scan_handoff_lineage(
               batch_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
             ) VALUES ('scan-source', 'windows_usn_v1', 'volume|12|40', 10);
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               source_revision_token, source_generation
              ) VALUES (
               'scan-source', 'windows-file-id-128-v1', 'volume:file',
               'asset-a', 'location-source', 'root-source',
               'C:/source/photo.jpg', 'photo.jpg', 'C:/preview/photo.jpg',
               10, 1, 2, 8, 6, 'ready', NULL, NULL,
               'metadata', '1', NULL, NULL, NULL, NULL,
               'windows-file-change-time-100ns-v1:0000000000000001', 1
              );",
        )
        .expect("normalized scan handoff fixture");

    let location = catalog
        .load_incremental_location_by_file_identity(
            &FileIdentityEvidence {
                scheme: "windows-file-id-128-v1".to_owned(),
                value: "volume:file".to_owned(),
            },
            &[LibraryChangeCatchUpEvidence {
                source: "windows_usn_v1".to_owned(),
                watermark: "volume|12|40".to_owned(),
            }],
        )
        .expect("load normalized handoff")
        .expect("retained location");

    assert_eq!(location.asset_id, "asset-a");
    assert_eq!(location.location_id, "location-source");
    assert_eq!(location.relative_path, "photo.jpg");
    assert_eq!(location.preview_path, "C:/preview/photo.jpg");
    assert!(matches!(location.preview_status, PreviewStatus::Ready));
}

fn seed_catalog(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    root_path: &str,
    locations: &[AssetLocationView],
) {
    let request = ScanRequest {
        scan_id: format!("scan-{root_id}"),
        root_path: root_path.to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 256,
    };
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove delta fixture first-import handoff");
    for location in locations {
        catalog
            .stage_location(&request.scan_id, root_id, location)
            .expect("stage location");
    }
    catalog
        .publish_scan(
            &request.scan_id,
            root_id,
            u64::try_from(locations.len()).expect("location count"),
            0,
        )
        .expect("publish scan");
}

fn lease_change(
    catalog: &mut SqliteCatalog,
    change: LibraryChangeIntent,
) -> crate::domain::LeasedLibraryChange {
    catalog
        .enqueue_library_change_intents(&[change], 1_000, policy())
        .expect("enqueue change");
    catalog
        .lease_library_changes("root-a", LibraryRootGeneration::initial(), 1_000, policy())
        .expect("lease changes")
        .into_iter()
        .next()
        .expect("leased change")
}

fn completion(leased: &crate::domain::LeasedLibraryChange) -> LibraryChangeCompletion {
    LibraryChangeCompletion {
        change_id: leased.change.id,
        lease_generation: leased.lease_generation,
        issue: None,
    }
}

fn intent(
    root_id: &str,
    kind: LibraryChangeIntentKind,
    relative_path: &str,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root_id.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: 1_000,
        most_recent_observed_unix_ms: 1_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    }
}

fn location(
    asset_id: &str,
    location_id: &str,
    root_id: &str,
    relative_path: &str,
) -> AssetLocationView {
    AssetLocationView {
        asset_id: asset_id.to_owned(),
        location_id: location_id.to_owned(),
        root_id: root_id.to_owned(),
        scan_id: format!("{root_id}-scan"),
        absolute_path: format!("C:/source/{relative_path}"),
        display_path: format!("C:/source/{relative_path}"),
        relative_path: relative_path.to_owned(),
        preview_path: String::new(),
        file_size: 10,
        created_unix_ms: Some(1_000),
        modified_unix_ms: 1_000,
        file_identity: None,
        source_revision: Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0000000000000001".to_owned(),
        }),
        source_generation: 1,
        width: 64,
        height: 64,
        preview_status: PreviewStatus::Pending,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "metadata".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    }
}

fn policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    }
}
