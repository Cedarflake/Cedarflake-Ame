use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Transaction, params};

use crate::domain::{ExpectedFileState, ScanError};

use super::super::{
    SqliteCatalog, database_error, sqlite_unsigned, stored_file_identity, stored_source_revision,
};

static NEXT_ROSTER_ID: AtomicU64 = AtomicU64::new(1);

// scan_id names the projection, not its content. Every other published location field participates.
const PAYLOAD_COLUMNS: &[&str] = &[
    "asset_id",
    "location_id",
    "root_id",
    "absolute_path",
    "relative_path",
    "preview_path",
    "file_size",
    "created_unix_ms",
    "modified_unix_ms",
    "file_local_time",
    "parent_relative_path",
    "natural_name_key",
    "width",
    "height",
    "preview_status",
    "preview_issue_code",
    "preview_issue_message",
    "metadata_engine_id",
    "metadata_engine_version",
    "capture_local_time",
    "capture_offset_minutes",
    "capture_time_source",
    "capture_raw_value",
    "file_identity_scheme",
    "file_identity_value",
    "source_revision_token",
    "source_generation",
];

const DERIVED_COLUMNS: &[&str] = &[
    "preview_path",
    "width",
    "height",
    "preview_status",
    "preview_issue_code",
    "preview_issue_message",
    "metadata_engine_id",
    "metadata_engine_version",
    "capture_local_time",
    "capture_offset_minutes",
    "capture_time_source",
    "capture_raw_value",
];

pub(crate) struct StagedValidationRoster {
    table: String,
    scan_id: String,
    total_items: u64,
}

pub(crate) struct ValidatedStagingProof(StagedValidationRoster);

#[derive(Clone, Copy)]
pub(crate) enum StagedValidationOutcome {
    FilesystemVerified = 1,
    AwaitingLive = 2,
    SupersededByLive = 3,
}

impl StagedValidationRoster {
    pub(crate) fn capture(catalog: &mut SqliteCatalog, scan_id: &str) -> Result<Self, ScanError> {
        catalog.flush_pending_locations()?;
        let table = format!(
            "ame_scan_validation_{}",
            NEXT_ROSTER_ID.fetch_add(1, Ordering::Relaxed)
        );
        // This scan-exclusive connection uses disk-backed TEMP storage and a bounded page cache;
        // the roster is never a source sidecar or a persistent catalog migration.
        catalog
            .connection
            .execute_batch("PRAGMA temp_store = FILE; PRAGMA temp.cache_size = -2048;")
            .map_err(database_error)?;
        catalog.connection.execute(
            &format!("CREATE TEMP TABLE {table} AS SELECT *, 0 AS validation_outcome FROM asset_locations WHERE scan_id = ?1"),
            [scan_id],
        ).map_err(database_error)?;
        let initialized = (|| {
            catalog
                .connection
                .execute_batch(&format!(
                    "CREATE UNIQUE INDEX temp.{table}_key ON {table}(location_id);"
                ))
                .map_err(database_error)?;
            let count: i64 = catalog
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM temp.{table}"), [], |row| {
                    row.get(0)
                })
                .map_err(database_error)?;
            Ok(Self {
                table: table.clone(),
                scan_id: scan_id.to_owned(),
                total_items: sqlite_unsigned(count, "validation roster count")?,
            })
        })();
        if initialized.is_err() {
            let _ = catalog
                .connection
                .execute_batch(&format!("DROP TABLE temp.{table};"));
        }
        initialized
    }

    pub(crate) fn total_items(&self) -> u64 {
        self.total_items
    }

    pub(crate) fn record_outcome(
        &self,
        catalog: &SqliteCatalog,
        location_id: &str,
        outcome: StagedValidationOutcome,
    ) -> Result<(), ScanError> {
        let mut statement = catalog.connection.prepare_cached(
            &format!("UPDATE temp.{} SET validation_outcome = ?2 WHERE location_id = ?1 AND validation_outcome = 0", self.table),
        ).map_err(database_error)?;
        let updated = statement
            .execute(params![location_id, outcome as i64])
            .map_err(database_error)?;
        if updated != 1 {
            return Err(proof_mismatch());
        }
        Ok(())
    }

    pub(crate) fn load_window(
        &self,
        catalog: &SqliteCatalog,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, String, ExpectedFileState)>, ScanError> {
        let mut statement = catalog
            .connection
            .prepare(&format!(
                "SELECT location_id, relative_path, absolute_path, file_size, modified_unix_ms,
                    file_identity_scheme, file_identity_value, source_revision_token
             FROM temp.{} WHERE (?1 IS NULL OR location_id > ?1)
             ORDER BY location_id LIMIT ?2",
                self.table,
            ))
            .map_err(database_error)?;
        let rows = statement
            .query_map(params![after, i64::from(limit)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(database_error)?;
        rows.map(|row| {
            let (id, path, absolute_path, size, modified_unix_ms, scheme, identity, revision) =
                row.map_err(database_error)?;
            Ok((
                id,
                path,
                ExpectedFileState {
                    absolute_path,
                    file_size: sqlite_unsigned(size, "roster file size")?,
                    modified_unix_ms,
                    file_identity: stored_file_identity(scheme, identity)?,
                    source_revision: stored_source_revision(revision)?,
                },
            ))
        })
        .collect()
    }

    pub(crate) fn superseded_by_live(
        &self,
        catalog: &SqliteCatalog,
        root_id: &str,
        location_id: &str,
    ) -> Result<bool, ScanError> {
        let matches_active = source_lease_matches("staged", "active");
        let matches_roster = source_lease_matches("staged", "roster");
        catalog
            .connection
            .query_row(
                &format!(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?2 AND active_scan_id IS NOT NULL)
             AND (
               (NOT EXISTS(SELECT 1 FROM asset_locations WHERE scan_id = ?1 AND location_id = ?3)
                AND NOT EXISTS(SELECT 1 FROM asset_locations AS active JOIN library_roots AS roots
                    ON active.scan_id = roots.active_scan_id AND active.root_id = roots.id
                    WHERE roots.id = ?2 AND active.location_id = ?3))
               OR EXISTS(SELECT 1 FROM asset_locations AS staged
                 JOIN library_roots AS roots ON roots.id = staged.root_id
                 JOIN asset_locations AS active ON active.scan_id = roots.active_scan_id
                    AND active.root_id = roots.id AND active.location_id = staged.location_id
                 WHERE staged.scan_id = ?1 AND staged.root_id = ?2 AND staged.location_id = ?3
                    AND {matches_active}
                    AND NOT EXISTS(SELECT 1 FROM temp.{} AS roster
                        WHERE roster.location_id = staged.location_id AND {matches_roster}))
             )", self.table,
        ),
                params![self.scan_id, root_id, location_id],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn finish(self, validated_items: u64) -> Result<ValidatedStagingProof, ScanError> {
        if validated_items != self.total_items {
            return Err(ScanError::new(
                "finalization_count_changed",
                "The immutable validation roster was not completely checked",
            ));
        }
        Ok(ValidatedStagingProof(self))
    }
}

impl ValidatedStagingProof {
    pub(crate) fn publish(
        &self,
        catalog: &mut SqliteCatalog,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        super::publish_scan_with_proof(
            catalog,
            &self.0.scan_id,
            root_id,
            asset_count,
            issue_count,
            Some(self),
        )
    }

    pub(super) fn require_current_staging(
        &self,
        transaction: &Transaction<'_>,
        scan_id: &str,
        root_id: &str,
    ) -> Result<(), ScanError> {
        if self.0.scan_id != scan_id {
            return Err(proof_mismatch());
        }
        self.reconcile_live_derived_payload(transaction, scan_id, root_id)?;
        let matches_roster = payload_matches("staged", "roster");
        let matches_active = payload_matches("staged", "active");
        let unproven: bool = transaction.query_row(&format!(
            "SELECT EXISTS(SELECT 1 FROM temp.{0} WHERE validation_outcome = 0)
             OR EXISTS(SELECT 1 FROM asset_locations AS staged
             WHERE staged.scan_id = ?1 AND staged.root_id = ?2
               AND NOT EXISTS(SELECT 1 FROM temp.{0} AS roster
                 WHERE roster.location_id = staged.location_id AND {matches_roster}
                   AND (roster.validation_outcome = 1 OR (
                     roster.validation_outcome = 2
                     AND EXISTS(SELECT 1 FROM library_roots WHERE id = ?2 AND active_scan_id IS NULL)
                     AND EXISTS(SELECT 1 FROM library_change_queue AS queue
                       JOIN library_change_queue_lanes AS lane ON lane.change_id = queue.id
                       JOIN scan_runs AS scan ON scan.id = ?1
                       WHERE queue.root_id = ?2 AND queue.root_generation = scan.root_generation_at_start
                         AND lane.lane = 'p0_live' AND queue.status IN ('pending', 'leased', 'retry_wait')
                         AND (queue.scope = 'root' OR (queue.scope = 'path' AND queue.relative_path = staged.relative_path)))
                   )))
               AND NOT EXISTS(SELECT 1 FROM asset_locations AS active
                 JOIN library_roots AS roots ON roots.id = active.root_id AND roots.active_scan_id = active.scan_id
                 WHERE roots.id = ?2 AND active.location_id = staged.location_id AND {matches_active}))
             OR EXISTS(SELECT 1 FROM temp.{0} AS roster
               WHERE NOT EXISTS(SELECT 1 FROM asset_locations AS staged
                    WHERE staged.scan_id = ?1 AND staged.location_id = roster.location_id)
                 AND EXISTS(SELECT 1 FROM asset_locations AS active
                   JOIN library_roots AS roots ON roots.id = active.root_id AND roots.active_scan_id = active.scan_id
                   WHERE roots.id = ?2 AND active.location_id = roster.location_id))", self.0.table,
        ), params![scan_id, root_id], |row| row.get(0)).map_err(database_error)?;
        if unproven {
            return Err(proof_mismatch());
        }
        Ok(())
    }

    fn reconcile_live_derived_payload(
        &self,
        transaction: &Transaction<'_>,
        scan_id: &str,
        root_id: &str,
    ) -> Result<(), ScanError> {
        let assignments = DERIVED_COLUMNS
            .iter()
            .map(|column| format!("{column} = active.{column}"))
            .collect::<Vec<_>>()
            .join(", ");
        let matches_lease = source_lease_matches("staged", "active");
        let matches_derived = columns_match(DERIVED_COLUMNS.iter().copied(), "staged", "active");
        let matches_roster = payload_matches("staged", "roster");
        // Preview publication may advance derived state only in the active projection. Adopt it
        // only for the identical source lease, without replacing independently verified scan data.
        transaction
            .execute(
                &format!(
                    "UPDATE asset_locations AS staged SET {assignments}
             FROM asset_locations AS active JOIN library_roots AS roots
               ON roots.id = active.root_id AND roots.active_scan_id = active.scan_id
             WHERE staged.scan_id = ?1 AND staged.root_id = ?2 AND roots.id = ?2
               AND active.location_id = staged.location_id AND {matches_lease}
               AND NOT ({matches_derived})
               AND NOT EXISTS(SELECT 1 FROM temp.{} AS roster
                 WHERE roster.location_id = staged.location_id
                   AND roster.validation_outcome = 1 AND {matches_roster})",
                    self.0.table,
                ),
                params![scan_id, root_id],
            )
            .map_err(database_error)?;
        Ok(())
    }

    // TEMP storage belongs to the scan-exclusive connection, which closes on success, failure,
    // detachment or cancellation. Cleanup cannot turn a successful COMMIT into a reported failure.
}

fn payload_matches(left: &str, right: &str) -> String {
    columns_match(PAYLOAD_COLUMNS.iter().copied(), left, right)
}

fn source_lease_matches(left: &str, right: &str) -> String {
    columns_match(
        PAYLOAD_COLUMNS
            .iter()
            .copied()
            .filter(|column| !DERIVED_COLUMNS.contains(column)),
        left,
        right,
    )
}

fn columns_match<'a>(columns: impl Iterator<Item = &'a str>, left: &str, right: &str) -> String {
    columns
        .map(|column| format!("{left}.{column} IS {right}.{column}"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn proof_mismatch() -> ScanError {
    ScanError::new(
        "catalog_scan_validation_proof_mismatch",
        "The staged catalog is neither the validated roster nor a current live publication",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_payload_covers_every_published_location_column() {
        let directory = tempfile::tempdir().expect("catalog");
        let catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        let mut statement = catalog
            .connection
            .prepare("PRAGMA table_info(asset_locations)")
            .expect("columns");
        let mut actual = statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("column names")
            .collect::<Result<Vec<_>, _>>()
            .expect("columns");
        actual.retain(|name| name != "scan_id");
        actual.sort();
        let mut expected = PAYLOAD_COLUMNS
            .iter()
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(actual, expected);
    }

    #[test]
    fn validation_roster_is_connection_local_bounded_and_discarded_on_close() {
        let directory = tempfile::tempdir().expect("catalog");
        let path = directory.path().join("catalog.sqlite3");
        let mut catalog = SqliteCatalog::open(path.clone()).expect("catalog");
        let schema_before: i64 = catalog
            .connection
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .expect("schema version");
        let roster = StagedValidationRoster::capture(&mut catalog, "empty-validation")
            .expect("empty roster");
        assert_eq!(roster.total_items(), 0);
        let temp_mode: i64 = catalog
            .connection
            .query_row("PRAGMA temp_store", [], |row| row.get(0))
            .expect("temporary storage");
        let cache_size: i64 = catalog
            .connection
            .query_row("PRAGMA temp.cache_size", [], |row| row.get(0))
            .expect("bounded page cache");
        assert_eq!(temp_mode, 1);
        assert_eq!(cache_size, -2048);
        let local_count: i64 = catalog.connection.query_row("SELECT COUNT(*) FROM sqlite_temp_master WHERE type = 'table' AND name LIKE 'ame_scan_validation_%'", [], |row| row.get(0)).expect("owned temporary table");
        assert_eq!(local_count, 1);
        drop(roster);
        drop(catalog);
        let connection = rusqlite::Connection::open(path).expect("reopen catalog");
        let remaining: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_temp_master WHERE type = 'table' AND name LIKE 'ame_scan_validation_%'", [], |row| row.get(0)).expect("released roster");
        let schema_after: i64 = connection
            .query_row("PRAGMA schema_version", [], |row| row.get(0))
            .expect("retained schema");
        assert_eq!(remaining, 0);
        assert_eq!(schema_before, schema_after);
    }
}
