use std::sync::Arc;
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, OpenFlags};

use crate::domain::{CatalogAutoVacuumMode, CatalogSpaceUsage, ScanError};
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

use super::{
    SCHEMA_VERSION, SQLITE_APPLICATION_ID, SqliteCatalogIdentityGuard, SqliteWritePermit,
    SqliteWritePreemptCallback, catalog_admission_path, database_error,
    open_catalog_database_identity_guard, sqlite_write_admission,
};

pub(crate) struct SqliteCatalogSpaceMaintenance {
    path: std::path::PathBuf,
}

struct MaintenanceConnection {
    connection: Connection,
    _identity_guard: SqliteCatalogIdentityGuard,
    _write_permit: SqliteWritePermit,
}

struct InterruptRegistration<'control> {
    control: &'control CatalogMaintenanceControl,
}

impl Drop for InterruptRegistration<'_> {
    fn drop(&mut self) {
        self.control.clear_interrupt();
    }
}

impl SqliteCatalogSpaceMaintenance {
    pub(crate) fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    fn try_open_write(
        &self,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<MaintenanceConnection>, ScanError> {
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let admission = sqlite_write_admission(&catalog_admission_path(&self.path));
        let preempt_control = control.clone();
        let preempt: SqliteWritePreemptCallback = Arc::new(move || preempt_control.preempt());
        let Some(write_permit) = admission.try_acquire_preemptible_maintenance(preempt) else {
            return Ok(CatalogMaintenanceAttempt::Busy);
        };
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let identity_guard = open_catalog_database_identity_guard(&self.path)?;
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let connection = match Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(connection) => connection,
            Err(error) if is_busy(&error) => return Ok(CatalogMaintenanceAttempt::Busy),
            Err(error) => return Err(database_error(error)),
        };
        connection
            .busy_timeout(Duration::ZERO)
            .map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(database_error)?;
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        match inspect_connection(&connection) {
            Ok(_) => Ok(CatalogMaintenanceAttempt::Completed(
                MaintenanceConnection {
                    connection,
                    _identity_guard: identity_guard,
                    _write_permit: write_permit,
                },
            )),
            Err(error) if is_busy(&error) => Ok(CatalogMaintenanceAttempt::Busy),
            Err(error) => Err(database_error(error)),
        }
    }
}

impl CatalogSpaceRepository for SqliteCatalogSpaceMaintenance {
    fn inspect_catalog_space(
        &self,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        let _identity_guard = open_catalog_database_identity_guard(&self.path)?;
        let connection = match Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(connection) => connection,
            Err(error) if is_busy(&error) => return Ok(CatalogMaintenanceAttempt::Busy),
            Err(error) => return Err(database_error(error)),
        };
        connection
            .busy_timeout(Duration::ZERO)
            .map_err(database_error)?;
        match inspect_connection(&connection) {
            Ok(raw) => raw.try_into().map(CatalogMaintenanceAttempt::Completed),
            Err(error) if is_busy(&error) => Ok(CatalogMaintenanceAttempt::Busy),
            Err(error) => Err(database_error(error)),
        }
    }

    fn try_convert_to_incremental(
        &self,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let maintenance = match self.try_open_write(control)? {
            CatalogMaintenanceAttempt::Completed(maintenance) => maintenance,
            CatalogMaintenanceAttempt::Busy => return Ok(CatalogMaintenanceAttempt::Busy),
            CatalogMaintenanceAttempt::Interrupted => {
                return Ok(CatalogMaintenanceAttempt::Interrupted);
            }
        };
        let connection = &maintenance.connection;
        let before: CatalogSpaceUsage = inspect_connection(connection)
            .map_err(database_error)?
            .try_into()?;
        if before.auto_vacuum == CatalogAutoVacuumMode::Incremental {
            return Ok(CatalogMaintenanceAttempt::Completed(before));
        }
        let interrupt = Arc::new(connection.get_interrupt_handle());
        let callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || interrupt.interrupt());
        control.install_interrupt(callback)?;
        let _interrupt_registration = InterruptRegistration { control };
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let progress_control = control.clone();
        connection
            .progress_handler(1_000, Some(move || progress_control.is_interrupted()))
            .map_err(database_error)?;
        let result = match before.auto_vacuum {
            CatalogAutoVacuumMode::None => {
                connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL; VACUUM;")
            }
            CatalogAutoVacuumMode::Full => {
                connection.execute_batch("PRAGMA auto_vacuum = INCREMENTAL;")
            }
            CatalogAutoVacuumMode::Incremental => unreachable!("handled before write setup"),
        };
        connection
            .progress_handler(0, None::<fn() -> bool>)
            .map_err(database_error)?;
        match result {
            Ok(()) if control.is_interrupted() => Ok(CatalogMaintenanceAttempt::Interrupted),
            Ok(()) => {
                let usage: CatalogSpaceUsage = inspect_connection(connection)
                    .map_err(database_error)?
                    .try_into()?;
                if usage.auto_vacuum != CatalogAutoVacuumMode::Incremental {
                    return Err(ScanError::new(
                        "catalog_reclamation_mode_invalid",
                        "Catalog auto-vacuum did not enter incremental mode",
                    ));
                }
                Ok(CatalogMaintenanceAttempt::Completed(usage))
            }
            Err(error) if is_busy(&error) => Ok(CatalogMaintenanceAttempt::Busy),
            Err(error)
                if error.sqlite_error_code() == Some(ErrorCode::OperationInterrupted)
                    || control.is_interrupted() =>
            {
                Ok(CatalogMaintenanceAttempt::Interrupted)
            }
            Err(error) => Err(database_error(error)),
        }
    }

    fn try_reclaim_incremental(
        &self,
        max_pages: u32,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<CatalogSpaceUsage>, ScanError> {
        if max_pages == 0 {
            return Err(ScanError::new(
                "catalog_reclamation_batch_invalid",
                "Catalog reclamation requires a positive page batch",
            ));
        }
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let maintenance = match self.try_open_write(control)? {
            CatalogMaintenanceAttempt::Completed(maintenance) => maintenance,
            CatalogMaintenanceAttempt::Busy => return Ok(CatalogMaintenanceAttempt::Busy),
            CatalogMaintenanceAttempt::Interrupted => {
                return Ok(CatalogMaintenanceAttempt::Interrupted);
            }
        };
        let connection = &maintenance.connection;
        let before: CatalogSpaceUsage = inspect_connection(connection)
            .map_err(database_error)?
            .try_into()?;
        if before.auto_vacuum != CatalogAutoVacuumMode::Incremental {
            return Err(ScanError::new(
                "catalog_reclamation_mode_invalid",
                "Incremental catalog reclamation requires incremental auto-vacuum",
            ));
        }
        let interrupt = Arc::new(connection.get_interrupt_handle());
        let callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || interrupt.interrupt());
        control.install_interrupt(callback)?;
        let _interrupt_registration = InterruptRegistration { control };
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let progress_control = control.clone();
        connection
            .progress_handler(1_000, Some(move || progress_control.is_interrupted()))
            .map_err(database_error)?;
        let result = connection.execute_batch(&format!("PRAGMA incremental_vacuum({max_pages});"));
        connection
            .progress_handler(0, None::<fn() -> bool>)
            .map_err(database_error)?;
        match result {
            Ok(()) if control.is_interrupted() => Ok(CatalogMaintenanceAttempt::Interrupted),
            Ok(()) => inspect_connection(connection)
                .map_err(database_error)?
                .try_into()
                .map(CatalogMaintenanceAttempt::Completed),
            Err(error) if is_busy(&error) => Ok(CatalogMaintenanceAttempt::Busy),
            Err(error)
                if error.sqlite_error_code() == Some(ErrorCode::OperationInterrupted)
                    || control.is_interrupted() =>
            {
                Ok(CatalogMaintenanceAttempt::Interrupted)
            }
            Err(error) => Err(database_error(error)),
        }
    }

    fn try_checkpoint_wal(
        &self,
        control: &CatalogMaintenanceControl,
    ) -> Result<CatalogMaintenanceAttempt<()>, ScanError> {
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let maintenance = match self.try_open_write(control)? {
            CatalogMaintenanceAttempt::Completed(maintenance) => maintenance,
            CatalogMaintenanceAttempt::Busy => return Ok(CatalogMaintenanceAttempt::Busy),
            CatalogMaintenanceAttempt::Interrupted => {
                return Ok(CatalogMaintenanceAttempt::Interrupted);
            }
        };
        let connection = &maintenance.connection;
        let interrupt = Arc::new(connection.get_interrupt_handle());
        let callback: Arc<dyn Fn() + Send + Sync> = Arc::new(move || interrupt.interrupt());
        control.install_interrupt(callback)?;
        let _interrupt_registration = InterruptRegistration { control };
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        let result = connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok(RawWalCheckpoint {
                busy: row.get(0)?,
                log_pages: row.get(1)?,
                checkpointed_pages: row.get(2)?,
            })
        });
        if control.is_interrupted() {
            return Ok(CatalogMaintenanceAttempt::Interrupted);
        }
        match result {
            Ok(result) => result.into_attempt(),
            Err(error) if is_busy(&error) => Ok(CatalogMaintenanceAttempt::Busy),
            Err(error) if error.sqlite_error_code() == Some(ErrorCode::OperationInterrupted) => {
                Ok(CatalogMaintenanceAttempt::Interrupted)
            }
            Err(error) => Err(database_error(error)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RawWalCheckpoint {
    busy: i64,
    log_pages: i64,
    checkpointed_pages: i64,
}

impl RawWalCheckpoint {
    fn into_attempt(self) -> Result<CatalogMaintenanceAttempt<()>, ScanError> {
        match (self.busy, self.log_pages, self.checkpointed_pages) {
            (0, -1, -1) | (0, 0, 0) => Ok(CatalogMaintenanceAttempt::Completed(())),
            (1, log_pages, checkpointed_pages)
                if log_pages >= 0 && checkpointed_pages >= 0 && checkpointed_pages <= log_pages =>
            {
                Ok(CatalogMaintenanceAttempt::Busy)
            }
            _ => Err(ScanError::new(
                "catalog_reclamation_checkpoint_result_invalid",
                "The catalog reported an invalid WAL checkpoint result",
            )),
        }
    }
}

struct RawCatalogSpaceUsage {
    page_size: i64,
    page_count: i64,
    freelist_count: i64,
    auto_vacuum: i64,
}

impl TryFrom<RawCatalogSpaceUsage> for CatalogSpaceUsage {
    type Error = ScanError;

    fn try_from(raw: RawCatalogSpaceUsage) -> Result<Self, Self::Error> {
        let auto_vacuum = match raw.auto_vacuum {
            0 => CatalogAutoVacuumMode::None,
            1 => CatalogAutoVacuumMode::Full,
            2 => CatalogAutoVacuumMode::Incremental,
            _ => {
                return Err(ScanError::new(
                    "catalog_reclamation_mode_invalid",
                    "The catalog reported an unsupported auto-vacuum mode",
                ));
            }
        };
        Ok(Self {
            page_size: u64::try_from(raw.page_size).map_err(|_| invalid_usage())?,
            page_count: u64::try_from(raw.page_count).map_err(|_| invalid_usage())?,
            freelist_count: u64::try_from(raw.freelist_count).map_err(|_| invalid_usage())?,
            auto_vacuum,
        })
    }
}

fn inspect_connection(connection: &Connection) -> rusqlite::Result<RawCatalogSpaceUsage> {
    let application_id: i64 =
        connection.query_row("PRAGMA application_id", [], |row| row.get::<_, i64>(0))?;
    let user_version: i64 =
        connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
    let schema_version =
        connection.query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })?;
    if application_id != SQLITE_APPLICATION_ID
        || user_version != SCHEMA_VERSION
        || schema_version != SCHEMA_VERSION
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(RawCatalogSpaceUsage {
        page_size: connection.query_row("PRAGMA page_size", [], |row| row.get(0))?,
        page_count: connection.query_row("PRAGMA page_count", [], |row| row.get(0))?,
        freelist_count: connection.query_row("PRAGMA freelist_count", [], |row| row.get(0))?,
        auto_vacuum: connection.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?,
    })
}

fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

fn invalid_usage() -> ScanError {
    ScanError::new(
        "catalog_reclamation_usage_invalid",
        "The catalog reported invalid page usage",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;

    use tempfile::tempdir;

    use super::*;
    use crate::adapters::SqliteCatalog;

    #[test]
    fn legacy_catalog_conversion_is_interruptible_and_enables_incremental_mode() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        seed_reclaimable_pages(&path, true);
        let before_size = fs::metadata(&path).expect("legacy catalog metadata").len();
        let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
        let before = completed(maintenance.inspect_catalog_space().expect("inspect legacy"));
        assert_eq!(before.auto_vacuum, CatalogAutoVacuumMode::None);
        assert!(before.freelist_count > 0);

        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
        let after = completed(
            maintenance
                .try_convert_to_incremental(&control)
                .expect("convert legacy catalog"),
        );

        assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::Incremental);
        assert_eq!(after.freelist_count, 0);
        assert!(fs::metadata(&path).expect("compacted metadata").len() < before_size);
    }

    #[test]
    fn full_auto_vacuum_switches_online_to_incremental_mode() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        Connection::open(&path)
            .expect("full-mode connection")
            .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = FULL; VACUUM;")
            .expect("enable full auto-vacuum");
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let before = completed(
            maintenance
                .inspect_catalog_space()
                .expect("inspect full mode"),
        );
        assert_eq!(before.auto_vacuum, CatalogAutoVacuumMode::Full);

        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
        let after = completed(
            maintenance
                .try_convert_to_incremental(&control)
                .expect("switch full mode online"),
        );

        assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::Incremental);
        assert_eq!(after.page_count, before.page_count);
    }

    #[test]
    fn incremental_reclamation_releases_at_most_the_requested_page_batch() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        seed_reclaimable_pages(&path, false);
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let before = completed(
            maintenance
                .inspect_catalog_space()
                .expect("inspect catalog"),
        );
        assert_eq!(before.auto_vacuum, CatalogAutoVacuumMode::Incremental);
        assert!(before.freelist_count > 32);

        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
        let after = completed(
            maintenance
                .try_reclaim_incremental(32, &control)
                .expect("reclaim bounded batch"),
        );

        let reclaimed_pages = before.freelist_count.saturating_sub(after.freelist_count);
        assert!(reclaimed_pages > 0);
        assert!(reclaimed_pages <= 32);
    }

    #[test]
    fn cancelled_reclamation_does_not_open_a_write_window() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let cancelled = Arc::new(AtomicBool::new(true));
        let control = CatalogMaintenanceControl::new(cancelled);

        assert!(matches!(
            maintenance
                .try_convert_to_incremental(&control)
                .expect("cancelled result"),
            CatalogMaintenanceAttempt::Interrupted
        ));
    }

    #[test]
    fn incremental_reclamation_refuses_legacy_none_without_an_implicit_vacuum() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        seed_reclaimable_pages(&path, true);
        let before_size = fs::metadata(&path).expect("legacy catalog metadata").len();
        let maintenance = SqliteCatalogSpaceMaintenance::new(path.clone());
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

        let error = match maintenance.try_reclaim_incremental(32, &control) {
            Err(error) => error,
            Ok(_) => panic!("legacy mode must require explicit conversion"),
        };

        assert_eq!(error.code, "catalog_reclamation_mode_invalid");
        let after = completed(maintenance.inspect_catalog_space().expect("inspect legacy"));
        assert_eq!(after.auto_vacuum, CatalogAutoVacuumMode::None);
        assert_eq!(
            fs::metadata(path).expect("legacy catalog metadata").len(),
            before_size,
        );
    }

    #[test]
    fn incremental_reclamation_rejects_an_empty_page_batch_before_opening() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

        let error = match maintenance.try_reclaim_incremental(0, &control) {
            Err(error) => error,
            Ok(_) => panic!("zero-page batch must fail"),
        };

        assert_eq!(error.code, "catalog_reclamation_batch_invalid");
    }

    #[test]
    fn truncate_checkpoint_waits_for_a_reader_then_truncates_the_wal() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        let writer = Connection::open(&path).expect("writer connection");
        writer
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA wal_autocheckpoint = 0;
                 CREATE TABLE checkpoint_fixture(value INTEGER NOT NULL);
                 INSERT INTO checkpoint_fixture(value) VALUES (1);",
            )
            .expect("seed WAL fixture");
        let reader = Connection::open(&path).expect("reader connection");
        reader.execute_batch("BEGIN;").expect("begin reader");
        let count: i64 = reader
            .query_row("SELECT COUNT(*) FROM checkpoint_fixture", [], |row| {
                row.get(0)
            })
            .expect("pin reader snapshot");
        assert_eq!(count, 1);
        writer
            .execute("INSERT INTO checkpoint_fixture(value) VALUES (2)", [])
            .expect("append WAL frame after reader snapshot");
        let wal_path = sqlite_sidecar_path(&path, "-wal");
        assert!(fs::metadata(&wal_path).expect("active WAL").len() > 0);
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

        assert!(matches!(
            maintenance
                .try_checkpoint_wal(&control)
                .expect("busy checkpoint result"),
            CatalogMaintenanceAttempt::Busy
        ));

        reader.execute_batch("ROLLBACK;").expect("release reader");
        drop(reader);
        completed(
            maintenance
                .try_checkpoint_wal(&control)
                .expect("checkpoint after reader release"),
        );
        assert_eq!(fs::metadata(wal_path).expect("truncated WAL").len(), 0);
    }

    #[test]
    fn truncate_checkpoint_succeeds_when_the_catalog_has_no_wal() {
        let directory = tempdir().expect("catalog directory");
        let path = directory.path().join("ame.sqlite3");
        drop(SqliteCatalog::open(path.clone()).expect("initialize catalog"));
        Connection::open(&path)
            .expect("delete-journal connection")
            .execute_batch("PRAGMA journal_mode = DELETE;")
            .expect("disable WAL mode");
        let maintenance = SqliteCatalogSpaceMaintenance::new(path);
        let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));

        completed(
            maintenance
                .try_checkpoint_wal(&control)
                .expect("no-WAL checkpoint"),
        );
    }

    #[test]
    fn truncate_checkpoint_requires_an_exact_terminal_result() {
        assert!(matches!(
            RawWalCheckpoint {
                busy: 0,
                log_pages: 0,
                checkpointed_pages: 0,
            }
            .into_attempt()
            .expect("terminal WAL result"),
            CatalogMaintenanceAttempt::Completed(())
        ));
        assert!(matches!(
            RawWalCheckpoint {
                busy: 0,
                log_pages: -1,
                checkpointed_pages: -1,
            }
            .into_attempt()
            .expect("no-WAL result"),
            CatalogMaintenanceAttempt::Completed(())
        ));
        assert!(matches!(
            RawWalCheckpoint {
                busy: 1,
                log_pages: 4,
                checkpointed_pages: 3,
            }
            .into_attempt()
            .expect("busy WAL result"),
            CatalogMaintenanceAttempt::Busy
        ));
        assert_eq!(
            RawWalCheckpoint {
                busy: 0,
                log_pages: 4,
                checkpointed_pages: 4,
            }
            .into_attempt()
            .err()
            .expect("TRUNCATE success must report an empty WAL")
            .code,
            "catalog_reclamation_checkpoint_result_invalid"
        );
    }

    fn seed_reclaimable_pages(path: &Path, legacy_mode: bool) {
        let connection = Connection::open(path).expect("seed connection");
        connection
            .busy_timeout(Duration::from_secs(5))
            .expect("seed busy timeout");
        if legacy_mode {
            connection
                .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA auto_vacuum = NONE; VACUUM;")
                .expect("downgrade fixture auto-vacuum");
        }
        connection
            .execute_batch(
                "CREATE TABLE reclamation_fixture(payload BLOB NOT NULL);
                 WITH RECURSIVE rows(value) AS (
                   SELECT 1 UNION ALL SELECT value + 1 FROM rows WHERE value < 4096
                 )
                 INSERT INTO reclamation_fixture(payload)
                 SELECT zeroblob(2048) FROM rows;
                 DELETE FROM reclamation_fixture;",
            )
            .expect("seed reclaimable pages");
    }

    fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        PathBuf::from(sidecar)
    }

    fn completed<T>(attempt: CatalogMaintenanceAttempt<T>) -> T {
        match attempt {
            CatalogMaintenanceAttempt::Completed(value) => value,
            CatalogMaintenanceAttempt::Busy => panic!("maintenance unexpectedly busy"),
            CatalogMaintenanceAttempt::Interrupted => {
                panic!("maintenance unexpectedly interrupted")
            }
        }
    }
}
