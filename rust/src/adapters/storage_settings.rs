use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{ScanError, StorageConfiguration};
use crate::ports::StorageSettingsRepository;

const SETTINGS_SCHEMA_VERSION: i64 = 1;

pub struct SqliteStorageSettings {
    connection: Connection,
}

impl SqliteStorageSettings {
    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ScanError::new(
                    "storage_settings_directory_unavailable",
                    format!("Could not create the settings directory: {error}"),
                )
            })?;
        }
        let connection = Connection::open(path).map_err(settings_error)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS storage_settings_schema (
                   version INTEGER NOT NULL
                 );
                 INSERT INTO storage_settings_schema(version)
                   SELECT 1 WHERE NOT EXISTS (SELECT 1 FROM storage_settings_schema);
                 CREATE TABLE IF NOT EXISTS storage_settings (
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   catalog_path TEXT NOT NULL,
                   preview_root TEXT NOT NULL,
                   preview_budget_bytes INTEGER NOT NULL CHECK(preview_budget_bytes > 0)
                 );",
            )
            .map_err(settings_error)?;
        let version: i64 = connection
            .query_row(
                "SELECT version FROM storage_settings_schema LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(settings_error)?;
        if version != SETTINGS_SCHEMA_VERSION {
            return Err(ScanError::new(
                "storage_settings_schema_unsupported",
                format!("Expected settings schema {SETTINGS_SCHEMA_VERSION}, found {version}"),
            ));
        }
        Ok(Self { connection })
    }
}

impl StorageSettingsRepository for SqliteStorageSettings {
    fn load_or_initialize(
        &mut self,
        defaults: &StorageConfiguration,
    ) -> Result<StorageConfiguration, ScanError> {
        let stored = self
            .connection
            .query_row(
                "SELECT catalog_path, preview_root, preview_budget_bytes
                 FROM storage_settings WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(settings_error)?;
        if let Some((catalog_path, preview_root, preview_budget_bytes)) = stored {
            return Ok(StorageConfiguration {
                catalog_path,
                preview_root,
                preview_budget_bytes: u64::try_from(preview_budget_bytes).map_err(|_| {
                    ScanError::new(
                        "storage_settings_budget_invalid",
                        "The stored preview budget is outside the supported range",
                    )
                })?,
            });
        }
        self.save(defaults)?;
        Ok(defaults.clone())
    }

    fn save(&mut self, configuration: &StorageConfiguration) -> Result<(), ScanError> {
        let preview_budget_bytes =
            i64::try_from(configuration.preview_budget_bytes).map_err(|_| {
                ScanError::new(
                    "storage_settings_budget_invalid",
                    "The preview budget exceeds the settings database range",
                )
            })?;
        let transaction = self.connection.transaction().map_err(settings_error)?;
        transaction
            .execute(
                "INSERT INTO storage_settings(
                   singleton, catalog_path, preview_root, preview_budget_bytes
                 ) VALUES (1, ?1, ?2, ?3)
                 ON CONFLICT(singleton) DO UPDATE SET
                   catalog_path = excluded.catalog_path,
                   preview_root = excluded.preview_root,
                   preview_budget_bytes = excluded.preview_budget_bytes",
                params![
                    configuration.catalog_path,
                    configuration.preview_root,
                    preview_budget_bytes,
                ],
            )
            .map_err(settings_error)?;
        transaction.commit().map_err(settings_error)
    }
}

fn settings_error(error: rusqlite::Error) -> ScanError {
    ScanError::new("storage_settings_database_error", error.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn initializes_and_transactionally_reloads_storage_configuration() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("settings.sqlite3");
        let defaults = StorageConfiguration {
            catalog_path: "C:\\Ame\\catalog.sqlite3".to_owned(),
            preview_root: "C:\\Ame\\previews".to_owned(),
            preview_budget_bytes: 1024,
        };
        let mut settings = SqliteStorageSettings::open(path.clone()).expect("settings");
        assert_eq!(
            settings
                .load_or_initialize(&defaults)
                .expect("initialized settings")
                .preview_budget_bytes,
            1024
        );
        let changed = StorageConfiguration {
            catalog_path: defaults.catalog_path.clone(),
            preview_root: "D:\\Ame\\previews".to_owned(),
            preview_budget_bytes: 2048,
        };
        settings.save(&changed).expect("saved settings");
        drop(settings);

        let mut restored = SqliteStorageSettings::open(path).expect("restored settings");
        let configuration = restored
            .load_or_initialize(&defaults)
            .expect("stored configuration");
        assert_eq!(configuration.preview_root, changed.preview_root);
        assert_eq!(configuration.preview_budget_bytes, 2048);
    }
}
