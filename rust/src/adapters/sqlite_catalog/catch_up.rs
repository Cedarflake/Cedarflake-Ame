use rusqlite::params;

use crate::domain::{LibraryChangeCatchUpCheckpoint, ScanError};
use crate::ports::LibraryChangeCatchUpRepository;

use super::{SqliteCatalog, database_error, sqlite_integer, sqlite_unsigned};

impl LibraryChangeCatchUpRepository for SqliteCatalog {
    fn load_library_change_catch_up_checkpoints(
        &self,
    ) -> Result<Vec<LibraryChangeCatchUpCheckpoint>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT volume_id, journal_id, next_usn, root_set_fingerprint,
                        catalog_revision, updated_unix_ms
                 FROM library_change_catch_up_state ORDER BY volume_id",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(LibraryChangeCatchUpCheckpoint {
                    volume_id: row.get(0)?,
                    journal_id: row.get(1)?,
                    next_usn: row.get(2)?,
                    root_set_fingerprint: row.get(3)?,
                    catalog_revision: sqlite_unsigned(row.get(4)?, "catch-up catalog revision")
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                    updated_unix_ms: row.get(5)?,
                })
            })
            .map_err(database_error)?;
        let mut checkpoints = Vec::new();
        for row in rows {
            let checkpoint = row.map_err(database_error)?;
            validate_checkpoint(&checkpoint)?;
            checkpoints.push(checkpoint);
        }
        Ok(checkpoints)
    }

    fn save_library_change_catch_up_checkpoint(
        &mut self,
        checkpoint: &LibraryChangeCatchUpCheckpoint,
    ) -> Result<(), ScanError> {
        validate_checkpoint(checkpoint)?;
        self.connection
            .execute(
                "INSERT INTO library_change_catch_up_state(
                   volume_id, journal_id, next_usn, root_set_fingerprint,
                   catalog_revision, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(volume_id) DO UPDATE SET
                   journal_id = excluded.journal_id,
                   next_usn = excluded.next_usn,
                   root_set_fingerprint = excluded.root_set_fingerprint,
                   catalog_revision = excluded.catalog_revision,
                   updated_unix_ms = excluded.updated_unix_ms",
                params![
                    checkpoint.volume_id,
                    checkpoint.journal_id,
                    checkpoint.next_usn,
                    checkpoint.root_set_fingerprint,
                    sqlite_integer(checkpoint.catalog_revision, "catch-up catalog revision")?,
                    checkpoint.updated_unix_ms,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }
}

fn validate_checkpoint(checkpoint: &LibraryChangeCatchUpCheckpoint) -> Result<(), ScanError> {
    let journal_is_canonical = checkpoint
        .journal_id
        .parse::<u64>()
        .is_ok_and(|value| value.to_string() == checkpoint.journal_id);
    let usn_is_canonical = checkpoint
        .next_usn
        .parse::<i64>()
        .is_ok_and(|value| value >= 0 && value.to_string() == checkpoint.next_usn);
    let fingerprint_is_valid = checkpoint.root_set_fingerprint.len() == 64
        && checkpoint
            .root_set_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if checkpoint.volume_id.trim().is_empty()
        || checkpoint.volume_id.len() > 512
        || checkpoint.volume_id.contains('\0')
        || !journal_is_canonical
        || !usn_is_canonical
        || !fingerprint_is_valid
        || checkpoint.updated_unix_ms < 0
    {
        return Err(ScanError::new(
            "library_change_catch_up_checkpoint_invalid",
            "The downtime catch-up checkpoint is outside its canonical storage contract",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::domain::LibraryChangeCatchUpCheckpoint;
    use crate::ports::LibraryChangeCatchUpRepository;

    use super::SqliteCatalog;

    #[test]
    fn checkpoint_round_trip_replaces_one_volume_atomically() {
        let temporary = tempdir().expect("temporary directory");
        let mut catalog =
            SqliteCatalog::open(temporary.path().join("catalog.sqlite3")).expect("catalog");
        let mut checkpoint = fixture_checkpoint("12", "34", 7);
        catalog
            .save_library_change_catch_up_checkpoint(&checkpoint)
            .expect("save checkpoint");
        checkpoint.next_usn = "56".to_owned();
        checkpoint.catalog_revision = 8;
        catalog
            .save_library_change_catch_up_checkpoint(&checkpoint)
            .expect("replace checkpoint");

        assert_eq!(
            catalog
                .load_library_change_catch_up_checkpoints()
                .expect("load checkpoints"),
            vec![checkpoint]
        );
    }

    #[test]
    fn noncanonical_unsigned_checkpoint_is_rejected() {
        let temporary = tempdir().expect("temporary directory");
        let mut catalog =
            SqliteCatalog::open(temporary.path().join("catalog.sqlite3")).expect("catalog");
        let checkpoint = fixture_checkpoint("0012", "34", 7);

        let error = catalog
            .save_library_change_catch_up_checkpoint(&checkpoint)
            .expect_err("noncanonical checkpoint");

        assert_eq!(error.code, "library_change_catch_up_checkpoint_invalid");
    }

    fn fixture_checkpoint(
        journal_id: &str,
        next_usn: &str,
        catalog_revision: u64,
    ) -> LibraryChangeCatchUpCheckpoint {
        LibraryChangeCatchUpCheckpoint {
            volume_id: "\\\\?\\Volume{fixture}\\".to_owned(),
            journal_id: journal_id.to_owned(),
            next_usn: next_usn.to_owned(),
            root_set_fingerprint: "a".repeat(64),
            catalog_revision,
            updated_unix_ms: 50,
        }
    }
}
