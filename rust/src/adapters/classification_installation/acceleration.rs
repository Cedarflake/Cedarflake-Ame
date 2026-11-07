use std::path::Path;

use crate::domain::ScanError;
use crate::ports::classification_acceleration::ClassificationAccelerationRepository;

use super::{ClassificationInstallationStore, open, storage_error};

impl ClassificationAccelerationRepository for ClassificationInstallationStore {
    fn gpu_enabled(&self, path: &Path) -> Result<bool, ScanError> {
        if !path.try_exists().map_err(storage_error)? {
            return Ok(true);
        }
        open(path)?
            .query_row(
                "SELECT gpu_enabled FROM acceleration WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }

    fn set_gpu_enabled(&self, path: &Path, enabled: bool) -> Result<(), ScanError> {
        open(path)?
            .execute(
                "UPDATE acceleration SET gpu_enabled = ?1 WHERE id = 1",
                [enabled],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn legacy_installation_keeps_its_selection_and_defaults_to_gpu() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("classification.sqlite3");
        let store = ClassificationInstallationStore;
        assert!(store.gpu_enabled(&path).unwrap());
        assert!(!path.exists());
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE installation (
                    id INTEGER PRIMARY KEY,
                    models TEXT,
                    runtime TEXT
                );
                INSERT INTO installation VALUES (1, 'retained-models', 'retained-runtime');
                CREATE TABLE download_intent (
                    id INTEGER PRIMARY KEY,
                    generation INTEGER,
                    package TEXT,
                    pending INTEGER
                );
                INSERT INTO download_intent VALUES (1, 4, 'retained-package', 1);
                PRAGMA user_version = 2;",
            )
            .unwrap();
        assert!(store.gpu_enabled(&path).unwrap());
        store.set_gpu_enabled(&path, false).unwrap();
        assert!(!store.gpu_enabled(&path).unwrap());
        assert_eq!(
            connection
                .query_row("SELECT models FROM installation", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "retained-models"
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT generation FROM download_intent WHERE pending = 1",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            4
        );
    }
}
