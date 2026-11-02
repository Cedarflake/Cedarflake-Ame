use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::domain::ScanError;

mod schema;
pub(crate) mod task;
use schema::open;
use task::DownloadIntent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClassificationInstallation {
    pub models: PathBuf,
    pub runtime: PathBuf,
}

pub(crate) struct ClassificationInstallationStore;

impl ClassificationInstallationStore {
    pub fn load(path: &Path) -> Result<Option<ClassificationInstallation>, ScanError> {
        if !path.try_exists().map_err(storage_error)? {
            return Ok(None);
        }
        let connection = open(path)?;
        read_installation(&connection)
    }

    pub fn save(path: &Path, installation: &ClassificationInstallation) -> Result<(), ScanError> {
        let connection = writable_installation(path, installation)?;
        write_installation(&connection, installation)
    }

    pub fn save_if_unchanged(
        path: &Path,
        expected: Option<&ClassificationInstallation>,
        installation: &ClassificationInstallation,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError> {
        Self::publish(path, expected, installation, None, cancelled)
    }

    pub fn publish(
        path: &Path,
        expected: Option<&ClassificationInstallation>,
        installation: &ClassificationInstallation,
        intent: Option<&DownloadIntent>,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError> {
        let mut connection = writable_installation(path, installation)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage_error)?;
        check_publication_cancelled(cancelled)?;
        if read_installation(&transaction)?.as_ref() != expected {
            return Err(ScanError::new(
                "classification_installation_changed",
                "The selected model installation changed while preparing these files",
            ));
        }
        if let Some(intent) = intent {
            task::complete(&transaction, intent)?;
        }
        write_installation(&transaction, installation)?;
        check_publication_cancelled(cancelled)?;
        transaction.commit().map_err(storage_error)
    }
}

fn check_publication_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ScanError::new(
            "classification_cancelled",
            "Model installation was cancelled",
        ))
    } else {
        Ok(())
    }
}

fn writable_installation(
    path: &Path,
    installation: &ClassificationInstallation,
) -> Result<Connection, ScanError> {
    if !installation.models.is_absolute() || !installation.runtime.is_absolute() {
        return Err(storage_error("Installation directories must be absolute"));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage_error)?;
    }
    open(path)
}

fn read_installation(
    connection: &Connection,
) -> Result<Option<ClassificationInstallation>, ScanError> {
    connection
        .query_row(
            "SELECT models, runtime FROM installation WHERE id = 1",
            [],
            |row| {
                Ok(ClassificationInstallation {
                    models: PathBuf::from(row.get::<_, String>(0)?),
                    runtime: PathBuf::from(row.get::<_, String>(1)?),
                })
            },
        )
        .optional()
        .map_err(storage_error)
}

fn write_installation(
    connection: &Connection,
    installation: &ClassificationInstallation,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT INTO installation (id, models, runtime) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET models = excluded.models, runtime = excluded.runtime",
            params![
                installation.models.to_string_lossy(),
                installation.runtime.to_string_lossy()
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn storage_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_installation_unavailable", error.to_string())
}

#[cfg(test)]
mod tests;
