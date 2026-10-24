use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::database_error;

pub(super) enum RestorationSchema {
    LocalDecisionsOnly,
    PortableArchives,
}

impl RestorationSchema {
    pub(super) fn read(connection: &Connection) -> Result<Self, ScanError> {
        let version: i64 = connection
            .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                row.get(0)
            })
            .map_err(database_error)?;
        // Migration 49 reconciles decisions before migration 54 installs archive provenance.
        // Missing archive tables in a current catalog must still produce an error.
        if version < 54 {
            Ok(Self::LocalDecisionsOnly)
        } else {
            Ok(Self::PortableArchives)
        }
    }
}
