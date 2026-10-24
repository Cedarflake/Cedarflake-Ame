use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use tempfile::NamedTempFile;

use crate::domain::ScanError;
use crate::domain::organization_archive::{OrganizationArchivePhase, OrganizationArchiveProgress};

use super::{database_error, error, io_error, report};

pub(super) struct Snapshot {
    pub connection: Connection,
    _file: NamedTempFile,
}

pub(super) fn capture(
    source: &Connection,
    catalog_path: &Path,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<Snapshot, ScanError> {
    let parent = catalog_path
        .parent()
        .ok_or_else(|| error("snapshot", "Missing catalog directory"))?;
    let file = tempfile::Builder::new()
        .prefix(".ame-export-snapshot-")
        .tempfile_in(parent)
        .map_err(io_error)?;
    let mut connection =
        Connection::open_with_flags(file.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(database_error)?;
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(database_error)?;
    let started = Instant::now();
    {
        let backup = Backup::new(source, &mut connection).map_err(database_error)?;
        loop {
            let copied = backup.progress();
            report(
                progress,
                OrganizationArchivePhase::Snapshot,
                (copied.pagecount - copied.remaining).max(0) as u64,
            )?;
            if started.elapsed() >= Duration::from_secs(300) {
                return Err(error(
                    "snapshot_busy",
                    "The catalog could not finish a consistent export snapshot",
                ));
            }
            match backup.step(256).map_err(database_error)? {
                StepResult::Done => break,
                StepResult::More => std::thread::yield_now(),
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => return Err(error("snapshot", "Unsupported catalog snapshot state")),
            }
        }
    }
    Ok(Snapshot {
        connection,
        _file: file,
    })
}
