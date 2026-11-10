use super::*;
use crate::domain::file_operation_journal::{
    FileOperationCatalogBinding, FileOperationPhase as Phase,
};
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationDirectory, FileOperationReceipt,
    FileOperationRecovery, FileOperationRequest, FileOperationSource,
};
use crate::domain::{ExpectedFileState, FileIdentityEvidence};
use crate::ports::CatalogRepository;

mod content_proof;
mod execution;
mod exports;
mod publication;
mod selection;
mod task;

fn fixture() -> (
    tempfile::TempDir,
    SqliteFileOperationJournal,
    AuthorizedFileOperation,
) {
    let directory = tempfile::tempdir().unwrap();
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "scan-a",
        "root-a",
        "C:\\Fixture",
        &[("photo-a", None, 30)],
    );
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
         file_identity_value = '0000000000000001:00000000000000000000000000000001',
         source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001';",
        )
        .unwrap();
    let location = catalog.load_active_location("photo-a").unwrap().unwrap();
    let parent = FileOperationDirectory {
        path: "C:\\Fixture".into(),
        identity: FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".into(),
            value: "0000000000000001:00000000000000000000000000000002".into(),
        },
    };
    let authorization = AuthorizedFileOperation {
        batch_id: "batch-one".into(),
        ordinal: 0,
        authorized_unix_ms: 1,
        catalog: FileOperationCatalogBinding {
            asset_id: location.asset_id,
            location_id: location.location_id,
            source_generation: location.source_generation,
            root_id: location.root_id,
            root_generation: 1,
        },
        request: FileOperationRequest {
            action: FileAction::Move,
            source: FileOperationSource {
                parent,
                state: ExpectedFileState {
                    absolute_path: location.absolute_path,
                    file_size: location.file_size,
                    modified_unix_ms: location.modified_unix_ms,
                    file_identity: location.file_identity,
                    source_revision: location.source_revision,
                },
            },
            destination: Some(FileOperationDestination {
                directory: FileOperationDirectory {
                    path: "D:\\Destination".into(),
                    identity: FileIdentityEvidence {
                        scheme: "windows-file-id-128-v1".into(),
                        value: "0000000000000002:00000000000000000000000000000002".into(),
                    },
                },
                name: "图片.jpg".into(),
            }),
        },
    };
    (
        directory,
        SqliteFileOperationJournal { catalog },
        authorization,
    )
}

#[test]
fn file_operation_journal_preserves_authority_and_rolls_back_failed_intent_or_result() {
    let (_directory, mut journal, authorization) = fixture();
    journal.catalog.connection.execute_batch(
        "CREATE TRIGGER reject_file_event BEFORE INSERT ON file_operation_events BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(
        journal
            .authorize(std::slice::from_ref(&authorization))
            .is_err()
    );
    assert!(journal.history(None, 10).unwrap().is_empty());
    let intent_count: i64 = journal
        .catalog
        .connection
        .query_row("SELECT COUNT(*) FROM file_operation_items", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(intent_count, 0);
    journal
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_file_event;")
        .unwrap();
    let record = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()
        .remove(0);
    let repeated = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()
        .remove(0);
    assert_eq!(record.id, repeated.id);
    let mut changed = authorization.clone();
    changed.request.action = FileAction::Copy;
    assert!(journal.authorize(&[changed]).is_err());
    for name in [
        "different.jpg",
        "图片(+1).jpg",
        "图片(01).jpg",
        "图片(1).png",
    ] {
        let mut target = authorization.request.destination.clone().unwrap();
        target.name = name.into();
        assert!(
            journal
                .advance(record.id, 0, FileOperationEvent::Started(Some(target)), 2)
                .is_err(),
            "{name}"
        );
    }
    journal.catalog.connection.execute_batch(
        "CREATE TRIGGER reject_file_head BEFORE UPDATE OF version ON file_operation_items BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(
        journal
            .advance(
                record.id,
                0,
                FileOperationEvent::Started(authorization.request.destination.clone()),
                2
            )
            .is_err()
    );
    assert_eq!(
        journal.load(record.id).unwrap().checkpoint.phase,
        Phase::Prepared
    );
    let events: i64 = journal
        .catalog
        .connection
        .query_row("SELECT COUNT(*) FROM file_operation_events", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(events, 1);
    journal
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_file_head;")
        .unwrap();
    journal
        .advance(record.id, 0, FileOperationEvent::Cancelled, 3)
        .unwrap();
    assert!(
        journal
            .advance(
                record.id,
                0,
                FileOperationEvent::Started(authorization.request.destination.clone()),
                4
            )
            .is_err()
    );
    assert!(
        journal
            .catalog
            .connection
            .execute(
                "UPDATE file_operation_items SET authorization = authorization",
                []
            )
            .is_err()
    );
    assert!(
        journal
            .catalog
            .connection
            .execute("DELETE FROM file_operation_events", [])
            .is_err()
    );
}

#[test]
fn file_operation_journal_upgrades_copied_moves_without_starting_or_retargeting_them() {
    let (directory, mut journal, authorization) = fixture();
    let record = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()
        .remove(0);
    let mut destination = authorization.request.destination.clone().unwrap();
    destination.name = "图片(2).jpg".into();
    journal
        .advance(
            record.id,
            0,
            FileOperationEvent::Started(Some(destination.clone())),
            2,
        )
        .unwrap();
    let mut copied = authorization.request.source.clone();
    copied.parent = destination.directory;
    copied.state.absolute_path = std::path::Path::new(&copied.parent.path)
        .join(&destination.name)
        .to_string_lossy()
        .into_owned();
    copied.state.file_identity.as_mut().unwrap().value =
        "0000000000000002:00000000000000000000000000000003".into();
    journal
        .advance(record.id, 1, FileOperationEvent::Copied(copied.clone()), 3)
        .unwrap();
    super::super::migrations::downgrade_to_v79_for_test(&journal.catalog.connection);
    drop(journal);
    let mut reopened =
        SqliteFileOperationJournal::open(directory.path().join("catalog.sqlite3")).unwrap();
    let pending = reopened.unfinished(0, 1).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].checkpoint.phase, Phase::Copied);
    assert_eq!(
        pending[0]
            .checkpoint
            .copied
            .as_ref()
            .unwrap()
            .state
            .file_identity,
        copied.state.file_identity
    );
    assert!(
        reopened
            .advance(
                record.id,
                2,
                FileOperationEvent::Started(authorization.request.destination),
                4
            )
            .is_err()
    );
    reopened
        .advance(record.id, 2, FileOperationEvent::RemovingSource, 4)
        .unwrap();
    reopened
        .advance(
            record.id,
            3,
            FileOperationEvent::NativeCompleted(FileOperationReceipt {
                destination: Some(copied),
                recovery: FileOperationRecovery::None,
            }),
            5,
        )
        .unwrap();
    validation::validate_history(&reopened.catalog.connection).unwrap();
    assert_eq!(
        reopened.load(record.id).unwrap().checkpoint.phase,
        Phase::NativeCompleted
    );
    assert!(reopened.history(Some(record.id), 1).unwrap().is_empty());
    assert!(
        reopened
            .history(None, FILE_OPERATION_PAGE_SIZE + 1)
            .is_err()
    );
    reopened
        .catalog
        .connection
        .execute(
            "UPDATE file_operation_items SET phase = 'published' WHERE id = ?1",
            [record.id as i64],
        )
        .unwrap();
    assert!(validation::validate_history(&reopened.catalog.connection).is_err());
}

#[test]
fn file_operation_journal_rejects_stale_selection_before_mutation_admission() {
    let (_directory, mut journal, authorization) = fixture();
    let record = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()
        .remove(0);
    journal.catalog.connection.execute("UPDATE asset_locations SET modified_unix_ms = modified_unix_ms + 1 WHERE location_id = ?1", [&authorization.catalog.location_id]).unwrap();
    assert!(
        journal
            .advance(
                record.id,
                0,
                FileOperationEvent::Started(authorization.request.destination),
                2
            )
            .is_err()
    );
    assert_eq!(
        journal.load(record.id).unwrap().checkpoint.phase,
        Phase::Prepared
    );
}
