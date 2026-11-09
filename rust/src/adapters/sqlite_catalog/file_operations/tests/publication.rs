use std::sync::atomic::AtomicBool;

use super::*;
use crate::domain::exact_duplicate_review::{ExactDuplicatePreference, ExactDuplicateReviewKey};
use crate::domain::exact_duplicates::{
    ContentFingerprint, ExactAnalysisPhase, ExactAnalysisProgress, ExactAnalysisRun,
    FingerprintCandidate,
};
use crate::domain::file_operation_publication::FileOperationDestinationPublication;
use crate::domain::organization::OrganizationSelection;
use crate::domain::{
    AssetLocationView, CatalogDeltaBatch, CatalogDeltaMutation, CatalogDeltaPublicationStatus,
    DerivedEvidenceDisposition, IncrementalReconciliationOutcome, LibraryChangeCompletion,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, PreviewStatus,
};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;
use crate::ports::exact_duplicates::ExactDuplicateRepository;
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::organization::OrganizationRepository;
use crate::ports::{IncrementalCatalogRepository, LibraryChangeQueue};

fn complete(
    journal: &mut SqliteFileOperationJournal,
    authorization: &AuthorizedFileOperation,
    receipt: FileOperationReceipt,
) -> FileOperationRecord {
    let mut record = journal
        .authorize(std::slice::from_ref(authorization))
        .unwrap()
        .remove(0);
    record = journal
        .advance(
            record.id,
            record.checkpoint.version,
            FileOperationEvent::Started(authorization.request.destination.clone()),
            2,
        )
        .unwrap();
    if authorization.request.action == FileAction::Move
        && receipt.destination.as_ref().unwrap().state.file_identity
            != authorization.request.source.state.file_identity
    {
        record = journal
            .advance(
                record.id,
                record.checkpoint.version,
                FileOperationEvent::Copied(receipt.destination.clone().unwrap()),
                3,
            )
            .unwrap();
        record = journal
            .advance(
                record.id,
                record.checkpoint.version,
                FileOperationEvent::RemovingSource,
                4,
            )
            .unwrap();
    }
    journal
        .advance(
            record.id,
            record.checkpoint.version,
            FileOperationEvent::NativeCompleted(receipt),
            5,
        )
        .unwrap()
}

fn selection(location: &AssetLocationView) -> OrganizationSelection {
    OrganizationSelection {
        asset_id: location.asset_id.clone(),
        location_id: location.location_id.clone(),
        source_generation: location.source_generation,
    }
}

#[test]
fn file_operation_publication_moves_location_and_favorite_atomically_and_is_idempotent() {
    for notification_first in [false, true] {
        check_move_publication(notification_first);
    }
}

fn check_move_publication(notification_first: bool) {
    let (directory, mut journal, mut authorization) = fixture();
    let original = journal
        .catalog
        .load_active_location("photo-a")
        .unwrap()
        .unwrap();
    let prepared = journal
        .catalog
        .prepare_organization_selection(&[selection(&original)])
        .unwrap();
    journal
        .catalog
        .set_organization_favorite("favorite-before-move", &prepared, true)
        .unwrap();
    let target = authorization.request.destination.as_mut().unwrap();
    target.directory.path = "C:\\Fixture\\organized".into();
    if notification_first {
        target.directory.identity.value =
            "0000000000000001:00000000000000000000000000000003".into();
    }
    let mut output = authorization.request.source.clone();
    output.parent = target.directory.clone();
    output.state.absolute_path = "C:\\Fixture\\organized\\图片.jpg".into();
    if notification_first {
        output.state.source_revision.as_mut().unwrap().value = "0000000000000002".into();
    } else {
        output.state.file_identity.as_mut().unwrap().value =
            "0000000000000002:00000000000000000000000000000003".into();
    }
    let record = complete(
        &mut journal,
        &authorization,
        FileOperationReceipt {
            destination: Some(output.clone()),
            recovery: FileOperationRecovery::None,
        },
    );
    let mut destination = original.clone();
    destination.location_id = "moved-photo".into();
    destination.absolute_path = output.state.absolute_path;
    destination.relative_path = "organized/图片.jpg".into();
    destination.file_identity = output.state.file_identity;
    destination.source_revision = output.state.source_revision;
    destination.source_generation = 0;
    destination.preview_path.clear();
    destination.preview_status = PreviewStatus::Pending;
    let target = FileOperationDestinationPublication {
        root: journal.publication_roots().unwrap().remove(0),
        location: destination,
    };
    if notification_first {
        observe(&mut journal.catalog, &target.location, false);
    }
    let observed_generation = journal
        .catalog
        .load_active_location("moved-photo")
        .unwrap()
        .map(|location| location.source_generation);
    journal.catalog.connection.execute_batch(
        "CREATE TRIGGER reject_publication BEFORE UPDATE OF phase ON file_operation_items WHEN NEW.phase = 'published'
         BEGIN SELECT RAISE(ABORT, 'injected final commit failure'); END;",
    ).unwrap();
    assert!(
        journal
            .publish_operation(record.id, record.checkpoint.version, Some(&target), 6)
            .is_err()
    );
    assert!(
        journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .is_some()
    );
    assert_eq!(
        journal
            .catalog
            .load_active_location("moved-photo")
            .unwrap()
            .map(|location| location.source_generation),
        observed_generation
    );
    assert_eq!(
        journal.load(record.id).unwrap().checkpoint.phase,
        Phase::NativeCompleted
    );
    journal
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_publication;")
        .unwrap();
    let completed = journal
        .publish_operation(record.id, record.checkpoint.version, Some(&target), 7)
        .unwrap();
    assert!(
        journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .is_none()
    );
    let moved = journal
        .catalog
        .load_active_location("moved-photo")
        .unwrap()
        .unwrap();
    assert_eq!(moved.asset_id, original.asset_id);
    assert_ne!(moved.source_generation, original.source_generation);
    if let Some(observed) = observed_generation {
        assert_eq!(moved.source_generation, observed);
    }
    let inspection = journal
        .catalog
        .inspect_organization_selection(&[selection(&moved)])
        .unwrap();
    assert!(inspection.decisions[0].favorite);
    let count: i64 = journal
        .catalog
        .connection
        .query_row(
            "SELECT asset_count FROM scan_runs WHERE id = 'scan-a'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    let repeated = journal
        .publish_operation(record.id, record.checkpoint.version, Some(&target), 8)
        .unwrap();
    assert_eq!(
        repeated.checkpoint.catalog_revision,
        completed.checkpoint.catalog_revision
    );
    drop(journal);
    let reopened =
        SqliteFileOperationJournal::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        reopened.load(record.id).unwrap().checkpoint.phase,
        Phase::Published
    );
}

#[test]
fn file_operation_publication_removes_only_selected_copy_and_persists_remaining_preference() {
    for notification_first in [false, true] {
        check_delete_publication(notification_first);
    }
}

fn check_delete_publication(notification_first: bool) {
    let (_directory, mut journal, mut authorization) = fixture();
    super::super::super::tests::publish_gallery_fixture(
        &mut journal.catalog,
        "scan-b",
        "root-b",
        "D:\\Fixture",
        &[("photo-b", None, 40)],
    );
    journal.catalog.connection.execute_batch(
        "INSERT INTO library_root_publication_namespaces(root_id, root_generation, identity_scheme, identity_value,
          authority_kind, established_catalog_revision, established_unix_ms, updated_unix_ms)
         SELECT root_id, generation, 'windows-file-id-128-v1', '0000000000000001:00000000000000000000000000000002', 'foreground_scan', 0, 0, 0
         FROM library_change_root_state;",
    ).unwrap();
    journal
        .catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "operation-duplicates".into(),
            started_unix_ms: 1,
        })
        .unwrap();
    let key = ExactDuplicateReviewKey {
        digest: "a".repeat(64),
        byte_length: authorization.request.source.state.file_size,
    };
    for id in ["photo-a", "photo-b"] {
        let location = journal.catalog.load_active_location(id).unwrap().unwrap();
        let root = journal
            .catalog
            .load_incremental_catalog_root(&location.root_id)
            .unwrap()
            .unwrap();
        assert!(
            journal
                .catalog
                .publish_fingerprint(
                    "operation-duplicates",
                    &FingerprintCandidate { location, root },
                    &ContentFingerprint {
                        digest: key.digest.clone(),
                        byte_length: key.byte_length
                    },
                    &AtomicBool::new(false)
                )
                .unwrap()
        );
    }
    let mut progress = ExactAnalysisProgress::running("operation-duplicates".into());
    progress.phase = ExactAnalysisPhase::Completed;
    journal.catalog.finish_exact_analysis(&progress).unwrap();
    journal
        .catalog
        .set_duplicate_preference(
            &key,
            Some(&ExactDuplicatePreference {
                location_id: authorization.catalog.location_id.clone(),
                asset_id: authorization.catalog.asset_id.clone(),
                source_generation: authorization.catalog.source_generation,
            }),
        )
        .unwrap();
    authorization.request.action = FileAction::Recycle;
    authorization.request.destination = None;
    let pending = journal.authorize(&[authorization]).unwrap().remove(0);
    let pending = journal
        .advance(pending.id, 0, FileOperationEvent::Started(None), 2)
        .unwrap();
    if notification_first {
        let selected = journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .unwrap();
        observe(&mut journal.catalog, &selected, true);
        assert_eq!(
            journal
                .catalog
                .load_review_members(&key, "")
                .unwrap()
                .decision
                .preferred_location_id
                .as_deref(),
            Some("photo-b")
        );
        assert_eq!(
            journal.load(pending.id).unwrap().checkpoint.phase,
            Phase::Executing
        );
    }
    let record = journal
        .advance(
            pending.id,
            pending.checkpoint.version,
            FileOperationEvent::NativeCompleted(FileOperationReceipt {
                destination: None,
                recovery: FileOperationRecovery::SystemRecycleBin,
            }),
            5,
        )
        .unwrap();
    journal
        .publish_operation(record.id, record.checkpoint.version, None, 7)
        .unwrap();
    assert!(
        journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .is_none()
    );
    assert!(
        journal
            .catalog
            .load_active_location("photo-b")
            .unwrap()
            .is_some()
    );
    let remaining = journal.catalog.load_review_members(&key, "").unwrap();
    assert_eq!(remaining.candidates.len(), 1);
    assert_eq!(
        remaining.decision.preferred_location_id.as_deref(),
        Some("photo-b")
    );
    assert_eq!(
        journal.load(record.id).unwrap().checkpoint.phase,
        Phase::Published
    );
}

fn observe(catalog: &mut SqliteCatalog, location: &AssetLocationView, removed: bool) {
    let root = catalog
        .load_incremental_catalog_root(&location.root_id)
        .unwrap()
        .unwrap();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..Default::default()
    };
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: location.relative_path.clone(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 1_000,
                most_recent_observed_unix_ms: 1_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            1_000,
            policy,
        )
        .unwrap();
    let lease = catalog
        .lease_library_changes(&root.root_id, root.root_generation, 1_000, policy)
        .unwrap()
        .remove(0);
    let publication = catalog
        .publish_catalog_delta(
            &CatalogDeltaBatch {
                root_id: root.root_id,
                root_generation: root.root_generation,
                expected_catalog_revision: root.catalog_revision,
                mutations: vec![CatalogDeltaMutation {
                    change_id: lease.change.id,
                    outcome: if removed {
                        IncrementalReconciliationOutcome::Removed
                    } else {
                        IncrementalReconciliationOutcome::Added
                    },
                    evidence_disposition: if removed {
                        DerivedEvidenceDisposition::RemoveFromCurrentProjection
                    } else {
                        DerivedEvidenceDisposition::NoReusableEvidence
                    },
                    remove_location_ids: if removed {
                        vec![location.location_id.clone()]
                    } else {
                        Vec::new()
                    },
                    upsert_location: (!removed).then(|| location.clone()),
                    retained_preview_expectation: None,
                }],
                terminal_media_evidence: Vec::new(),
                completions: vec![LibraryChangeCompletion {
                    change_id: lease.change.id,
                    lease_generation: lease.lease_generation,
                    issue: None,
                }],
            },
            2_000,
        )
        .unwrap();
    assert_eq!(publication.status, CatalogDeltaPublicationStatus::Applied);
}
