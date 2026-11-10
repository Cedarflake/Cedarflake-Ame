use std::sync::{Arc, Mutex};

use super::*;
use crate::domain::file_operation_exports::{
    FileExportBatch, FileExportReport, FileExportSkip, FileOrganizationRule,
};
use crate::ports::file_operation_exports::{
    FileExportReportWriter, FileExportRepository, NativeFileExports,
};

struct ReportDestination {
    fail: bool,
    written: Arc<Mutex<String>>,
}

impl NativeFileExports for ReportDestination {
    fn organization_directory(
        &self,
        _: &FileOperationDirectory,
        _: &str,
    ) -> Result<FileOperationDirectory, ScanError> {
        unreachable!()
    }
    fn begin_export_report(
        &self,
        _: &FileOperationDirectory,
    ) -> Result<Box<dyn FileExportReportWriter>, ScanError> {
        if self.fail {
            return Err(ScanError::new(
                "file_export_report_failed",
                "Destination unavailable",
            ));
        }
        Ok(Box::new(ReportText(Arc::clone(&self.written))))
    }
}

struct ReportText(Arc<Mutex<String>>);
impl FileExportReportWriter for ReportText {
    fn append(&mut self, text: &str) -> Result<(), ScanError> {
        self.0.lock().unwrap().push_str(text);
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<String, ScanError> {
        Ok("D:\\Destination\\result.txt".into())
    }
}

#[test]
fn file_operation_report_failure_and_retry_preserve_recorded_file_outcomes() {
    let (directory, mut journal, authorization) = fixture();
    let batch = FileExportBatch {
        batch_id: authorization.batch_id.clone(),
        authorized_unix_ms: 1_700_000_000_000,
        action: FileAction::Move,
        rule: FileOrganizationRule::Album,
        selected_files: 2,
        root: authorization
            .request
            .destination
            .as_ref()
            .unwrap()
            .directory
            .clone(),
    };
    journal.begin_export(&batch).unwrap();
    let item = journal.authorize(&[authorization]).unwrap().remove(0);
    journal
        .advance(item.id, 0, FileOperationEvent::Cancelled, 2)
        .unwrap();
    journal
        .record_export_skips(
            &batch.batch_id,
            &[FileExportSkip {
                ordinal: 1,
                source: "C:\\Fixture\\other.jpg".into(),
                reason: "存在多个相册目标".into(),
            }],
        )
        .unwrap();
    assert!(
        journal
            .catalog
            .connection
            .execute(
                "UPDATE file_operation_exports SET root_path = 'C:\\Other'",
                []
            )
            .is_err()
    );
    let written = Arc::new(Mutex::new(String::new()));
    let failed = crate::application::file_operations::exports::generate(
        &mut journal,
        &ReportDestination {
            fail: true,
            written: Arc::clone(&written),
        },
        &batch.batch_id,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(failed.report, FileExportReport::Failed(_)));
    drop(journal);
    let mut journal =
        SqliteFileOperationJournal::open(directory.path().join("catalog.sqlite3")).unwrap();
    let retried = crate::application::file_operations::exports::generate(
        &mut journal,
        &ReportDestination {
            fail: false,
            written: Arc::clone(&written),
        },
        &batch.batch_id,
    )
    .unwrap()
    .unwrap();
    assert!(matches!(retried.report, FileExportReport::Written(_)));
    assert_eq!(journal.load(item.id).unwrap().checkpoint.version, 1);
    assert_eq!(
        journal.load(item.id).unwrap().checkpoint.phase,
        Phase::Cancelled
    );
    let text = written.lock().unwrap();
    assert!(text.contains("2023-11-14 22:13:20 UTC"));
    assert!(text.contains("操作已停止"));
    assert!(text.contains("C:\\Fixture\\other.jpg"));
    assert!(text.contains("存在多个相册目标"));
    assert_eq!(journal.batch_count(&batch.batch_id).unwrap(), 1);
}
