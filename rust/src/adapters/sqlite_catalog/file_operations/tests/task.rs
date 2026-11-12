use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use super::*;
use crate::application::file_operations::FileOperationTask;
use crate::domain::IncrementalCatalogRoot;
use crate::domain::file_operation_publication::FileOperationOutputMetadata;
use crate::domain::file_operation_task::{
    FileOperationTaskKind, FileOperationTaskPhase, FileOperationTaskRequest,
    FileOperationTaskTarget,
};
use crate::domain::file_operations::{
    FileOperationIssue, FileOperationIssueKind, FileOperationResult,
};
use crate::ports::file_operation_output::{FileOperationOutput, FileOperationOutputReader};
use crate::ports::file_operations::{FileOperationControl, NativeFileOperations};

struct BlockedNative {
    entered: mpsc::SyncSender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl crate::ports::file_operation_exports::NativeFileExports for BlockedNative {
    fn organization_directory(
        &self,
        _: &FileOperationDirectory,
        _: &[&str],
    ) -> Result<FileOperationDirectory, ScanError> {
        unreachable!()
    }
    fn begin_export_report(
        &self,
        _: &FileOperationDirectory,
    ) -> Result<Box<dyn crate::ports::file_operation_exports::FileExportReportWriter>, ScanError>
    {
        unreachable!()
    }
}

impl NativeFileOperations for BlockedNative {
    fn verify_content(
        &self,
        _: &FileOperationSource,
        _: &FileOperationContentProof,
        _: &FileOperationControl,
    ) -> Result<(), FileOperationIssue> {
        unreachable!()
    }

    fn inspect_source(&self, _: &str) -> Result<FileOperationSource, FileOperationIssue> {
        unreachable!()
    }
    fn inspect_directory(&self, _: &str) -> Result<FileOperationDirectory, FileOperationIssue> {
        unreachable!()
    }
    fn destination_occupied(
        &self,
        _: &FileOperationDestination,
    ) -> Result<bool, FileOperationIssue> {
        Ok(false)
    }
    fn inspect_location(
        &self,
        _: &FileOperationDestination,
    ) -> Result<Option<FileOperationSource>, FileOperationIssue> {
        unreachable!()
    }

    fn execute(
        &self,
        _: &FileOperationRequest,
        control: &FileOperationControl,
    ) -> FileOperationResult {
        control.publish_copy_progress(7, 20);
        self.entered.send(()).unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        assert!(control.is_cancelled());
        Err(FileOperationIssue::new(
            FileOperationIssueKind::Cancelled,
            "cancelled held native fixture",
        ))
    }

    fn complete_move(
        &self,
        _: &FileOperationSource,
        _: &FileOperationSource,
        _: &FileOperationControl,
    ) -> FileOperationResult {
        unreachable!()
    }
}

enum UnusedOutput {}

impl FileOperationOutput for UnusedOutput {
    fn metadata(&self) -> Option<&FileOperationOutputMetadata> {
        match *self {}
    }
    fn revalidate(&self) -> Result<(), FileOperationIssue> {
        match *self {}
    }
}

impl FileOperationOutputReader for BlockedNative {
    type Output = UnusedOutput;
    fn hold_output(
        &self,
        _: &FileOperationSource,
        _: Option<(&IncrementalCatalogRoot, &str)>,
    ) -> Result<Self::Output, FileOperationIssue> {
        unreachable!()
    }
}

#[test]
fn file_operation_task_cancellation_joins_native_work_and_retires_unstarted_authority() {
    let (directory, mut journal, mut first) = fixture();
    first.request.action = FileAction::Copy;
    let mut records = Vec::new();
    for start in [0, FILE_OPERATION_PAGE_SIZE] {
        let page = (start..(start + FILE_OPERATION_PAGE_SIZE).min(130))
            .map(|ordinal| {
                let mut item = first.clone();
                item.ordinal = ordinal as u64;
                item
            })
            .collect::<Vec<_>>();
        records.extend(journal.authorize(&page).unwrap());
    }
    let request = FileOperationTaskRequest {
        kind: FileOperationTaskKind::Execute,
        target: FileOperationTaskTarget::Batch {
            batch_id: first.batch_id.clone(),
            expected_items: records.len() as u64,
        },
    };
    let (entered, entry) = mpsc::sync_channel(1);
    let (release, held) = mpsc::sync_channel(1);
    let native = BlockedNative {
        entered,
        release: Mutex::new(held),
    };
    let path = directory.path().join("catalog.sqlite3");
    let task = Arc::new(
        FileOperationTask::spawn(
            request.clone(),
            move || SqliteFileOperationJournal::open(path),
            native,
        )
        .unwrap(),
    );
    entry.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(task.snapshot().copied_bytes, 7);
    assert_eq!(
        journal.load(records[0].id).unwrap().checkpoint.phase,
        Phase::Executing
    );
    let (entered, _) = mpsc::sync_channel(1);
    let (_, held) = mpsc::sync_channel(1);
    let another = FileOperationTask::spawn(
        request,
        || -> Result<SqliteFileOperationJournal, ScanError> {
            panic!("a busy lane cannot open another catalog")
        },
        BlockedNative {
            entered,
            release: Mutex::new(held),
        },
    );
    assert!(matches!(another, Err(error) if error.code == "file_operation_busy"));
    task.cancel();
    assert_eq!(task.snapshot().phase, FileOperationTaskPhase::Stopping);
    let closing = Arc::clone(&task);
    let (closed, observed_close) = mpsc::sync_channel(1);
    let close = std::thread::spawn(move || {
        closed.send(closing.close()).unwrap();
    });
    assert!(matches!(
        observed_close.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    observed_close
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    close.join().unwrap();
    let progress = task.snapshot();
    assert_eq!(progress.phase, FileOperationTaskPhase::Cancelled);
    assert_eq!(progress.processed_items, records.len() as u64);
    assert_eq!(progress.cancelled_items, records.len() as u64);
    for record in records {
        assert_eq!(
            journal.load(record.id).unwrap().checkpoint.phase,
            Phase::Cancelled
        );
    }
    assert!(
        journal
            .catalog
            .load_active_location("photo-a")
            .unwrap()
            .is_some()
    );
}
