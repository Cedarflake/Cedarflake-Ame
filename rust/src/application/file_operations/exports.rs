use crate::domain::ScanError;
use crate::domain::file_operation_exports::{FileExportRecord, FileExportReport};
use crate::domain::file_operation_journal::{
    FILE_OPERATION_PAGE_SIZE, FileOperationPhase, FileOperationRecord,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_exports::{FileExportReportWriter, NativeFileExports};

pub(crate) fn generate(
    catalog: &mut impl FileOperationCatalog,
    native: &impl NativeFileExports,
    batch_id: &str,
) -> Result<Option<FileExportRecord>, ScanError> {
    let Some(mut record) = catalog.load_export(batch_id)? else {
        return Ok(None);
    };
    let result = write(catalog, native, &record);
    record.report = match result {
        Ok(Some(path)) => FileExportReport::Written(path),
        Ok(None) => FileExportReport::NotNeeded,
        Err(error) => FileExportReport::Failed(error),
    };
    catalog.save_export_report(batch_id, &record.report)?;
    Ok(Some(record))
}

struct Report<'a, N> {
    native: &'a N,
    record: &'a FileExportRecord,
    writer: Option<Box<dyn FileExportReportWriter>>,
}

impl<N: NativeFileExports> Report<'_, N> {
    fn append(&mut self, text: &str) -> Result<(), ScanError> {
        if self.writer.is_none() {
            let mut writer = self.native.begin_export_report(&self.record.batch.root)?;
            writer.append(&format!("Ame 文件整理结果\r\n操作时间：{}\r\n目标文件夹：{}\r\n所选文件：{}\r\n以下仅列出重命名、跳过、失败或未完成的项目。\r\n",
                self.record.authorized_at_utc, self.record.batch.root.path, self.record.batch.selected_files))?;
            self.writer = Some(writer);
        }
        if let Some(writer) = &mut self.writer {
            writer.append(text)?;
        }
        Ok(())
    }
}

fn write(
    catalog: &impl FileOperationCatalog,
    native: &impl NativeFileExports,
    record: &FileExportRecord,
) -> Result<Option<String>, ScanError> {
    let mut report = Report {
        native,
        record,
        writer: None,
    };
    let mut after = None;
    let mut seen = 0;
    loop {
        let page = catalog.batch_page(&record.batch.batch_id, after, FILE_OPERATION_PAGE_SIZE)?;
        if page.is_empty() {
            break;
        }
        for item in page {
            after = Some(item.authorization.ordinal);
            seen += 1;
            if let Some(text) = exception(&item) {
                report.append(&text)?;
            }
        }
    }
    let mut after = None;
    loop {
        let page = catalog.export_skips(&record.batch.batch_id, after, FILE_OPERATION_PAGE_SIZE)?;
        if page.is_empty() {
            break;
        }
        for item in page {
            after = Some(item.ordinal);
            seen += 1;
            report.append(&format!(
                "\r\n来源：{}\r\n结果：已跳过\r\n原因：{}\r\n",
                item.source, item.reason
            ))?;
        }
    }
    if seen < record.batch.selected_files {
        report.append(&format!(
            "\r\n另有 {} 个所选文件尚未完成准备，没有开始文件操作。\r\n",
            record.batch.selected_files - seen
        ))?;
    }
    report.writer.map(|writer| writer.finish()).transpose()
}

fn exception(record: &FileOperationRecord) -> Option<String> {
    let request = &record.authorization.request;
    let checkpoint = &record.checkpoint;
    let actual = checkpoint
        .receipt
        .as_ref()
        .and_then(|receipt| receipt.destination.as_ref())
        .or(checkpoint.copied.as_ref())
        .map(|source| source.state.absolute_path.clone());
    let expected_name = request
        .destination
        .as_ref()
        .map(|destination| destination.name.as_str());
    let actual_name = actual
        .as_deref()
        .and_then(|path| std::path::Path::new(path).file_name())
        .and_then(|name| name.to_str());
    let reason = match checkpoint.phase {
        FileOperationPhase::Published if expected_name == actual_name => return None,
        FileOperationPhase::Published => "目标已有同名文件，已按编号保存",
        FileOperationPhase::Failed => "文件操作未完成",
        FileOperationPhase::Cancelled => "操作已停止",
        FileOperationPhase::NativeCompleted => "文件操作已完成，图库记录尚待更新",
        FileOperationPhase::Copied => "移动未完成",
        _ => "结果尚未确认，需要检查操作记录",
    };
    let mut text = format!(
        "\r\n来源：{}\r\n结果：{reason}\r\n",
        request.source.state.absolute_path
    );
    if let Some(actual) = actual {
        text.push_str(&format!("实际目标：{actual}\r\n"));
    } else if let Some(target) = &checkpoint.destination {
        text.push_str(&format!(
            "计划目标：{}\r\n",
            std::path::Path::new(&target.directory.path)
                .join(&target.name)
                .display()
        ));
    }
    if let Some(issue) = &checkpoint.issue {
        text.push_str(&format!("详情：{}\r\n", issue.message));
    }
    Some(text)
}
