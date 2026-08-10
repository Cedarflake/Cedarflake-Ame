use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};

use crate::domain::{
    AssetLocationView, CaptureTimeEvidence, CaptureTimeSource, CatalogCursor, CatalogSnapshot,
    ExpectedFileState, FileIdentityEvidence, GalleryLayoutDateGroup, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GallerySortKey, GalleryTimeAnchor,
    GalleryTimeBucket, GalleryTimeline, LibraryFolderCursor, LibraryFolderPage,
    LibraryRootAvailability, LibraryRootView, PreviewStatus, RecoverableScan, ScanCheckpoint,
    ScanError, ScanIssue, ScanRequest,
};
use crate::ports::CatalogRepository;

use super::user_visible_path;

mod folders;
mod gallery;
mod migrations;

use gallery::{
    build_gallery_asset_query, build_gallery_count_query, build_gallery_layout_manifest_query,
    build_gallery_timeline_query, gallery_cursor_for_asset, resolve_gallery_anchor_cursor,
    validate_gallery_query,
};
use migrations::migrate_schema;

const SCHEMA_VERSION: i64 = 13;
const LOCATION_STAGE_BATCH: usize = 128;
const MAX_LAYOUT_MANIFEST_CHUNK_ITEMS: u32 = 4_096;
const LAYOUT_FLAG_DIMENSIONS_KNOWN: u8 = 1;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub struct SqliteCatalog {
    path: PathBuf,
    connection: Connection,
    pending_locations: Vec<PendingLocation>,
}

#[derive(Clone)]
struct PendingLocation {
    scan_id: String,
    root_id: String,
    location: AssetLocationView,
}

struct StoredLayoutManifestItem {
    location_id: String,
    root_id: String,
    width: u32,
    height: u32,
    date_key: Option<String>,
    primary_missing: bool,
    primary_text: String,
    primary_number: i64,
}

impl SqliteCatalog {
    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                ScanError::new(
                    "catalog_directory_unavailable",
                    format!("Could not create the catalog directory: {error}"),
                )
            })?;
        }
        let mut connection = Connection::open(&path).map_err(database_error)?;
        connection
            .busy_timeout(SQLITE_BUSY_TIMEOUT)
            .map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(database_error)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            connection
                .execute_batch("PRAGMA journal_mode = WAL;")
                .map_err(database_error)?;
        }
        migrate_schema(&mut connection)?;

        Ok(Self {
            path,
            connection,
            pending_locations: Vec::with_capacity(LOCATION_STAGE_BATCH),
        })
    }

    fn flush_pending_locations(&mut self) -> Result<(), ScanError> {
        if self.pending_locations.is_empty() {
            return Ok(());
        }
        let pending = self.pending_locations.clone();
        let transaction = self.connection.transaction().map_err(database_error)?;
        for item in &pending {
            persist_location(&transaction, &item.scan_id, &item.root_id, &item.location)?;
        }
        transaction.commit().map_err(database_error)?;
        self.pending_locations.clear();
        Ok(())
    }
}

impl CatalogRepository for SqliteCatalog {
    fn catalog_path(&self) -> &Path {
        &self.path
    }

    fn begin_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        let now = unix_time_ms();
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET path = excluded.path",
                params![root_id, root_path, now],
            )
            .map_err(database_error)?;
        let existing = transaction
            .query_row(
                "SELECT root_id, status, max_items, max_entries, preview_edge,
                        last_visited_relative_path, visited_entries, accepted_items, issue_count
                 FROM scan_runs WHERE id = ?1",
                [&request.scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?;
        let checkpoint = if let Some((
            stored_root_id,
            status,
            max_items,
            max_entries,
            preview_edge,
            last_visited_relative_path,
            visited_entries,
            accepted_items,
            issue_count,
        )) = existing
        {
            let stored_max_items = optional_sqlite_u32(max_items, "item limit")?;
            let stored_max_entries = optional_sqlite_u32(max_entries, "entry limit")?;
            let stored_preview_edge = sqlite_u32(preview_edge, "preview edge")?;
            let is_paused = status == "paused";
            if status != "running" && !is_paused
                || stored_root_id != root_id
                || stored_max_items != request.max_items
                || stored_max_entries != request.max_entries
                || stored_preview_edge != request.preview_edge
            {
                return Err(ScanError::new(
                    "catalog_scan_resume_mismatch",
                    "The stored scan cannot be resumed with different identity or parameters",
                ));
            }
            if is_paused {
                transaction
                    .execute(
                        "UPDATE scan_runs SET status = 'running' WHERE id = ?1 AND status = 'paused'",
                        [&request.scan_id],
                    )
                    .map_err(database_error)?;
            }
            ScanCheckpoint {
                last_visited_relative_path,
                visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
                accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
                issue_count: sqlite_unsigned(issue_count, "issue count")?,
            }
        } else {
            transaction
                .execute(
                    "INSERT INTO scan_runs(
                       id, root_id, status, started_unix_ms, max_items, max_entries, preview_edge
                     ) VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6)",
                    params![
                        request.scan_id,
                        root_id,
                        now,
                        request.max_items.map(i64::from),
                        request.max_entries.map(i64::from),
                        i64::from(request.preview_edge),
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                    [&request.scan_id],
                )
                .map_err(database_error)?;
            ScanCheckpoint::default()
        };
        transaction.commit().map_err(database_error)?;
        Ok(checkpoint)
    }

    fn has_active_locations(&self) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_roots AS roots
                   JOIN asset_locations AS locations
                     ON locations.scan_id = roots.active_scan_id
                   LIMIT 1
                 )",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_active_location_by_file_identity(
        &self,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.file_identity_scheme = ?1
                   AND locations.file_identity_value = ?2
                 ORDER BY locations.location_id
                 LIMIT 1",
                params![identity.scheme, identity.value],
                read_stored_asset,
            )
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    fn load_active_location(
        &self,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.location_id = ?1
                 LIMIT 1",
                [location_id],
                read_stored_asset,
            )
            .optional()
            .map_err(database_error)?
            .map(stored_asset_view)
            .transpose()
    }

    fn stage_location(
        &mut self,
        scan_id: &str,
        root_id: &str,
        location: &AssetLocationView,
    ) -> Result<(), ScanError> {
        if location.root_id != root_id {
            return Err(ScanError::new(
                "catalog_root_mismatch",
                "The staged location does not belong to the scan root",
            ));
        }
        self.pending_locations.push(PendingLocation {
            scan_id: scan_id.to_owned(),
            root_id: root_id.to_owned(),
            location: location.clone(),
        });
        if self.pending_locations.len() >= LOCATION_STAGE_BATCH {
            self.flush_pending_locations()?;
        }
        Ok(())
    }

    fn update_active_preview(&mut self, location: &AssetLocationView) -> Result<(), ScanError> {
        let file_size = sqlite_integer(location.file_size, "file size")?;
        let updated = self
            .connection
            .execute(
                "UPDATE asset_locations
                 SET preview_path = ?2, width = ?3, height = ?4,
                     preview_status = ?5, preview_issue_code = ?6,
                     preview_issue_message = ?7
                 WHERE location_id = ?1 AND file_size = ?8 AND modified_unix_ms = ?9
                   AND scan_id IN (
                     SELECT active_scan_id FROM library_roots
                     WHERE active_scan_id IS NOT NULL
                   )",
                params![
                    location.location_id,
                    location.preview_path,
                    i64::from(location.width),
                    i64::from(location.height),
                    preview_status_text(&location.preview_status),
                    location.preview_issue_code,
                    location.preview_issue_message,
                    file_size,
                    location.modified_unix_ms,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "active_preview_location_stale",
                "The active catalog location changed before its preview was updated",
            ));
        }
        Ok(())
    }

    fn record_issue(&mut self, scan_id: &str, issue: &ScanIssue) -> Result<(), ScanError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO scan_issues(scan_id, path, code, message)
                 VALUES (?1, ?2, ?3, ?4)",
                params![scan_id, issue.path, issue.code, issue.message],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn checkpoint_scan(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs
                 SET last_visited_relative_path = ?2, visited_entries = ?3,
                     accepted_items = ?4, issue_count = ?5
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_checkpointable",
                "The scan is no longer in a checkpointable running state",
            ));
        }
        Ok(())
    }

    fn load_recoverable_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "running")
    }

    fn load_paused_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "paused")
    }

    fn claim_next_directory(&mut self, scan_id: &str) -> Result<Option<String>, ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current = transaction
            .query_row(
                "SELECT current_directory_relative_path
                 FROM scan_runs WHERE id = ?1 AND status = 'running'",
                [scan_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_not_traversable",
                    "The scan is no longer in a traversable running state",
                )
            })?;
        if current.is_some() {
            transaction.commit().map_err(database_error)?;
            return Ok(current);
        }
        let next = transaction
            .query_row(
                "SELECT id, relative_path FROM scan_directory_frontier
                 WHERE scan_id = ?1 ORDER BY id LIMIT 1",
                [scan_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(database_error)?;
        let Some((frontier_id, relative_path)) = next else {
            transaction.commit().map_err(database_error)?;
            return Ok(None);
        };
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE id = ?1 AND scan_id = ?2",
                params![frontier_id, scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE scan_runs
                 SET current_directory_relative_path = ?2,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, relative_path],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(Some(relative_path))
    }

    fn is_current_directory_enumerated(
        &self,
        scan_id: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT current_directory_enumerated
                 FROM scan_runs
                 WHERE id = ?1 AND status = 'running'
                   AND current_directory_relative_path = ?2",
                params![scan_id, relative_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_directory_mismatch",
                    "The requested directory is not the current running directory",
                )
            })
    }

    fn stage_directory_entries(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        relative_paths: &[String],
    ) -> Result<(), ScanError> {
        if relative_paths.is_empty() {
            return Ok(());
        }
        let transaction = self.connection.transaction().map_err(database_error)?;
        let is_enumerating: bool = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE id = ?1 AND status = 'running'
                     AND current_directory_relative_path = ?2
                     AND current_directory_enumerated = 0
                 )",
                params![scan_id, relative_directory],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_enumerating {
            return Err(ScanError::new(
                "catalog_scan_directory_not_enumerable",
                "Directory entries cannot be staged outside the active enumeration",
            ));
        }
        {
            let mut statement = transaction
                .prepare_cached(
                    "INSERT OR IGNORE INTO scan_directory_entries(
                       scan_id, directory_relative_path, relative_path
                     ) VALUES (?1, ?2, ?3)",
                )
                .map_err(database_error)?;
            for relative_path in relative_paths {
                statement
                    .execute(params![scan_id, relative_directory, relative_path])
                    .map_err(database_error)?;
            }
        }
        transaction.commit().map_err(database_error)
    }

    fn complete_directory_enumeration(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
    ) -> Result<(), ScanError> {
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs SET current_directory_enumerated = 1
                 WHERE id = ?1 AND status = 'running'
                   AND current_directory_relative_path = ?2
                   AND current_directory_enumerated = 0",
                params![scan_id, relative_directory],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_directory_not_enumerable",
                "The current directory enumeration could not be completed",
            ));
        }
        Ok(())
    }

    fn has_directory_entry(
        &self,
        scan_id: &str,
        relative_directory: &str,
        relative_path: &str,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_directory_entries
                   WHERE scan_id = ?1 AND directory_relative_path = ?2
                     AND relative_path = ?3
                 )",
                params![scan_id, relative_directory, relative_path],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_directory_entry_window(
        &self,
        scan_id: &str,
        relative_directory: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, ScanError> {
        if limit == 0 {
            return Err(ScanError::new(
                "directory_entry_window_invalid",
                "The directory entry window must contain at least one item",
            ));
        }
        let is_ready: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE id = ?1 AND status = 'running'
                     AND current_directory_relative_path = ?2
                     AND current_directory_enumerated = 1
                 )",
                params![scan_id, relative_directory],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_ready {
            return Err(ScanError::new(
                "catalog_scan_directory_not_ready",
                "Directory entries cannot be loaded before enumeration completes",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT relative_path FROM scan_directory_entries
                 WHERE scan_id = ?1 AND directory_relative_path = ?2
                   AND (?3 IS NULL OR relative_path > ?3)
                 ORDER BY relative_path
                 LIMIT ?4",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![scan_id, relative_directory, after, i64::from(limit)],
                |row| row.get::<_, String>(0),
            )
            .map_err(database_error)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(database_error)?);
        }
        Ok(entries)
    }

    fn enqueue_directory(&mut self, scan_id: &str, relative_path: &str) -> Result<(), ScanError> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let is_running: bool = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs WHERE id = ?1 AND status = 'running'
                 )",
                [scan_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if !is_running {
            return Err(ScanError::new(
                "catalog_scan_not_traversable",
                "The scan is no longer in a traversable running state",
            ));
        }
        transaction
            .execute(
                "INSERT OR IGNORE INTO scan_directory_frontier(scan_id, relative_path)
                 VALUES (?1, ?2)",
                params![scan_id, relative_path],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn complete_directory(
        &mut self,
        scan_id: &str,
        checkpoint: &ScanCheckpoint,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let current_directory = transaction
            .query_row(
                "SELECT current_directory_relative_path FROM scan_runs
                 WHERE id = ?1 AND status = 'running'",
                [scan_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(database_error)?
            .flatten()
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_directory_not_completable",
                    "The current scan directory is missing",
                )
            })?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries
                 WHERE scan_id = ?1 AND directory_relative_path = ?2",
                params![scan_id, current_directory],
            )
            .map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL,
                     visited_entries = ?2, accepted_items = ?3, issue_count = ?4
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, visited_entries, accepted_items, issue_count],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_directory_not_completable",
                "The current scan directory could not be completed",
            ));
        }
        transaction.commit().map_err(database_error)
    }

    fn pause_scan(&mut self, scan_id: &str, checkpoint: &ScanCheckpoint) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let visited_entries = sqlite_integer(checkpoint.visited_entries, "visited entry count")?;
        let accepted_items = sqlite_integer(checkpoint.accepted_items, "accepted item count")?;
        let issue_count = sqlite_integer(checkpoint.issue_count, "issue count")?;
        let updated = self
            .connection
            .execute(
                "UPDATE scan_runs
                 SET status = 'paused', last_visited_relative_path = ?2,
                     visited_entries = ?3, accepted_items = ?4, issue_count = ?5
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_pausable",
                "The scan is no longer in a pausable running state",
            ));
        }
        Ok(())
    }

    fn count_staged_file_states(&mut self, scan_id: &str) -> Result<u64, ScanError> {
        self.flush_pending_locations()?;
        let count = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1",
                [scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        sqlite_unsigned(count, "staged file state count")
    }

    fn load_staged_file_state_window(
        &self,
        scan_id: &str,
        after_location_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, ExpectedFileState)>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT location_id, absolute_path, file_size, modified_unix_ms,
                        file_identity_scheme, file_identity_value
                 FROM asset_locations
                 WHERE scan_id = ?1 AND (?2 IS NULL OR location_id > ?2)
                 ORDER BY location_id LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![scan_id, after_location_id, i64::from(limit)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let mut states = Vec::new();
        for row in rows {
            let (
                location_id,
                absolute_path,
                file_size,
                modified_unix_ms,
                identity_scheme,
                identity_value,
            ) = row.map_err(database_error)?;
            let file_size = u64::try_from(file_size).map_err(|_| {
                ScanError::new(
                    "catalog_integer_invalid",
                    "A staged file size is outside the supported range",
                )
            })?;
            states.push((
                location_id,
                ExpectedFileState {
                    absolute_path,
                    file_size,
                    modified_unix_ms,
                    file_identity: stored_file_identity(identity_scheme, identity_value)?,
                },
            ));
        }
        Ok(states)
    }

    fn publish_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        self.flush_pending_locations()?;
        let asset_count = sqlite_integer(asset_count, "asset count")?;
        let issue_count = sqlite_integer(issue_count, "issue count")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let previous_active_scan = transaction
            .query_row(
                "SELECT active_scan_id FROM library_roots WHERE id = ?1",
                [root_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(database_error)?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = 'completed', completed_unix_ms = ?2,
                     asset_count = ?3, issue_count = ?4,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, unix_time_ms(), asset_count, issue_count],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_publishable",
                "The scan is no longer in a publishable running state",
            ));
        }
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_roots SET active_scan_id = ?2 WHERE id = ?1",
                params![root_id, scan_id],
            )
            .map_err(database_error)?;
        if let Some(previous_active_scan) = previous_active_scan
            && previous_active_scan != scan_id
        {
            transaction
                .execute(
                    "DELETE FROM asset_locations WHERE scan_id = ?1",
                    [previous_active_scan],
                )
                .map_err(database_error)?;
        }
        delete_orphan_assets(&transaction)?;
        let revision_updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if revision_updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
        transaction.commit().map_err(database_error)
    }

    fn abandon_scan(
        &mut self,
        scan_id: &str,
        status: &str,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        self.pending_locations
            .retain(|pending| pending.scan_id != scan_id);
        let issue_count = sqlite_integer(issue_count, "issue count")?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction
            .execute(
                "UPDATE scan_runs
                 SET status = ?2, completed_unix_ms = ?3, issue_count = ?4,
                     current_directory_relative_path = NULL,
                     current_directory_enumerated = 0,
                     last_visited_relative_path = NULL
                 WHERE id = ?1 AND status = 'running'",
                params![scan_id, status, unix_time_ms(), issue_count],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM scan_directory_frontier WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM scan_directory_entries WHERE scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM asset_locations WHERE scan_id = ?1", [scan_id])
            .map_err(database_error)?;
        delete_orphan_assets(&transaction)?;
        transaction.commit().map_err(database_error)
    }

    fn load_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                "The catalog page limit must be greater than zero",
            ));
        }
        validate_gallery_query(query)?;
        if usize::from(after.is_some())
            + usize::from(before.is_some())
            + usize::from(anchor.is_some())
            > 1
        {
            return Err(ScanError::new(
                "catalog_query_invalid",
                "A gallery request accepts only one page cursor or time anchor",
            ));
        }

        let catalog_path = self.path.to_string_lossy().into_owned();
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        if after.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if before.is_some_and(|cursor| cursor.revision != revision || cursor.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this page cursor was created",
            ));
        }
        if anchor.is_some_and(|value| value.revision != revision || value.query_id != query_id) {
            return Err(ScanError::new(
                "catalog_cursor_stale",
                "The catalog or gallery query changed after this time anchor was created",
            ));
        }

        let roots = load_root_views(&transaction)?;

        let requested = usize::try_from(max_items).map_err(|_| {
            ScanError::new(
                "catalog_page_limit_invalid",
                "The catalog page limit is outside the supported range",
            )
        })?;
        let sql_limit = i64::from(max_items).saturating_add(1);
        let resolved_anchor_cursor = anchor
            .filter(|value| value.item_offset > 0)
            .map(|value| {
                resolve_gallery_anchor_cursor(&transaction, revision, query, query_id, value)
            })
            .transpose()?;
        let effective_after = after.or(resolved_anchor_cursor.as_ref());
        let effective_anchor = if resolved_anchor_cursor.is_some() {
            None
        } else {
            anchor
        };
        let built =
            build_gallery_asset_query(query, effective_after, before, effective_anchor, sql_limit)?;
        let mut asset_statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let mut asset_rows = asset_statement
            .query(params_from_iter(built.parameters.iter()))
            .map_err(database_error)?;
        let mut stored_assets = Vec::new();
        while let Some(row) = asset_rows.next().map_err(database_error)? {
            let stored = read_stored_asset(row).map_err(database_error)?;
            stored_assets.push(stored_asset_view(stored)?);
        }
        drop(asset_rows);
        drop(asset_statement);

        let has_more = stored_assets.len() > requested;
        stored_assets.truncate(requested);
        if before.is_some() {
            stored_assets.reverse();
        }
        let previous_cursor = if before.is_some() && has_more {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if after.is_some() || anchor.is_some() {
            stored_assets
                .first()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let next_cursor = if before.is_some() {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else if has_more {
            stored_assets
                .last()
                .map(|asset| {
                    gallery_cursor_for_asset(&transaction, revision, query_id, query, asset)
                })
                .transpose()?
        } else {
            None
        };
        let assets = stored_assets;

        transaction.commit().map_err(database_error)?;

        Ok(CatalogSnapshot {
            catalog_path,
            revision,
            query_id: query_id.to_owned(),
            roots,
            assets,
            previous_cursor,
            next_cursor,
        })
    }

    fn load_gallery_timeline(
        &mut self,
        query: &GalleryQuery,
        query_id: &str,
    ) -> Result<GalleryTimeline, ScanError> {
        validate_gallery_query(query)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        let built = build_gallery_timeline_query(query);
        let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(built.parameters.iter()), |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(database_error)?;
        let mut total_items = 0_u64;
        let mut buckets = Vec::new();
        for row in rows {
            let (month_key, item_count, aspect_ratio_milli_sum) = row.map_err(database_error)?;
            let item_count = sqlite_unsigned(item_count, "timeline bucket item count")?;
            let aspect_ratio_milli_sum =
                sqlite_unsigned(aspect_ratio_milli_sum, "timeline bucket aspect ratio sum")?;
            total_items = total_items.checked_add(item_count).ok_or_else(|| {
                ScanError::new(
                    "catalog_timeline_count_invalid",
                    "The gallery timeline item count exceeds the supported range",
                )
            })?;
            if !matches!(query.sort_key, GallerySortKey::FileName) {
                buckets.push(GalleryTimeBucket {
                    month_key,
                    item_count,
                    aspect_ratio_milli_sum,
                });
            }
        }
        drop(statement);
        transaction.commit().map_err(database_error)?;

        Ok(GalleryTimeline {
            revision,
            query_id: query_id.to_owned(),
            total_items,
            buckets,
        })
    }

    fn load_gallery_layout_manifest_chunk(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&GalleryLayoutManifestCursor>,
    ) -> Result<GalleryLayoutManifestChunk, ScanError> {
        if max_items == 0 || max_items > MAX_LAYOUT_MANIFEST_CHUNK_ITEMS {
            return Err(ScanError::new(
                "catalog_layout_chunk_limit_invalid",
                format!(
                    "The gallery layout chunk limit must be between 1 and \
                     {MAX_LAYOUT_MANIFEST_CHUNK_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        if after.is_some_and(|cursor| {
            cursor.revision != revision
                || cursor.query_id != query_id
                || cursor.after.revision != revision
                || cursor.after.query_id != query_id
                || cursor.next_ordinal > cursor.total_items
        }) {
            return Err(ScanError::new(
                "catalog_layout_cursor_stale",
                "The catalog or gallery query changed after this layout cursor was created",
            ));
        }

        let total_items = if let Some(cursor) = after {
            cursor.total_items
        } else {
            let count_query = build_gallery_count_query(query);
            let count = transaction
                .query_row(
                    &count_query.sql,
                    params_from_iter(count_query.parameters.iter()),
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            sqlite_unsigned(count, "gallery layout item count")?
        };
        let start_ordinal = after.map_or(0, |cursor| cursor.next_ordinal);
        let requested = usize::try_from(max_items).map_err(|_| {
            ScanError::new(
                "catalog_layout_chunk_limit_invalid",
                "The gallery layout chunk limit is outside the supported range",
            )
        })?;
        let built = build_gallery_layout_manifest_query(
            query,
            after.map(|cursor| &cursor.after),
            i64::from(max_items).saturating_add(1),
        )?;
        let mut statement = transaction.prepare(&built.sql).map_err(database_error)?;
        let mut rows = statement
            .query(params_from_iter(built.parameters.iter()))
            .map_err(database_error)?;
        let mut items = Vec::with_capacity(requested.saturating_add(1));
        while let Some(row) = rows.next().map_err(database_error)? {
            items.push(StoredLayoutManifestItem {
                location_id: row.get(0).map_err(database_error)?,
                root_id: row.get(1).map_err(database_error)?,
                width: sqlite_u32(row.get(2).map_err(database_error)?, "layout width")?,
                height: sqlite_u32(row.get(3).map_err(database_error)?, "layout height")?,
                date_key: row.get(4).map_err(database_error)?,
                primary_missing: row.get::<_, i64>(5).map_err(database_error)? != 0,
                primary_text: row.get(6).map_err(database_error)?,
                primary_number: row.get(7).map_err(database_error)?,
            });
        }
        drop(rows);
        drop(statement);

        let has_more = items.len() > requested;
        items.truncate(requested);
        let loaded_items = u64::try_from(items.len()).map_err(|_| {
            ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout chunk exceeds the supported range",
            )
        })?;
        let next_ordinal = start_ordinal.checked_add(loaded_items).ok_or_else(|| {
            ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout ordinal exceeds the supported range",
            )
        })?;
        if next_ordinal > total_items || (has_more && next_ordinal >= total_items) {
            return Err(ScanError::new(
                "catalog_layout_count_invalid",
                "The gallery layout cursor does not match the complete query count",
            ));
        }

        let next_cursor = if has_more {
            items.last().map(|item| GalleryLayoutManifestCursor {
                revision,
                query_id: query_id.to_owned(),
                total_items,
                next_ordinal,
                after: CatalogCursor {
                    revision,
                    query_id: query_id.to_owned(),
                    primary_missing: item.primary_missing,
                    primary_text: item.primary_text.clone(),
                    primary_number: item.primary_number,
                    root_id: item.root_id.clone(),
                    location_id: item.location_id.clone(),
                },
            })
        } else {
            None
        };

        let mut location_ids = Vec::with_capacity(items.len());
        let mut aspect_ratio_milli = Vec::with_capacity(items.len());
        let mut date_group_indices = Vec::with_capacity(items.len());
        let mut date_groups = Vec::new();
        let mut date_group_lookup = HashMap::new();
        let mut flags = Vec::with_capacity(items.len());
        for item in items {
            let has_dimensions = item.width > 0 && item.height > 0;
            let ratio = if has_dimensions {
                let scaled = u64::from(item.width)
                    .saturating_mul(1_000)
                    .checked_div(u64::from(item.height))
                    .unwrap_or(1_000)
                    .clamp(200, 5_000);
                u16::try_from(scaled).map_err(|_| {
                    ScanError::new(
                        "catalog_layout_ratio_invalid",
                        "The gallery layout aspect ratio exceeds the supported range",
                    )
                })?
            } else {
                1_000
            };
            let date_group_index = if let Some(index) = date_group_lookup.get(&item.date_key) {
                *index
            } else {
                let index = u16::try_from(date_groups.len()).map_err(|_| {
                    ScanError::new(
                        "catalog_layout_date_groups_invalid",
                        "The gallery layout chunk contains too many date groups",
                    )
                })?;
                date_groups.push(GalleryLayoutDateGroup {
                    date_key: item.date_key.clone(),
                });
                date_group_lookup.insert(item.date_key, index);
                index
            };
            location_ids.push(item.location_id);
            aspect_ratio_milli.push(ratio);
            date_group_indices.push(date_group_index);
            flags.push(if has_dimensions {
                LAYOUT_FLAG_DIMENSIONS_KNOWN
            } else {
                0
            });
        }

        transaction.commit().map_err(database_error)?;
        Ok(GalleryLayoutManifestChunk {
            revision,
            query_id: query_id.to_owned(),
            total_items,
            start_ordinal,
            location_ids,
            aspect_ratio_milli,
            date_group_indices,
            date_groups,
            flags,
            next_cursor,
        })
    }

    fn load_folder_page(
        &mut self,
        root_id: &str,
        parent_relative_path: &str,
        max_items: u32,
        after: Option<&LibraryFolderCursor>,
    ) -> Result<LibraryFolderPage, ScanError> {
        folders::load_folder_page(
            &mut self.connection,
            root_id,
            parent_relative_path,
            max_items,
            after,
        )
    }

    fn unregister_root(&mut self, root_id: &str) -> Result<bool, ScanError> {
        self.flush_pending_locations()?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        for table in [
            "scan_directory_frontier",
            "scan_directory_entries",
            "scan_issues",
        ] {
            transaction
                .execute(
                    &format!(
                        "DELETE FROM {table}
                         WHERE scan_id IN (
                           SELECT id FROM scan_runs WHERE root_id = ?1
                         )"
                    ),
                    [root_id],
                )
                .map_err(database_error)?;
        }
        transaction
            .execute("DELETE FROM asset_locations WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM scan_runs WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        let removed = transaction
            .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
            .map_err(database_error)?;
        if removed == 0 {
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        }
        delete_orphan_assets(&transaction)?;
        let revision_updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if revision_updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }
}

fn load_catalog_revision(transaction: &Transaction<'_>) -> Result<u64, ScanError> {
    let revision = transaction
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)?;
    sqlite_unsigned(revision, "catalog revision")
}

fn load_root_views(transaction: &Transaction<'_>) -> Result<Vec<LibraryRootView>, ScanError> {
    let mut statement = transaction
        .prepare(
            "SELECT roots.id, roots.path, roots.active_scan_id, roots.created_unix_ms,
                    COALESCE(scans.asset_count, 0), COALESCE(scans.issue_count, 0)
             FROM library_roots AS roots
             LEFT JOIN scan_runs AS scans ON scans.id = roots.active_scan_id
             ORDER BY roots.created_unix_ms, roots.id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?;
    let mut roots = Vec::new();
    for row in rows {
        let (root_id, path, active_scan_id, created_unix_ms, asset_count, issue_count) =
            row.map_err(database_error)?;
        roots.push(LibraryRootView {
            root_id,
            display_path: user_visible_path(&path),
            path,
            active_scan_id,
            created_unix_ms,
            asset_count: sqlite_unsigned(asset_count, "asset count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
            availability: LibraryRootAvailability::Unknown,
            availability_message: None,
        });
    }
    Ok(roots)
}

fn normalize_relative_folder(value: &str) -> String {
    value.replace('\\', "/").trim_matches('/').to_owned()
}

fn parent_relative_path(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    normalized
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_owned())
        .unwrap_or_default()
}

fn natural_name_key(relative_path: &str) -> String {
    let normalized = relative_path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    let mut output = String::with_capacity(file_name.len());
    let mut characters = file_name.chars().peekable();
    while let Some(character) = characters.next() {
        if !character.is_ascii_digit() {
            output.extend(character.to_lowercase());
            continue;
        }
        let mut digits = String::from(character);
        while characters.peek().is_some_and(char::is_ascii_digit) {
            if let Some(digit) = characters.next() {
                digits.push(digit);
            }
        }
        let significant = digits.trim_start_matches('0');
        let significant = if significant.is_empty() {
            "0"
        } else {
            significant
        };
        output.push('\u{1e}');
        output.push_str(&format!(
            "{:04x}:{significant}:{:04x}",
            significant.len(),
            digits.len()
        ));
        output.push('\u{1f}');
    }
    output
}

fn persist_location(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location: &AssetLocationView,
) -> Result<(), ScanError> {
    let file_size = sqlite_integer(location.file_size, "file size")?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO assets(id, created_unix_ms) VALUES (?1, ?2)",
            params![location.asset_id, unix_time_ms()],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                strftime(
                  '%Y-%m-%dT%H:%M:%f', COALESCE(?9, ?10) / 1000.0,
                  'unixepoch', 'localtime'
                ),
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
              )
             ON CONFLICT(scan_id, location_id) DO UPDATE SET
               asset_id = excluded.asset_id,
               root_id = excluded.root_id,
               absolute_path = excluded.absolute_path,
               relative_path = excluded.relative_path,
               preview_path = excluded.preview_path,
               file_size = excluded.file_size,
               created_unix_ms = excluded.created_unix_ms,
               modified_unix_ms = excluded.modified_unix_ms,
               file_local_time = excluded.file_local_time,
               parent_relative_path = excluded.parent_relative_path,
               natural_name_key = excluded.natural_name_key,
               width = excluded.width,
               height = excluded.height,
               preview_status = excluded.preview_status,
               preview_issue_code = excluded.preview_issue_code,
               preview_issue_message = excluded.preview_issue_message,
               metadata_engine_id = excluded.metadata_engine_id,
               metadata_engine_version = excluded.metadata_engine_version,
               capture_local_time = excluded.capture_local_time,
               capture_offset_minutes = excluded.capture_offset_minutes,
               capture_time_source = excluded.capture_time_source,
               capture_raw_value = excluded.capture_raw_value,
               file_identity_scheme = excluded.file_identity_scheme,
               file_identity_value = excluded.file_identity_value",
            params![
                scan_id,
                location.asset_id,
                location.location_id,
                root_id,
                location.absolute_path,
                location.relative_path,
                location.preview_path,
                file_size,
                location.created_unix_ms,
                location.modified_unix_ms,
                parent_relative_path(&location.relative_path),
                natural_name_key(&location.relative_path),
                i64::from(location.width),
                i64::from(location.height),
                preview_status_text(&location.preview_status),
                location.preview_issue_code,
                location.preview_issue_message,
                location.metadata_engine_id,
                location.metadata_engine_version,
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| &evidence.local_time),
                location
                    .capture_time
                    .as_ref()
                    .and_then(|evidence| evidence.offset_minutes)
                    .map(i64::from),
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| capture_time_source_text(&evidence.source)),
                location
                    .capture_time
                    .as_ref()
                    .map(|evidence| &evidence.raw_value),
                location
                    .file_identity
                    .as_ref()
                    .map(|identity| &identity.scheme),
                location
                    .file_identity
                    .as_ref()
                    .map(|identity| &identity.value),
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

struct StoredAssetRow {
    asset_id: String,
    location_id: String,
    root_id: String,
    absolute_path: String,
    relative_path: String,
    preview_path: String,
    file_size: i64,
    created_unix_ms: Option<i64>,
    modified_unix_ms: i64,
    width: i64,
    height: i64,
    preview_status: String,
    preview_issue_code: Option<String>,
    preview_issue_message: Option<String>,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
    file_identity_scheme: Option<String>,
    file_identity_value: Option<String>,
}

fn read_stored_asset(row: &Row<'_>) -> rusqlite::Result<StoredAssetRow> {
    Ok(StoredAssetRow {
        asset_id: row.get(0)?,
        location_id: row.get(1)?,
        root_id: row.get(2)?,
        absolute_path: row.get(3)?,
        relative_path: row.get(4)?,
        preview_path: row.get(5)?,
        file_size: row.get(6)?,
        created_unix_ms: row.get(7)?,
        modified_unix_ms: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        preview_status: row.get(11)?,
        preview_issue_code: row.get(12)?,
        preview_issue_message: row.get(13)?,
        metadata_engine_id: row.get(14)?,
        metadata_engine_version: row.get(15)?,
        capture_local_time: row.get(16)?,
        capture_offset_minutes: row.get(17)?,
        capture_time_source: row.get(18)?,
        capture_raw_value: row.get(19)?,
        file_identity_scheme: row.get(20)?,
        file_identity_value: row.get(21)?,
    })
}

fn stored_asset_view(stored: StoredAssetRow) -> Result<AssetLocationView, ScanError> {
    let capture_time = stored_capture_time(
        stored.capture_local_time,
        stored.capture_offset_minutes,
        stored.capture_time_source,
        stored.capture_raw_value,
    )?;
    Ok(AssetLocationView {
        asset_id: stored.asset_id,
        location_id: stored.location_id,
        root_id: stored.root_id,
        display_path: user_visible_path(&stored.absolute_path),
        absolute_path: stored.absolute_path,
        relative_path: stored.relative_path,
        preview_path: stored.preview_path,
        file_size: sqlite_unsigned(stored.file_size, "file size")?,
        created_unix_ms: stored.created_unix_ms,
        modified_unix_ms: stored.modified_unix_ms,
        file_identity: stored_file_identity(
            stored.file_identity_scheme,
            stored.file_identity_value,
        )?,
        width: sqlite_u32(stored.width, "image width")?,
        height: sqlite_u32(stored.height, "image height")?,
        preview_status: parse_preview_status(&stored.preview_status)?,
        preview_issue_code: stored.preview_issue_code,
        preview_issue_message: stored.preview_issue_message,
        metadata_engine_id: stored.metadata_engine_id,
        metadata_engine_version: stored.metadata_engine_version,
        capture_time,
    })
}

fn stored_capture_time(
    local_time: Option<String>,
    offset_minutes: Option<i64>,
    source: Option<String>,
    raw_value: Option<String>,
) -> Result<Option<CaptureTimeEvidence>, ScanError> {
    match (local_time, source, raw_value) {
        (None, None, None) if offset_minutes.is_none() => Ok(None),
        (Some(local_time), Some(source), Some(raw_value)) => Ok(Some(CaptureTimeEvidence {
            local_time,
            offset_minutes: offset_minutes
                .map(|value| sqlite_i16(value, "capture time offset"))
                .transpose()?,
            source: parse_capture_time_source(&source)?,
            raw_value,
        })),
        _ => Err(ScanError::new(
            "catalog_capture_time_incomplete",
            "The stored capture-time evidence is incomplete",
        )),
    }
}

fn stored_file_identity(
    scheme: Option<String>,
    value: Option<String>,
) -> Result<Option<FileIdentityEvidence>, ScanError> {
    match (scheme, value) {
        (None, None) => Ok(None),
        (Some(scheme), Some(value)) => Ok(Some(FileIdentityEvidence { scheme, value })),
        _ => Err(ScanError::new(
            "catalog_file_identity_incomplete",
            "The stored file-identity evidence is incomplete",
        )),
    }
}

fn preview_status_text(status: &PreviewStatus) -> &'static str {
    match status {
        PreviewStatus::Pending => "pending",
        PreviewStatus::Ready => "ready",
        PreviewStatus::Failed => "failed",
    }
}

fn parse_preview_status(value: &str) -> Result<PreviewStatus, ScanError> {
    match value {
        "pending" => Ok(PreviewStatus::Pending),
        "ready" => Ok(PreviewStatus::Ready),
        "failed" => Ok(PreviewStatus::Failed),
        _ => Err(ScanError::new(
            "catalog_preview_status_invalid",
            format!("Unknown preview status {value}"),
        )),
    }
}

fn capture_time_source_text(source: &CaptureTimeSource) -> &'static str {
    match source {
        CaptureTimeSource::Original => "exif_original",
        CaptureTimeSource::Digitized => "exif_digitized",
        CaptureTimeSource::Image => "exif_datetime",
    }
}

fn parse_capture_time_source(value: &str) -> Result<CaptureTimeSource, ScanError> {
    match value {
        "exif_original" => Ok(CaptureTimeSource::Original),
        "exif_digitized" => Ok(CaptureTimeSource::Digitized),
        "exif_datetime" => Ok(CaptureTimeSource::Image),
        _ => Err(ScanError::new(
            "catalog_capture_time_source_invalid",
            format!("Unknown capture-time source {value}"),
        )),
    }
}

fn load_scan_with_status(
    connection: &Connection,
    status: &str,
) -> Result<Option<RecoverableScan>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT scans.id, roots.path, scans.max_items, scans.max_entries,
                    scans.preview_edge, scans.visited_entries, scans.accepted_items,
                    scans.issue_count
             FROM scan_runs AS scans
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.status = ?1
             ORDER BY scans.started_unix_ms DESC, scans.id DESC
             LIMIT 1",
            [status],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    stored
        .map(
            |(
                scan_id,
                root_path,
                max_items,
                max_entries,
                preview_edge,
                visited_entries,
                accepted_items,
                issue_count,
            )| {
                Ok(RecoverableScan {
                    scan_id,
                    display_root_path: user_visible_path(&root_path),
                    root_path,
                    max_items: optional_sqlite_u32(max_items, "item limit")?,
                    max_entries: optional_sqlite_u32(max_entries, "entry limit")?,
                    preview_edge: sqlite_u32(preview_edge, "preview edge")?,
                    visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
                    accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
                    issue_count: sqlite_unsigned(issue_count, "issue count")?,
                })
            },
        )
        .transpose()
}

fn database_error(error: rusqlite::Error) -> ScanError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        match failure.code {
            rusqlite::ErrorCode::DatabaseBusy => {
                return ScanError::new(
                    "catalog_database_busy",
                    format!("The catalog database remained busy after waiting: {error}"),
                );
            }
            rusqlite::ErrorCode::DatabaseLocked => {
                return ScanError::new(
                    "catalog_database_locked",
                    format!("The catalog database is locked: {error}"),
                );
            }
            _ => {}
        }
    }
    ScanError::new("catalog_database_error", error.to_string())
}

fn delete_orphan_assets(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM assets
             WHERE NOT EXISTS (
               SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
             )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn sqlite_integer(value: u64, field: &str) -> Result<i64, ScanError> {
    i64::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_overflow",
            format!("The {field} exceeds the SQLite integer range"),
        )
    })
}

fn sqlite_unsigned(value: i64, field: &str) -> Result<u64, ScanError> {
    u64::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn sqlite_u32(value: i64, field: &str) -> Result<u32, ScanError> {
    u32::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn sqlite_i16(value: i64, field: &str) -> Result<i16, ScanError> {
    i16::try_from(value).map_err(|_| {
        ScanError::new(
            "catalog_integer_invalid",
            format!("The stored {field} is outside the supported range"),
        )
    })
}

fn optional_sqlite_u32(value: Option<i64>, field: &str) -> Result<Option<u32>, ScanError> {
    value.map(|value| sqlite_u32(value, field)).transpose()
}

fn unix_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
