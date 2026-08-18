use std::collections::HashSet;

use rusqlite::{TransactionBehavior, params};

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

    fn cleanup_obsolete_library_change_catch_up_checkpoints(
        &mut self,
        retained_volume_ids: &[String],
        updated_before_unix_ms: i64,
        limit: u32,
    ) -> Result<u32, ScanError> {
        validate_checkpoint_cleanup(retained_volume_ids, updated_before_unix_ms, limit)?;
        let retained = retained_volume_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let selection_limit = limit
            .checked_add(u32::try_from(retained.len()).map_err(|_| {
                ScanError::new(
                    "library_change_catch_up_cleanup_invalid",
                    "The retained catch-up volume count exceeded the supported range",
                )
            })?)
            .ok_or_else(|| {
                ScanError::new(
                    "library_change_catch_up_cleanup_invalid",
                    "The catch-up cleanup selection bound overflowed",
                )
            })?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let unresolved_gap = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_change_queue
                   WHERE intent_kind = 'freshness_unknown'
                     AND status IN ('pending', 'leased', 'retry_wait')
                 )",
                [],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if unresolved_gap {
            return Ok(0);
        }
        let candidates = {
            let mut statement = transaction
                .prepare(
                    "SELECT volume_id FROM library_change_catch_up_state
                     WHERE updated_unix_ms < ?1
                     ORDER BY updated_unix_ms, volume_id
                     LIMIT ?2",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map(
                    params![updated_before_unix_ms, i64::from(selection_limit)],
                    |row| row.get::<_, String>(0),
                )
                .map_err(database_error)?;
            let mut candidates = Vec::new();
            for row in rows {
                candidates.push(row.map_err(database_error)?);
            }
            candidates
        };
        let mut deleted = 0_u32;
        for volume_id in candidates {
            if retained.contains(volume_id.as_str()) || deleted >= limit {
                continue;
            }
            let changed = transaction
                .execute(
                    "DELETE FROM library_change_catch_up_state WHERE volume_id = ?1",
                    [&volume_id],
                )
                .map_err(database_error)?;
            deleted = deleted.saturating_add(u32::try_from(changed).unwrap_or(u32::MAX));
        }
        transaction.commit().map_err(database_error)?;
        Ok(deleted)
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

fn validate_checkpoint_cleanup(
    retained_volume_ids: &[String],
    updated_before_unix_ms: i64,
    limit: u32,
) -> Result<(), ScanError> {
    let retained = retained_volume_ids.iter().collect::<HashSet<_>>();
    if updated_before_unix_ms < 0
        || limit == 0
        || limit > 1_024
        || retained_volume_ids.len() > 4_096
        || retained.len() != retained_volume_ids.len()
        || retained_volume_ids.iter().any(|volume_id| {
            volume_id.trim().is_empty() || volume_id.len() > 512 || volume_id.contains('\0')
        })
    {
        return Err(ScanError::new(
            "library_change_catch_up_cleanup_invalid",
            "Catch-up checkpoint cleanup must stay within its bounded canonical contract",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::domain::{
        LibraryChangeCatchUpCheckpoint, LibraryChangeIntent, LibraryChangeIntentKind,
        LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
        ScanRequest,
    };
    use crate::ports::{CatalogRepository, LibraryChangeCatchUpRepository, LibraryChangeQueue};

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

    #[test]
    fn obsolete_checkpoint_cleanup_retains_current_volumes() {
        let temporary = tempdir().expect("temporary directory");
        let mut catalog =
            SqliteCatalog::open(temporary.path().join("catalog.sqlite3")).expect("catalog");
        let obsolete = fixture_checkpoint_for("\\\\?\\Volume{obsolete}\\", 10);
        let retained = fixture_checkpoint_for("\\\\?\\Volume{retained}\\", 10);
        catalog
            .save_library_change_catch_up_checkpoint(&obsolete)
            .expect("save obsolete checkpoint");
        catalog
            .save_library_change_catch_up_checkpoint(&retained)
            .expect("save retained checkpoint");

        let deleted = catalog
            .cleanup_obsolete_library_change_catch_up_checkpoints(
                std::slice::from_ref(&retained.volume_id),
                50,
                1,
            )
            .expect("clean obsolete checkpoint");

        assert_eq!(deleted, 1);
        assert_eq!(
            catalog
                .load_library_change_catch_up_checkpoints()
                .expect("load retained checkpoint"),
            vec![retained]
        );
    }

    #[test]
    fn unresolved_freshness_gap_blocks_checkpoint_cleanup() {
        let temporary = tempdir().expect("temporary directory");
        let mut catalog =
            SqliteCatalog::open(temporary.path().join("catalog.sqlite3")).expect("catalog");
        let checkpoint = fixture_checkpoint_for("\\\\?\\Volume{unresolved}\\", 10);
        catalog
            .save_library_change_catch_up_checkpoint(&checkpoint)
            .expect("save checkpoint");
        let request = ScanRequest {
            scan_id: "cleanup-scan".to_owned(),
            root_path: "C:\\fixture".to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        let scan = catalog
            .begin_scan(&request, "cleanup-root", &request.root_path)
            .expect("begin cleanup root scan");
        catalog
            .publish_scan(
                &request.scan_id,
                "cleanup-root",
                scan.accepted_items,
                scan.issue_count,
            )
            .expect("publish cleanup root scan");
        catalog
            .enqueue_library_change_intents(
                &[freshness_gap_intent()],
                20,
                LibraryChangeQueuePolicy::default(),
            )
            .expect("enqueue unresolved gap");

        let deleted = catalog
            .cleanup_obsolete_library_change_catch_up_checkpoints(&[], 50, 1)
            .expect("preserve unresolved checkpoint");

        assert_eq!(deleted, 0);
        assert_eq!(
            catalog
                .load_library_change_catch_up_checkpoints()
                .expect("load preserved checkpoint"),
            vec![checkpoint]
        );
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

    fn fixture_checkpoint_for(
        volume_id: &str,
        updated_unix_ms: i64,
    ) -> LibraryChangeCatchUpCheckpoint {
        LibraryChangeCatchUpCheckpoint {
            volume_id: volume_id.to_owned(),
            updated_unix_ms,
            ..fixture_checkpoint("12", "34", 7)
        }
    }

    fn freshness_gap_intent() -> LibraryChangeIntent {
        LibraryChangeIntent {
            root_id: "cleanup-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            kind: LibraryChangeIntentKind::FreshnessUnknown,
            scope: LibraryChangeScope::Root,
            relative_path: String::new(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::StartupCatchUp,
            first_observed_unix_ms: 20,
            most_recent_observed_unix_ms: 20,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        }
    }
}
