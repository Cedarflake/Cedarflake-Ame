use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;

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

    pub fn save(path: &Path, installation: &ClassificationInstallation) -> Result<(), ScanError> {
        if !installation.models.is_absolute() || !installation.runtime.is_absolute() {
            return Err(storage_error("Installation directories must be absolute"));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage_error)?;
        }
        let connection = open(path)?;
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
}

fn open(path: &Path) -> Result<Connection, ScanError> {
    let mut connection = Connection::open(path).map_err(storage_error)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(storage_error)?;
    let transaction = connection.transaction().map_err(storage_error)?;
    let version: u32 = transaction
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_error)?;
    match version {
        0 => transaction
            .execute_batch(
                "CREATE TABLE installation (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                models TEXT NOT NULL CHECK(length(models) > 0),
                runtime TEXT NOT NULL CHECK(length(runtime) > 0)
             ) STRICT;
             PRAGMA user_version = 1;",
            )
            .map_err(storage_error)?,
        1 => {}
        _ => {
            return Err(storage_error(
                "The classification installation format is newer than supported",
            ));
        }
    }
    transaction.commit().map_err(storage_error)?;
    Ok(connection)
}

fn storage_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_installation_unavailable", error.to_string())
}

#[cfg(test)]
mod tests;
