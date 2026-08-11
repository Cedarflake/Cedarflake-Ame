use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::{ScanError, StorageConfiguration};
use crate::ports::StorageSettingsRepository;

const SETTINGS_SCHEMA_VERSION: i64 = 2;

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
        let mut connection = Connection::open(path).map_err(settings_error)?;
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
        match version {
            1 => migrate_settings_schema_v1_to_v2(&mut connection)?,
            SETTINGS_SCHEMA_VERSION => {}
            _ => {
                return Err(ScanError::new(
                    "storage_settings_schema_unsupported",
                    format!("Expected settings schema {SETTINGS_SCHEMA_VERSION}, found {version}"),
                ));
            }
        }
        Ok(Self { connection })
    }
}

fn migrate_settings_schema_v1_to_v2(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(settings_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE preview_root_ownership (
               preview_root TEXT PRIMARY KEY,
               state TEXT NOT NULL CHECK(state IN ('pending', 'retired')),
               recorded_unix_ms INTEGER NOT NULL
             );
             UPDATE storage_settings_schema SET version = 2;",
        )
        .map_err(settings_error)?;
    transaction.commit().map_err(settings_error)
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
        self.save(defaults, None)?;
        Ok(defaults.clone())
    }

    fn save(
        &mut self,
        configuration: &StorageConfiguration,
        pending_preview_root: Option<&str>,
    ) -> Result<(), ScanError> {
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
        transaction
            .execute(
                "DELETE FROM preview_root_ownership WHERE preview_root = ?1",
                [&configuration.preview_root],
            )
            .map_err(settings_error)?;
        if let Some(pending_preview_root) = pending_preview_root
            && pending_preview_root != configuration.preview_root
        {
            transaction
                .execute(
                    "INSERT INTO preview_root_ownership(preview_root, state, recorded_unix_ms)
                     VALUES (?1, 'pending', ?2)
                     ON CONFLICT(preview_root) DO UPDATE SET
                       state = 'pending',
                       recorded_unix_ms = excluded.recorded_unix_ms",
                    params![pending_preview_root, current_unix_ms()?],
                )
                .map_err(settings_error)?;
        }
        transaction.commit().map_err(settings_error)
    }

    fn load_pending_preview_roots(&mut self) -> Result<Vec<String>, ScanError> {
        self.load_preview_roots("pending")
    }

    fn activate_preview_root(&mut self, preview_root: &str) -> Result<(), ScanError> {
        let transaction = self.connection.transaction().map_err(settings_error)?;
        transaction
            .execute(
                "UPDATE preview_root_ownership
                 SET state = 'retired'
                 WHERE state = 'pending' AND preview_root <> ?1",
                [preview_root],
            )
            .map_err(settings_error)?;
        transaction
            .execute(
                "DELETE FROM preview_root_ownership WHERE preview_root = ?1",
                [preview_root],
            )
            .map_err(settings_error)?;
        transaction.commit().map_err(settings_error)
    }

    fn load_retired_preview_roots(&mut self) -> Result<Vec<String>, ScanError> {
        self.load_preview_roots("retired")
    }

    fn forget_retired_preview_root(&mut self, preview_root: &str) -> Result<bool, ScanError> {
        let deleted = self
            .connection
            .execute(
                "DELETE FROM preview_root_ownership
                 WHERE preview_root = ?1 AND state = 'retired'",
                [preview_root],
            )
            .map_err(settings_error)?;
        Ok(deleted != 0)
    }
}

impl SqliteStorageSettings {
    fn load_preview_roots(&mut self, state: &str) -> Result<Vec<String>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT preview_root
                 FROM preview_root_ownership
                 WHERE state = ?1
                 ORDER BY recorded_unix_ms DESC, preview_root",
            )
            .map_err(settings_error)?;
        let roots = statement
            .query_map([state], |row| row.get::<_, String>(0))
            .map_err(settings_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(settings_error)?;
        Ok(roots)
    }
}

fn current_unix_ms() -> Result<i64, ScanError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ScanError::new(
            "storage_settings_clock_unavailable",
            "The system clock is earlier than the Unix epoch",
        )
    })?;
    i64::try_from(elapsed.as_millis()).map_err(|_| {
        ScanError::new(
            "storage_settings_clock_unavailable",
            "The system clock is outside the supported settings range",
        )
    })
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
        settings
            .save(&changed, Some(&defaults.preview_root))
            .expect("saved settings");
        assert_eq!(
            settings
                .load_pending_preview_roots()
                .expect("pending preview roots"),
            vec![defaults.preview_root.clone()]
        );
        settings
            .activate_preview_root(&changed.preview_root)
            .expect("activate preview root");
        assert_eq!(
            settings
                .load_retired_preview_roots()
                .expect("retired preview roots"),
            vec![defaults.preview_root.clone()]
        );
        drop(settings);

        let mut restored = SqliteStorageSettings::open(path).expect("restored settings");
        let configuration = restored
            .load_or_initialize(&defaults)
            .expect("stored configuration");
        assert_eq!(configuration.preview_root, changed.preview_root);
        assert_eq!(configuration.preview_budget_bytes, 2048);
    }

    #[test]
    fn migrates_schema_one_before_tracking_preview_root_ownership() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("settings.sqlite3");
        let connection = Connection::open(&path).expect("legacy settings");
        connection
            .execute_batch(
                "CREATE TABLE storage_settings_schema (version INTEGER NOT NULL);
                 INSERT INTO storage_settings_schema(version) VALUES (1);
                 CREATE TABLE storage_settings (
                   singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                   catalog_path TEXT NOT NULL,
                   preview_root TEXT NOT NULL,
                   preview_budget_bytes INTEGER NOT NULL CHECK(preview_budget_bytes > 0)
                 );",
            )
            .expect("legacy schema");
        drop(connection);

        let mut settings = SqliteStorageSettings::open(path).expect("migrated settings");

        assert!(
            settings
                .load_retired_preview_roots()
                .expect("retired preview roots")
                .is_empty()
        );
    }
}
