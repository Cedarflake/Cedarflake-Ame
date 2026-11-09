use std::collections::BTreeMap;
use std::sync::Mutex;

use super::*;
use crate::application::file_operations::{
    FileOperationExecutor, FileOperationInspection, inspect_file_operation,
};
use crate::domain::file_operations::{
    FileOperationIssue, FileOperationIssueKind as Kind, FileOperationResult,
};
use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};

struct ControlledNative {
    files: Mutex<BTreeMap<String, FileOperationSource>>,
    calls: Mutex<Vec<FileAction>>,
    lose_removal_receipt: bool,
}

impl ControlledNative {
    fn new(source: &FileOperationSource) -> Self {
        Self {
            files: Mutex::new(BTreeMap::from([(
                source.state.absolute_path.clone(),
                source.clone(),
            )])),
            calls: Mutex::new(Vec::new()),
            lose_removal_receipt: false,
        }
    }
}

impl NativeFileOperations for ControlledNative {
    fn inspect_source(&self, path: &str) -> Result<FileOperationSource, FileOperationIssue> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FileOperationIssue::new(Kind::Unavailable, "missing fixture"))
    }

    fn inspect_directory(&self, _: &str) -> Result<FileOperationDirectory, FileOperationIssue> {
        unreachable!()
    }

    fn destination_occupied(
        &self,
        destination: &FileOperationDestination,
    ) -> Result<bool, FileOperationIssue> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .contains_key(&target_path(destination)))
    }

    fn inspect_location(
        &self,
        location: &FileOperationDestination,
    ) -> Result<Option<FileOperationSource>, FileOperationIssue> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .get(&target_path(location))
            .cloned())
    }

    fn execute(
        &self,
        request: &FileOperationRequest,
        _: &FileOperationControl,
    ) -> FileOperationResult {
        self.calls.lock().unwrap().push(request.action);
        if request.action == FileAction::Move {
            return Err(FileOperationIssue::new(
                Kind::DifferentVolume,
                "fixture volumes differ",
            ));
        }
        assert_eq!(request.action, FileAction::Copy);
        let destination = request.destination.as_ref().unwrap();
        if destination.name == "图片.jpg" {
            return Err(FileOperationIssue::new(
                Kind::Collision,
                "another writer occupied the chosen name",
            ));
        }
        let mut copied = request.source.clone();
        copied.parent = destination.directory.clone();
        copied.state.absolute_path = target_path(destination);
        copied.state.file_identity.as_mut().unwrap().value =
            "0000000000000002:00000000000000000000000000000003".into();
        self.files
            .lock()
            .unwrap()
            .insert(copied.state.absolute_path.clone(), copied.clone());
        Ok(FileOperationReceipt {
            destination: Some(copied),
            recovery: FileOperationRecovery::None,
        })
    }

    fn complete_move(
        &self,
        source: &FileOperationSource,
        destination: &FileOperationSource,
        _: &FileOperationControl,
    ) -> FileOperationResult {
        self.calls
            .lock()
            .unwrap()
            .push(FileAction::DeletePermanently);
        self.files
            .lock()
            .unwrap()
            .remove(&source.state.absolute_path);
        if self.lose_removal_receipt {
            return Err(FileOperationIssue::new(
                Kind::OutcomeUnknown,
                "lost completion callback",
            ));
        }
        Ok(FileOperationReceipt {
            destination: Some(destination.clone()),
            recovery: FileOperationRecovery::None,
        })
    }
}

fn target_path(destination: &FileOperationDestination) -> String {
    std::path::Path::new(&destination.directory.path)
        .join(&destination.name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn file_operation_execution_records_collision_and_recovers_a_lost_move_receipt_without_replay() {
    let (directory, mut journal, authorization) = fixture();
    let mut native = ControlledNative::new(&authorization.request.source);
    native.lose_removal_receipt = true;
    let id = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()[0]
        .id;
    let control = FileOperationControl::default();
    let mut unstarted = authorization.clone();
    unstarted.batch_id = "interrupted-before-start".into();
    let unstarted_id = journal.authorize(&[unstarted]).unwrap()[0].id;
    let retired = FileOperationExecutor::new(&mut journal, &native, &control, &|| 2)
        .reconcile(unstarted_id)
        .unwrap();
    assert_eq!(retired.checkpoint.phase, Phase::Cancelled);
    assert!(native.calls.lock().unwrap().is_empty());
    let record = FileOperationExecutor::new(&mut journal, &native, &control, &|| 2)
        .execute(id)
        .unwrap();
    assert_eq!(record.checkpoint.phase, Phase::Uncertain);
    assert_eq!(
        record.checkpoint.destination.as_ref().unwrap().name,
        "图片(1).jpg"
    );
    assert!(
        !native
            .files
            .lock()
            .unwrap()
            .contains_key(&authorization.request.source.state.absolute_path)
    );
    assert_eq!(native.files.lock().unwrap().len(), 1);
    let calls = native.calls.lock().unwrap().clone();
    drop(journal);
    let mut reopened =
        SqliteFileOperationJournal::open(directory.path().join("catalog.sqlite3")).unwrap();
    let record = FileOperationExecutor::new(&mut reopened, &native, &control, &|| 3)
        .reconcile(id)
        .unwrap();
    assert_eq!(record.checkpoint.phase, Phase::NativeCompleted);
    assert_eq!(
        record
            .checkpoint
            .receipt
            .as_ref()
            .unwrap()
            .destination
            .as_ref()
            .unwrap()
            .state
            .absolute_path,
        "D:\\Destination\\图片(1).jpg"
    );
    assert_eq!(*native.calls.lock().unwrap(), calls);
    assert!(
        FileOperationExecutor::new(&mut reopened, &native, &control, &|| 4)
            .execute(id)
            .is_err()
    );
    assert_eq!(*native.calls.lock().unwrap(), calls);
}

#[test]
fn file_operation_execution_requires_durable_start_and_copy_before_removing_source() {
    let (_directory, mut journal, authorization) = fixture();
    let native = ControlledNative::new(&authorization.request.source);
    let id = journal
        .authorize(std::slice::from_ref(&authorization))
        .unwrap()[0]
        .id;
    let control = FileOperationControl::default();
    journal.catalog.connection.execute_batch(
        "CREATE TRIGGER reject_started BEFORE UPDATE OF phase ON file_operation_items WHEN NEW.phase = 'executing'
         BEGIN SELECT RAISE(ABORT, 'injected durable intent failure'); END;",
    ).unwrap();
    assert!(
        FileOperationExecutor::new(&mut journal, &native, &control, &|| 2)
            .execute(id)
            .is_err()
    );
    assert!(native.calls.lock().unwrap().is_empty());
    assert_eq!(journal.load(id).unwrap().checkpoint.phase, Phase::Prepared);
    journal.catalog.connection.execute_batch(
        "DROP TRIGGER reject_started;
         CREATE TRIGGER reject_copied BEFORE UPDATE OF phase ON file_operation_items WHEN NEW.phase = 'copied'
         BEGIN SELECT RAISE(ABORT, 'injected copy receipt failure'); END;",
    ).unwrap();
    assert!(
        FileOperationExecutor::new(&mut journal, &native, &control, &|| 3)
            .execute(id)
            .is_err()
    );
    assert_eq!(journal.load(id).unwrap().checkpoint.phase, Phase::Executing);
    assert_eq!(native.files.lock().unwrap().len(), 2);
    assert!(
        !native
            .calls
            .lock()
            .unwrap()
            .contains(&FileAction::DeletePermanently)
    );
    assert!(matches!(
        inspect_file_operation(&journal.load(id).unwrap(), &native),
        FileOperationInspection::NeedsAttention(_)
    ));
    assert!(
        FileOperationExecutor::new(&mut journal, &native, &control, &|| 4)
            .execute(id)
            .is_err()
    );
    assert!(
        native
            .files
            .lock()
            .unwrap()
            .contains_key(&authorization.request.source.state.absolute_path)
    );
}
