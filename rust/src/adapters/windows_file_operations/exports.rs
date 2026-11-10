use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_DISPOSITION_INFO, FILE_GENERIC_WRITE, FILE_SHARE_READ, FileDispositionInfo,
    SetFileInformationByHandle,
};

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::ScanError;
use crate::domain::file_operations::{FileOperationDirectory, FileOperationIssueKind};
use crate::ports::file_operation_exports::{FileExportReportWriter, NativeFileExports};

use super::{WindowsFileOperations, rename, scope};

impl NativeFileExports for WindowsFileOperations {
    fn organization_directory(
        &self,
        root: &FileOperationDirectory,
        name: &str,
    ) -> Result<FileOperationDirectory, ScanError> {
        let _lane = super::mutation_lane()
            .map_err(|issue| ScanError::new("file_operation_busy", issue.message))?;
        scope::validate_name(name)
            .map_err(|issue| ScanError::new("file_organization_name_invalid", issue.message))?;
        let guard = scope::open_directory(root, FileOperationIssueKind::DestinationChanged)
            .map_err(|issue| {
                ScanError::new("file_operation_destination_unavailable", issue.message)
            })?;
        let path = guard.operation_path(name);
        match std::fs::create_dir(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(ScanError::new(
                    "file_organization_directory_failed",
                    error.to_string(),
                ));
            }
        }
        // The root guard pins every ancestor. Opening the child rejects files, reparse points
        // and unavailable placeholders before any media request receives this directory identity.
        let visible = std::path::Path::new(&root.path).join(name);
        scope::inspect_directory(&visible.to_string_lossy())
            .map_err(|issue| ScanError::new("file_organization_directory_failed", issue.message))
    }

    fn begin_export_report(
        &self,
        root: &FileOperationDirectory,
    ) -> Result<Box<dyn FileExportReportWriter>, ScanError> {
        let lane = super::mutation_lane().map_err(|issue| report_error(issue.message))?;
        let guard = scope::open_directory(root, FileOperationIssueKind::DestinationChanged)
            .map_err(|issue| report_error(issue.message))?;
        for _ in 0..16 {
            let name = unique_name();
            let temporary = guard.operation_path(&format!(".{name}.tmp"));
            let opened = OpenOptions::new()
                .write(true)
                .create_new(true)
                .access_mode(FILE_GENERIC_WRITE | DELETE)
                .share_mode(FILE_SHARE_READ)
                .open(&temporary);
            match opened {
                Ok(file) => {
                    return Ok(Box::new(ReportWriter {
                        writer: BufWriter::with_capacity(16 * 1024, file),
                        guard,
                        root: root.path.clone(),
                        name,
                        published: false,
                        _lane: lane,
                    }));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(report_error(error.to_string())),
            }
        }
        Err(report_error(
            "Could not allocate a unique result-report filename",
        ))
    }
}

struct ReportWriter {
    writer: BufWriter<File>,
    guard: PublicationGuardedFileDiscovery,
    root: String,
    name: String,
    published: bool,
    _lane: std::sync::MutexGuard<'static, ()>,
}

impl FileExportReportWriter for ReportWriter {
    fn append(&mut self, text: &str) -> Result<(), ScanError> {
        self.writer
            .write_all(text.as_bytes())
            .map_err(|error| report_error(error.to_string()))
    }

    fn finish(mut self: Box<Self>) -> Result<String, ScanError> {
        self.writer
            .flush()
            .and_then(|()| self.writer.get_ref().sync_all())
            .map_err(|error| report_error(error.to_string()))?;
        rename::rename(
            self.writer.get_ref(),
            &self.guard.operation_path(&self.name),
        )
        .map_err(|error| report_error(error.to_string()))?;
        self.published = true;
        Ok(std::path::Path::new(&self.root)
            .join(&self.name)
            .to_string_lossy()
            .into_owned())
    }
}

impl Drop for ReportWriter {
    fn drop(&mut self) {
        if !self.published {
            let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
            // SAFETY: ADR 0042; this is the exact create-new temporary handle owned by the writer,
            // with DELETE access and no write/delete sharing. No existing report or media is opened.
            unsafe {
                SetFileInformationByHandle(
                    self.writer.get_ref().as_raw_handle(),
                    FileDispositionInfo,
                    std::ptr::from_ref(&disposition).cast(),
                    std::mem::size_of_val(&disposition) as u32,
                );
            }
        }
    }
}

fn unique_name() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "Ame-整理结果-{time}-{}.txt",
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn report_error(message: impl Into<String>) -> ScanError {
    ScanError::new("file_export_report_failed", message)
}
