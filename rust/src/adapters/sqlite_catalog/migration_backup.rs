use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};

use crate::domain::ScanError;

use super::database_error;

const COPY_DEADLINE: Duration = Duration::from_secs(60);
const PAGES_PER_STEP: i32 = 256;

pub(super) fn prepare(
    connection: &Connection,
    catalog_path: &Path,
    target_version: i64,
) -> Result<Option<PathBuf>, ScanError> {
    let has_schema: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE type = 'table' AND name = 'schema_info')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !has_schema {
        return Ok(None);
    }
    let source_version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .map_err(database_error)?;
    if source_version <= 0 || source_version >= target_version {
        return Ok(None);
    }

    let identifier: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(database_error)?;
    let mut name = catalog_path
        .file_name()
        .ok_or_else(|| backup_error("The catalog has no filename"))?
        .to_os_string();
    name.push(format!(
        ".before-v{source_version}-to-v{target_version}-{identifier}.sqlite3"
    ));
    let backup_path = catalog_path.with_file_name(name);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .map_err(|error| backup_error(format!("Cannot reserve an upgrade backup: {error}")))?;

    // A missing completion receipt means interruption, never permission to restore this file.
    // Keep incomplete output for diagnosis; neither retries nor migrations overwrite it.
    create_snapshot(connection, &backup_path, source_version, COPY_DEADLINE)?;
    file.sync_all()
        .map_err(|error| backup_error(format!("Cannot flush the upgrade backup: {error}")))?;
    write_completion_receipt(&backup_path, source_version, target_version)?;
    Ok(Some(backup_path))
}

fn create_snapshot(
    source: &Connection,
    path: &Path,
    source_version: i64,
    deadline: Duration,
) -> Result<(), ScanError> {
    let mut destination = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(database_error)?;
    destination
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    {
        let backup = Backup::new(source, &mut destination).map_err(database_error)?;
        let started = Instant::now();
        loop {
            if started.elapsed() >= deadline {
                return Err(backup_error(
                    "The catalog backup did not finish within its copy budget; the schema was not upgraded",
                ));
            }
            match backup.step(PAGES_PER_STEP).map_err(database_error)? {
                StepResult::Done => break,
                StepResult::More => std::thread::yield_now(),
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => return Err(backup_error("The catalog backup returned an unknown state")),
            }
        }
    }
    let actual_version: i64 = destination
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .map_err(database_error)?;
    let integrity: String = destination
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(database_error)?;
    if actual_version != source_version || integrity != "ok" {
        return Err(backup_error(
            "The pre-upgrade catalog snapshot failed validation",
        ));
    }
    destination
        .execute_batch("PRAGMA journal_mode = DELETE;")
        .map_err(database_error)?;
    destination
        .close()
        .map_err(|(_, error)| database_error(error))
}

fn write_completion_receipt(
    backup_path: &Path,
    source_version: i64,
    target_version: i64,
) -> Result<(), ScanError> {
    let mut name = backup_path.as_os_str().to_os_string();
    name.push(".complete");
    let mut receipt: File = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(PathBuf::from(name))
        .map_err(|error| backup_error(format!("Cannot reserve a backup receipt: {error}")))?;
    write!(
        receipt,
        "ame-catalog-backup-v1\nfrom={source_version}\nto={target_version}\n"
    )
    .and_then(|()| receipt.sync_all())
    .map_err(|error| backup_error(format!("Cannot complete the backup receipt: {error}")))
}

fn backup_error(message: impl Into<String>) -> ScanError {
    ScanError::new("catalog_upgrade_backup_failed", message)
}

#[cfg(test)]
mod tests;
