use rusqlite::{OptionalExtension, params};

use crate::domain::file_operation_exports::{
    FileExportBatch, FileExportRecord, FileExportReport, FileExportSkip, FileOrganizationRule,
};
use crate::domain::file_operation_journal::invalid;
use crate::domain::file_operations::{FileAction, FileOperationDirectory};
use crate::domain::organization::validate_identifier;
use crate::domain::{FileIdentityEvidence, ScanError};
use crate::ports::file_operation_exports::FileExportRepository;

use super::super::{database_error, sqlite_integer, sqlite_unsigned};
use super::{SqliteFileOperationJournal, check_page};

const COLUMNS: &str = "id, batch_id, authorized_unix_ms, action, rule, selected_files,
    root_path, identity_scheme, identity_value, report_state, report_path, report_error_code,
    report_error_message, (SELECT COUNT(*) FROM file_operation_export_skips AS skipped
        WHERE skipped.batch_id = file_operation_exports.batch_id),
    strftime('%Y-%m-%d %H:%M:%S UTC', authorized_unix_ms / 1000.0, 'unixepoch')";

impl FileExportRepository for SqliteFileOperationJournal {
    fn begin_export(&mut self, batch: &FileExportBatch) -> Result<(), ScanError> {
        validate_identifier(&batch.batch_id)?;
        let action = match batch.action {
            FileAction::Copy => "copy",
            FileAction::Move => "move",
            _ => return Err(invalid("Only copy and move can create export batches")),
        };
        let rule = rule_name(batch.rule);
        let transaction = self.catalog.begin_user_interactive_write()?;
        transaction
            .execute(
                "INSERT INTO file_operation_exports
               (batch_id, authorized_unix_ms, action, rule, selected_files,
                root_path, identity_scheme, identity_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    batch.batch_id,
                    batch.authorized_unix_ms,
                    action,
                    rule,
                    sqlite_integer(batch.selected_files, "export file count")?,
                    batch.root.path,
                    batch.root.identity.scheme,
                    batch.root.identity.value,
                ],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn load_export(&self, batch_id: &str) -> Result<Option<FileExportRecord>, ScanError> {
        self.catalog
            .connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM file_operation_exports WHERE batch_id = ?1"),
                [batch_id],
                read,
            )
            .optional()
            .map_err(database_error)?
            .transpose()
    }

    fn export_history(
        &self,
        before_id: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileExportRecord>, ScanError> {
        check_page(limit)?;
        let mut statement = self
            .catalog
            .connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM file_operation_exports
            WHERE id < ?1 ORDER BY id DESC LIMIT ?2"
            ))
            .map_err(database_error)?;
        let before = before_id
            .map(|id| sqlite_integer(id, "export cursor"))
            .transpose()?
            .unwrap_or(i64::MAX);
        statement
            .query_map(params![before, limit as i64], read)
            .map_err(database_error)?
            .map(|row| row.map_err(database_error)?)
            .collect()
    }

    fn record_export_skips(
        &mut self,
        batch_id: &str,
        skipped: &[FileExportSkip],
    ) -> Result<(), ScanError> {
        if skipped.is_empty() {
            return Ok(());
        }
        check_page(skipped.len())?;
        let transaction = self.catalog.begin_user_interactive_write()?;
        for item in skipped {
            let existing: bool = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM file_operation_items
                 WHERE batch_id = ?1 AND ordinal = ?2)",
                    params![batch_id, sqlite_integer(item.ordinal, "export ordinal")?],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            if existing {
                return Err(invalid("An authorized item cannot also be skipped"));
            }
            transaction.execute(
                "INSERT INTO file_operation_export_skips(batch_id, ordinal, source_path, reason)
                 VALUES (?1, ?2, ?3, ?4)",
                params![batch_id, sqlite_integer(item.ordinal, "export ordinal")?, item.source, item.reason],
            ).map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    fn export_skips(
        &self,
        batch_id: &str,
        after_ordinal: Option<u64>,
        limit: usize,
    ) -> Result<Vec<FileExportSkip>, ScanError> {
        check_page(limit)?;
        let after = after_ordinal
            .map(|value| sqlite_integer(value, "export ordinal"))
            .transpose()?
            .unwrap_or(-1);
        let mut statement = self
            .catalog
            .connection
            .prepare(
                "SELECT ordinal, source_path, reason FROM file_operation_export_skips
            WHERE batch_id = ?1 AND ordinal > ?2 ORDER BY ordinal LIMIT ?3",
            )
            .map_err(database_error)?;
        statement
            .query_map(params![batch_id, after, limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(database_error)?
            .map(|row| {
                let (ordinal, source, reason) = row.map_err(database_error)?;
                Ok(FileExportSkip {
                    ordinal: sqlite_unsigned(ordinal, "export ordinal")?,
                    source,
                    reason,
                })
            })
            .collect()
    }

    fn save_export_report(
        &mut self,
        batch_id: &str,
        report: &FileExportReport,
    ) -> Result<(), ScanError> {
        let (state, path, code, message) = match report {
            FileExportReport::Pending => ("pending", None, None, None),
            FileExportReport::NotNeeded => ("not_needed", None, None, None),
            FileExportReport::Written(path) => ("written", Some(path.as_str()), None, None),
            FileExportReport::Failed(error) => (
                "failed",
                None,
                Some(error.code.as_str()),
                Some(error.message.as_str()),
            ),
        };
        let transaction = self.catalog.begin_user_interactive_write()?;
        let updated = transaction
            .execute(
                "UPDATE file_operation_exports SET report_state = ?2, report_path = ?3,
            report_error_code = ?4, report_error_message = ?5 WHERE batch_id = ?1",
                params![batch_id, state, path, code, message],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(invalid("The export batch is missing"));
        }
        transaction.commit().map_err(database_error)
    }
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<FileExportRecord, ScanError>> {
    let id: i64 = row.get(0)?;
    let action: String = row.get(3)?;
    let rule: String = row.get(4)?;
    let count: i64 = row.get(5)?;
    let state: String = row.get(9)?;
    let report = match state.as_str() {
        "pending" => FileExportReport::Pending,
        "not_needed" => FileExportReport::NotNeeded,
        "written" => FileExportReport::Written(row.get(10)?),
        "failed" => FileExportReport::Failed(ScanError::new(
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
        )),
        _ => return Ok(Err(invalid("Invalid export report state"))),
    };
    let batch_id = row.get(1)?;
    let authorized_unix_ms = row.get(2)?;
    let root = FileOperationDirectory {
        path: row.get(6)?,
        identity: FileIdentityEvidence {
            scheme: row.get(7)?,
            value: row.get(8)?,
        },
    };
    let skipped: i64 = row.get(13)?;
    let authorized_at_utc = row.get(14)?;
    Ok((|| {
        Ok(FileExportRecord {
            id: sqlite_unsigned(id, "export id")?,
            authorized_at_utc,
            batch: FileExportBatch {
                batch_id,
                authorized_unix_ms,
                action: match action.as_str() {
                    "copy" => FileAction::Copy,
                    "move" => FileAction::Move,
                    _ => return Err(invalid("Invalid export action")),
                },
                rule: match rule.as_str() {
                    "direct" => FileOrganizationRule::Direct,
                    "category" => FileOrganizationRule::Category,
                    "album" => FileOrganizationRule::Album,
                    "date" => FileOrganizationRule::Date,
                    "source" => FileOrganizationRule::Source,
                    "album_year" => FileOrganizationRule::AlbumYear,
                    _ => return Err(invalid("Invalid export rule")),
                },
                selected_files: sqlite_unsigned(count, "export file count")?,
                root,
            },
            skipped_files: sqlite_unsigned(skipped, "export skipped count")?,
            report,
        })
    })())
}

fn rule_name(rule: FileOrganizationRule) -> &'static str {
    match rule {
        FileOrganizationRule::Direct => "direct",
        FileOrganizationRule::Category => "category",
        FileOrganizationRule::Album => "album",
        FileOrganizationRule::Date => "date",
        FileOrganizationRule::Source => "source",
        FileOrganizationRule::AlbumYear => "album_year",
    }
}
