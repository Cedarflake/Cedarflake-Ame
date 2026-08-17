use tempfile::tempdir;

use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, IncrementalReconciliationOutcome, LibraryChangeCompletion,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRootGeneration, PreviewArtifact, PreviewStatus, ScanRequest,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue};

use super::super::SqliteCatalog;

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
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                }],
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
                    outcome: IncrementalReconciliationOutcome::Modified,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                }],
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
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-stale",
                        "location-stale",
                        "root-a",
                        "same.jpg",
                    )),
                }],
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
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-stale",
                        "location-stale",
                        "root-a",
                        "new.jpg",
                    )),
                }],
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
fn a_running_full_scan_blocks_incremental_publication() {
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

    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: "root-a".to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                expected_catalog_revision: revision,
                mutations: vec![CatalogDeltaMutation {
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                }],
                completions: vec![completion(&leased)],
            },
            2_000,
        )
        .expect("reject publication during scan");

    assert_eq!(
        publication.status,
        CatalogDeltaPublicationStatus::RootScanInProgress
    );
    assert_eq!(publication.catalog_revision, revision);
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
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                }],
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
                    outcome: IncrementalReconciliationOutcome::Added,
                    evidence_disposition: DerivedEvidenceDisposition::NoReusableEvidence,
                    remove_location_ids: Vec::new(),
                    upsert_location: Some(location(
                        "asset-new",
                        "location-new",
                        "root-a",
                        "new.jpg",
                    )),
                }],
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
        .update_active_preview(&old_location, Some(&artifact))
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
                    outcome: IncrementalReconciliationOutcome::RenamedOrMoved,
                    evidence_disposition: DerivedEvidenceDisposition::RetainCompatible,
                    remove_location_ids: vec![old_location.location_id],
                    upsert_location: Some(new_location.clone()),
                }],
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
        absolute_path: format!("C:/source/{relative_path}"),
        display_path: format!("C:/source/{relative_path}"),
        relative_path: relative_path.to_owned(),
        preview_path: String::new(),
        file_size: 10,
        created_unix_ms: Some(1_000),
        modified_unix_ms: 1_000,
        file_identity: None,
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
