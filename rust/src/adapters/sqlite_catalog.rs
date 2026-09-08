use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use rusqlite::types::Value;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Row, Transaction, TransactionBehavior, params,
    params_from_iter,
};

use crate::domain::{
    AssetLocationView, CaptureTimeEvidence, CaptureTimeSource, CatalogCursor, CatalogSnapshot,
    ExpectedFileState, FileIdentityEvidence, GalleryLayoutDateGroup, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GallerySortKey, GalleryTimeAnchor,
    GalleryTimeBucket, GalleryTimeline, LibraryChangeLane, LibraryChangeQueuePolicy,
    LibraryFolderCursor, LibraryFolderPage, LibraryRootAvailability, LibraryRootGeneration,
    LibraryRootView, PreviewArtifact, PreviewReclamationCandidate, PreviewRequest, PreviewStatus,
    RecoverableScan, ScanCheckpoint, ScanError, ScanIssue, ScanRequest, SourceRevisionEvidence,
};
use crate::ports::CatalogRepository;

use super::{
    file_identity_evidence, open_catalog_identity_guard, revalidate_file_state, user_visible_path,
};

mod folders;
mod gallery;
mod metadata_inventory;
mod migrations;
mod operation_diagnostics;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "R2c-P admits durable journal persistence before R2c-Q schedules production replay"
    )
)]
mod persistent_journal;
#[cfg(all(test, windows))]
mod poll_retirement_diagnostics;
mod preview_recovery;
mod read_retry;
mod reclamation;
mod retained_scan;
mod reusable_connection;
mod scan_lifecycle;
mod scan_publication;
mod scan_resumption;
#[cfg(test)]
mod session_open_diagnostics;
mod spool_retirement;
mod write_admission;

#[cfg(test)]
use write_admission::{SQLITE_MAINTENANCE_PRIORITY, SQLITE_USER_INTERACTIVE_PRIORITY};
use write_admission::{
    SqliteWriteAdmission, SqliteWritePermit, SqliteWritePreemptCallback, sqlite_write_priority,
};

use scan_lifecycle::abandon_scan_transaction;

use change_queue::{activate_root_change_queue, retire_root_change_queue};
use gallery::{
    GalleryAssetAnchor, build_gallery_asset_query, build_gallery_count_query,
    build_gallery_layout_manifest_query, build_gallery_timeline_query, gallery_cursor_for_asset,
    resolve_gallery_anchor_cursor, resolve_gallery_asset_anchor, resolve_gallery_location_anchor,
    validate_gallery_query,
};
pub(crate) use metadata_inventory::MetadataInventorySpoolExecution;
use migrations::{migrate_schema, prepare_fresh_catalog_auto_vacuum};
pub(crate) use read_retry::SqliteCatalogReadExecutor;
pub(crate) use reclamation::SqliteCatalogSpaceMaintenance;
pub(crate) use scan_publication::{
    RejectedInputValidationRoster, StagedValidationOutcome, StagedValidationRoster,
    ValidatedStagingProof, load_retained_scan_issue_window, retained_scan_has_unclassified_issues,
};

mod catalog_delta;
#[cfg(test)]
mod catch_up;
mod change_queue;

#[cfg(test)]
pub(crate) use catalog_delta::set_before_catalog_delta_commit_hook;
#[cfg(test)]
pub(crate) use metadata_inventory::set_before_metadata_inventory_spool_commit_hook;
#[cfg(test)]
pub(crate) use scan_publication::set_before_scan_projection_replacement_hook;
const SCHEMA_VERSION: i64 = 32;
const SQLITE_APPLICATION_ID: i64 = 0x414D_4531;
const SCAN_QUEUE_LEASE_MILLIS: i64 = 15 * 60 * 1_000;
const MAX_SCAN_CATCH_UP_LINEAGE: i64 = 4_096;
const LOCATION_STAGE_BATCH: usize = 128;
const MAX_LAYOUT_MANIFEST_CHUNK_ITEMS: u32 = 4_096;
const MAX_CATALOG_PAGE_ITEMS: u32 = 4_096;
const LAYOUT_FLAG_DIMENSIONS_KNOWN: u8 = 1;
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

static SQLITE_WRITE_ADMISSIONS: OnceLock<Mutex<HashMap<PathBuf, Weak<SqliteWriteAdmission>>>> =
    OnceLock::new();
static SQLITE_SCHEMA_INITIALIZERS: OnceLock<Mutex<HashMap<PathBuf, Weak<Mutex<()>>>>> =
    OnceLock::new();

#[cfg(test)]
static SQLITE_FULL_SCHEMA_VALIDATION_COUNTS: OnceLock<Mutex<HashMap<PathBuf, usize>>> =
    OnceLock::new();

struct PriorityTransaction<'connection> {
    transaction: Option<Transaction<'connection>>,
    _permit: SqliteWritePermit,
}

impl PriorityTransaction<'_> {
    fn commit(mut self) -> rusqlite::Result<()> {
        self.transaction
            .take()
            .expect("priority transaction is present until commit")
            .commit()
    }
}

impl<'connection> Deref for PriorityTransaction<'connection> {
    type Target = Transaction<'connection>;

    fn deref(&self) -> &Self::Target {
        self.transaction
            .as_ref()
            .expect("priority transaction is present while borrowed")
    }
}

impl DerefMut for PriorityTransaction<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.transaction
            .as_mut()
            .expect("priority transaction is present while mutably borrowed")
    }
}

fn sqlite_write_admission(path: &Path) -> Arc<SqliteWriteAdmission> {
    let registry = SQLITE_WRITE_ADMISSIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut admissions = registry.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(admission) = admissions.get(path).and_then(Weak::upgrade) {
        return admission;
    }
    admissions.retain(|_, admission| admission.strong_count() > 0);
    let admission = Arc::new(SqliteWriteAdmission::new());
    admissions.insert(path.to_path_buf(), Arc::downgrade(&admission));
    admission
}

fn sqlite_schema_initializer(path: &Path) -> Arc<Mutex<()>> {
    let registry = SQLITE_SCHEMA_INITIALIZERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut initializers = registry.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(initializer) = initializers.get(path).and_then(Weak::upgrade) {
        return initializer;
    }
    initializers.retain(|_, initializer| initializer.strong_count() > 0);
    let initializer = Arc::new(Mutex::new(()));
    initializers.insert(path.to_path_buf(), Arc::downgrade(&initializer));
    initializer
}

#[cfg(test)]
pub(crate) fn reset_full_schema_validation_count(path: &Path) {
    SQLITE_FULL_SCHEMA_VALIDATION_COUNTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(catalog_admission_path(path), 0);
}

#[cfg(test)]
pub(crate) fn full_schema_validation_count(path: &Path) -> usize {
    SQLITE_FULL_SCHEMA_VALIDATION_COUNTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&catalog_admission_path(path))
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn remove_persistent_journal_v22_contract_for_test(connection: &Connection) {
    migrations::downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER IF EXISTS library_live_gap_recovery_claim_insert_guard;
             DROP INDEX IF EXISTS library_live_gap_recovery_claims_root;
             DROP TABLE IF EXISTS library_live_gap_recovery_claims;
             DROP TABLE IF EXISTS library_live_gap_recovery_contract;
             DROP INDEX IF EXISTS library_scan_publication_namespace_root;
             DROP TABLE IF EXISTS library_scan_publication_namespace_bindings;
             DROP TABLE IF EXISTS library_root_publication_namespaces;
             DROP TABLE IF EXISTS library_root_publication_namespace_contract;
             DROP TRIGGER IF EXISTS library_metadata_inventory_spool_directory_complete_guard;
             DROP TRIGGER IF EXISTS library_metadata_inventory_spool_binding_update_guard;
             DROP INDEX IF EXISTS library_metadata_inventory_spool_entries_order;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_entries;
             DROP INDEX IF EXISTS library_metadata_inventory_spool_directories_state;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_directories;
             DROP TABLE IF EXISTS library_metadata_inventory_spools;
             DROP TABLE IF EXISTS library_metadata_inventory_spool_contract;
             DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_update_guard;
             DROP TRIGGER IF EXISTS library_metadata_inventory_candidate_owner_insert_guard;
             DROP INDEX IF EXISTS library_metadata_inventory_candidate_owners_change;
             DROP TABLE IF EXISTS library_metadata_inventory_candidate_owners;
             DROP INDEX IF EXISTS library_metadata_inventory_frontier_state;
             DROP TABLE IF EXISTS library_metadata_inventory_frontier;
             DROP TABLE IF EXISTS library_recovery_execution_contract;
             DROP TRIGGER IF EXISTS library_persistent_journal_baseline_update_guard;
             DROP TRIGGER IF EXISTS library_persistent_journal_baseline_insert_guard;
             DROP INDEX IF EXISTS library_persistent_journal_baselines_root;
             DROP TABLE IF EXISTS library_persistent_journal_baselines;
             DROP TRIGGER IF EXISTS library_recovery_authority_update_guard;
             DROP TRIGGER IF EXISTS library_recovery_authority_insert_guard;
             DROP INDEX IF EXISTS library_recovery_authorities_root;
             DROP TABLE IF EXISTS library_recovery_authorities;
             DROP TABLE IF EXISTS library_recovery_authority_contract;
             DROP TRIGGER IF EXISTS library_change_queue_lane_origin_update;
             DROP TRIGGER IF EXISTS library_change_queue_lane_insert;
             DROP TRIGGER IF EXISTS library_change_queue_lane_update_guard;
             DROP TRIGGER IF EXISTS library_change_queue_lane_insert_guard;
             DROP INDEX IF EXISTS library_change_queue_lanes_eligible;
             DROP TABLE IF EXISTS library_change_queue_lanes;
             DROP TABLE IF EXISTS library_change_lane_contract;
             DROP TABLE library_persistent_journal_pending_renames;
             DROP TABLE library_persistent_journal_range_lifecycle;
             DROP TABLE library_persistent_journal_cross_root_ranges;
             DROP TABLE library_persistent_journal_cross_root_lineage;
             DROP TABLE library_persistent_journal_queue_lineage;
             DROP TABLE library_persistent_journal_source_ranges;
             DROP TABLE library_persistent_journal_checkpoints;
             DROP TABLE library_persistent_journal_root_state;
             DROP TABLE library_persistent_journal_contract;
             DROP INDEX library_change_root_state_generation_identity;",
        )
        .expect("remove v22 persistent journal contract from migration fixture");
}

pub struct SqliteCatalog {
    path: PathBuf,
    connection: Connection,
    _identity_guard: Option<SqliteCatalogIdentityGuard>,
    session: SqliteCatalogSession,
    write_admission: Arc<SqliteWriteAdmission>,
    pending_locations: Vec<PendingLocation>,
    pending_authoritative_retry_paths: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct SqliteCatalogSession {
    path: PathBuf,
    database_identity: SqliteDatabaseIdentity,
    application_id: i64,
    user_version: i64,
    schema_cookie: i64,
    write_admission: Arc<SqliteWriteAdmission>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SqliteDatabaseIdentity {
    canonical_path: PathBuf,
    file_identity: FileIdentityEvidence,
}

struct SqliteCatalogIdentityGuard {
    _file: fs::File,
    identity: SqliteDatabaseIdentity,
}

#[derive(Clone)]
struct PendingLocation {
    scan_id: String,
    root_id: String,
    location: AssetLocationView,
    identity_group_baseline: Option<StoredIdentityGroupState>,
}

#[derive(Clone)]
enum IdentityGenerationExpectation {
    Captured(Option<StoredIdentityGroupState>),
    MirrorCurrentActive,
    ResolveCurrent,
}

struct PersistLocationOutcome {
    active_projection_changed: bool,
    source_generation: i64,
}

struct IdentityGenerationBatchState {
    baseline_state: Option<StoredIdentityGroupState>,
    assigned_generation: i64,
    source_revision_token: Option<String>,
    file_size: i64,
    modified_unix_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredIdentityGroupState {
    generation: i64,
    source_revision_token: Option<String>,
    file_size: i64,
    modified_unix_ms: i64,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SqliteCatalogReadStage {
    Open,
    Validation,
    Query,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WatcherRecoveryObservation {
    pub(crate) authority_count: u64,
    pub(crate) active_inventory_run_count: u64,
    pub(crate) active_authority_change_id: Option<u64>,
}

impl SqliteCatalogSession {
    pub(crate) fn validate(path: PathBuf) -> Result<Self, ScanError> {
        prepare_catalog_directory(&path)?;
        let admission_path = catalog_admission_path(&path);
        let initializer = sqlite_schema_initializer(&admission_path);
        let _initialization = initializer
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut connection = Connection::open(&path).map_err(database_error)?;
        prepare_fresh_catalog_auto_vacuum(&connection)?;
        configure_catalog_connection(&connection)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            connection
                .execute_batch("PRAGMA journal_mode = WAL;")
                .map_err(database_error)?;
        }
        #[cfg(test)]
        {
            let mut counts = SQLITE_FULL_SCHEMA_VALIDATION_COUNTS
                .get_or_init(|| Mutex::new(HashMap::new()))
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let count = counts.entry(admission_path.clone()).or_default();
            *count = count.saturating_add(1);
        }
        migrate_schema(&mut connection)?;
        let application_id = catalog_pragma_integer(&connection, "application_id")?;
        let user_version = catalog_pragma_integer(&connection, "user_version")?;
        let schema_cookie = catalog_schema_cookie(&connection)?;
        let database_identity = open_catalog_database_identity_guard(&path)?.identity;
        Ok(Self {
            path,
            database_identity,
            application_id,
            user_version,
            schema_cookie,
            write_admission: sqlite_write_admission(&admission_path),
        })
    }

    pub(crate) fn open_in_lane(&self, lane: LibraryChangeLane) -> Result<SqliteCatalog, ScanError> {
        #[cfg(test)]
        {
            self.open_in_lane_with_diagnostics(lane)
        }
        #[cfg(not(test))]
        self.open_in_lane_with_read_stage(lane, |_| Ok(()))
    }

    #[cfg(test)]
    pub(super) fn validate_existing_for_read(
        path: PathBuf,
        mut before_stage: impl FnMut(SqliteCatalogReadStage) -> Result<(), ScanError>,
    ) -> Result<Self, ScanError> {
        let identity_guard = open_catalog_database_identity_guard(&path)?;
        before_stage(SqliteCatalogReadStage::Open)?;
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(database_error)?;
        configure_catalog_read_connection(&connection)?;
        before_stage(SqliteCatalogReadStage::Validation)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        let application_id = catalog_pragma_integer(&connection, "application_id")?;
        let user_version = catalog_pragma_integer(&connection, "user_version")?;
        if !journal_mode.eq_ignore_ascii_case("wal")
            || application_id != SQLITE_APPLICATION_ID
            || user_version != SCHEMA_VERSION
        {
            return Err(ScanError::new(
                "catalog_read_schema_requires_preparation",
                "The catalog schema must be prepared before it can serve application reads",
            ));
        }
        let version = connection
            .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(database_error)?;
        let schema_cookie = catalog_schema_cookie(&connection)?;
        if version != SCHEMA_VERSION {
            return Err(ScanError::new(
                "catalog_read_schema_requires_preparation",
                "The catalog schema must be prepared before it can serve application reads",
            ));
        }
        let admission_path = catalog_admission_path(&path);
        Ok(Self {
            path,
            database_identity: identity_guard.identity.clone(),
            application_id,
            user_version,
            schema_cookie,
            write_admission: sqlite_write_admission(&admission_path),
        })
    }

    pub(super) fn open_in_lane_with_read_stage(
        &self,
        _lane: LibraryChangeLane,
        mut before_stage: impl FnMut(SqliteCatalogReadStage) -> Result<(), ScanError>,
    ) -> Result<SqliteCatalog, ScanError> {
        let before_identity = catalog_database_identity(&self.path)?;
        if before_identity != self.database_identity {
            return Err(stale_catalog_session_error());
        }
        before_stage(SqliteCatalogReadStage::Open)?;
        let connection = Connection::open(&self.path).map_err(database_error)?;
        configure_catalog_connection(&connection)?;
        before_stage(SqliteCatalogReadStage::Validation)?;
        self.validate_connection_proof(&connection, &before_identity)?;
        Ok(SqliteCatalog {
            path: self.path.clone(),
            connection,
            _identity_guard: None,
            session: self.clone(),
            write_admission: Arc::clone(&self.write_admission),
            pending_locations: Vec::with_capacity(LOCATION_STAGE_BATCH),
            pending_authoritative_retry_paths: Vec::new(),
        })
    }

    pub(super) fn open_read_only_with_read_stage(
        &self,
        mut before_stage: impl FnMut(SqliteCatalogReadStage) -> Result<(), ScanError>,
    ) -> Result<SqliteCatalog, ScanError> {
        let identity_guard = open_catalog_database_identity_guard(&self.path)?;
        if identity_guard.identity != self.database_identity {
            return Err(stale_catalog_session_error());
        }
        before_stage(SqliteCatalogReadStage::Open)?;
        let connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(database_error)?;
        configure_catalog_read_connection(&connection)?;
        before_stage(SqliteCatalogReadStage::Validation)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .map_err(database_error)?;
        let version = connection
            .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(database_error)?;
        let application_id = catalog_pragma_integer(&connection, "application_id")?;
        let user_version = catalog_pragma_integer(&connection, "user_version")?;
        let schema_cookie = catalog_schema_cookie(&connection)?;
        if !journal_mode.eq_ignore_ascii_case("wal")
            || version != SCHEMA_VERSION
            || application_id != self.application_id
            || application_id != SQLITE_APPLICATION_ID
            || user_version != self.user_version
            || user_version != SCHEMA_VERSION
            || schema_cookie != self.schema_cookie
        {
            return Err(stale_catalog_session_error());
        }
        Ok(SqliteCatalog {
            path: self.path.clone(),
            connection,
            _identity_guard: Some(identity_guard),
            session: self.clone(),
            write_admission: Arc::clone(&self.write_admission),
            pending_locations: Vec::with_capacity(LOCATION_STAGE_BATCH),
            pending_authoritative_retry_paths: Vec::new(),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

fn prepare_catalog_directory(path: &Path) -> Result<(), ScanError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            ScanError::new(
                "catalog_directory_unavailable",
                format!("Could not create the catalog directory: {error}"),
            )
        })?;
    }
    Ok(())
}

fn catalog_admission_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .and_then(|parent| path.file_name().map(|file_name| parent.join(file_name)))
            .unwrap_or_else(|| path.to_path_buf())
    })
}

fn configure_catalog_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(database_error)
}

fn configure_catalog_read_connection(connection: &Connection) -> Result<(), ScanError> {
    connection
        .busy_timeout(read_retry::PROTOCOL_READ_DEADLINE)
        .map_err(database_error)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = ON;")
        .map_err(database_error)
}

fn catalog_schema_cookie(connection: &Connection) -> Result<i64, ScanError> {
    catalog_pragma_integer(connection, "schema_version")
}

fn catalog_pragma_integer(connection: &Connection, name: &str) -> Result<i64, ScanError> {
    connection
        .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .map_err(database_error)
}

fn catalog_database_identity(path: &Path) -> Result<SqliteDatabaseIdentity, ScanError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| {
        ScanError::new(
            "catalog_identity_unavailable",
            format!("Could not canonicalize the catalog identity: {error}"),
        )
    })?;
    let file_identity = file_identity_evidence(path)
        .map_err(|error| {
            ScanError::new(
                "catalog_identity_unavailable",
                format!("Could not inspect the catalog identity: {error}"),
            )
        })?
        .ok_or_else(|| {
            ScanError::new(
                "catalog_identity_unavailable",
                "The catalog file identity is unavailable",
            )
        })?;
    Ok(SqliteDatabaseIdentity {
        canonical_path,
        file_identity,
    })
}

fn open_catalog_database_identity_guard(
    path: &Path,
) -> Result<SqliteCatalogIdentityGuard, ScanError> {
    let (file, file_identity) = open_catalog_identity_guard(path).map_err(|error| {
        ScanError::new(
            "catalog_identity_unavailable",
            format!("Could not hold the catalog identity: {error}"),
        )
    })?;
    let canonical_path = fs::canonicalize(path).map_err(|error| {
        ScanError::new(
            "catalog_identity_unavailable",
            format!("Could not canonicalize the held catalog identity: {error}"),
        )
    })?;
    let file_identity = file_identity.ok_or_else(|| {
        ScanError::new(
            "catalog_identity_unsupported",
            "The current platform cannot provide stable catalog file identity evidence",
        )
    })?;
    Ok(SqliteCatalogIdentityGuard {
        _file: file,
        identity: SqliteDatabaseIdentity {
            canonical_path,
            file_identity,
        },
    })
}

fn stale_catalog_session_error() -> ScanError {
    ScanError::new(
        "catalog_validated_session_stale",
        "The catalog identity or schema changed after runtime validation",
    )
}

impl SqliteCatalog {
    #[cfg(test)]
    pub fn open(path: PathBuf) -> Result<Self, ScanError> {
        Self::open_in_lane(path, LibraryChangeLane::Recovery)
    }

    #[cfg(test)]
    pub(crate) fn open_in_lane(path: PathBuf, lane: LibraryChangeLane) -> Result<Self, ScanError> {
        SqliteCatalogSession::validate(path)?.open_in_lane(lane)
    }

    pub(crate) fn completed_write_epoch(&self) -> u64 {
        self.write_admission.completed_write_epoch()
    }

    pub(crate) fn wait_for_completed_write_after(
        &self,
        observed_epoch: u64,
        timeout: Duration,
    ) -> u64 {
        self.write_admission
            .wait_for_completed_write_after(observed_epoch, timeout)
    }

    #[cfg(test)]
    pub(crate) fn prove_live_only_first_import_handoff_for_test(
        &mut self,
        scan_id: &str,
    ) -> Result<(), ScanError> {
        let (root_id, stored_generation, started_unix_ms) = self
            .connection
            .query_row(
                "SELECT scan.root_id, scan.root_generation_at_start, scan.started_unix_ms
                 FROM scan_runs AS scan
                 JOIN library_roots AS root ON root.id = scan.root_id
                 WHERE scan.id = ?1 AND scan.status = 'running'
                   AND scan.scan_owner = 'foreground'
                   AND root.active_scan_id IS NULL",
                [scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_test_first_import_handoff_invalid",
                    "The test handoff requires a running foreground first-import scan",
                )
            })?;
        let root_generation = LibraryRootGeneration::new(sqlite_unsigned(
            stored_generation,
            "first-import test root generation",
        )?)
        .ok_or_else(|| {
            ScanError::new(
                "catalog_test_first_import_handoff_invalid",
                "The test handoff root generation is invalid",
            )
        })?;
        <Self as crate::ports::PersistentJournalRepository>::save_persistent_journal_capability(
            self,
            &crate::domain::PersistentJournalCapability {
                root_id,
                root_generation,
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                contract_version: crate::domain::PERSISTENT_JOURNAL_CONTRACT_VERSION,
                state: crate::domain::PersistentJournalCapabilityState::LiveOnly,
                continuity: crate::domain::PersistentJournalContinuityState::LiveOnly,
                failure: None,
                updated_unix_ms: started_unix_ms,
            },
        )
    }

    pub(crate) fn validated_session(&self) -> SqliteCatalogSession {
        self.session.clone()
    }

    #[cfg(test)]
    pub(crate) fn load_watcher_recovery_observation_for_test(
        &self,
        root_id: &str,
    ) -> Result<WatcherRecoveryObservation, ScanError> {
        let (authority_count, active_inventory_run_count, change_id, active_authority_count) = self
            .connection
            .query_row(
                "SELECT
                   (SELECT COUNT(*) FROM library_recovery_authorities
                    WHERE root_id = ?1 AND reason = 'watcher_uncovered_gap'),
                   (SELECT COUNT(*) FROM library_metadata_inventory_runs
                    WHERE root_id = ?1 AND status IN ('running', 'comparing')),
                   COALESCE(MIN(authority.change_id), 0),
                   COUNT(authority.change_id)
                 FROM library_recovery_authorities AS authority
                 JOIN library_change_queue_lanes AS lane ON lane.change_id = authority.change_id
                 JOIN library_metadata_inventory_runs AS run ON run.id = authority.run_id
                 WHERE authority.root_id = ?1
                   AND authority.reason = 'watcher_uncovered_gap'
                   AND authority.retired_unix_ms IS NULL
                   AND lane.lane = 'p2_recovery'
                   AND run.status IN ('running', 'comparing')",
                [root_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let active_authority_count =
            sqlite_unsigned(active_authority_count, "active watcher-gap authority count")?;
        if active_authority_count > 1 {
            return Err(ScanError::new(
                "catalog_watcher_gap_authority_ambiguous",
                "More than one active watcher-gap recovery authority exists for a library root",
            ));
        }
        Ok(WatcherRecoveryObservation {
            authority_count: sqlite_unsigned(
                authority_count,
                "watcher-gap recovery authority count",
            )?,
            active_inventory_run_count: sqlite_unsigned(
                active_inventory_run_count,
                "active metadata inventory run count",
            )?,
            active_authority_change_id: (active_authority_count == 1)
                .then(|| {
                    sqlite_unsigned(change_id, "active watcher-gap authority change identifier")
                })
                .transpose()?,
        })
    }

    fn begin_write(&mut self) -> Result<PriorityTransaction<'_>, ScanError> {
        self.begin_write_in_lane(LibraryChangeLane::Recovery)
    }

    fn begin_write_in_lane(
        &mut self,
        lane: LibraryChangeLane,
    ) -> Result<PriorityTransaction<'_>, ScanError> {
        let permit = self.write_admission.acquire(lane);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        Ok(PriorityTransaction {
            transaction: Some(transaction),
            _permit: permit,
        })
    }

    fn begin_preemptible_write_in_lane(
        &mut self,
        lane: LibraryChangeLane,
        preempt: SqliteWritePreemptCallback,
    ) -> Result<PriorityTransaction<'_>, ScanError> {
        let permit = self.write_admission.acquire_preemptible(lane, preempt);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        Ok(PriorityTransaction {
            transaction: Some(transaction),
            _permit: permit,
        })
    }

    fn begin_user_interactive_write(&mut self) -> Result<PriorityTransaction<'_>, ScanError> {
        let permit = self
            .write_admission
            .acquire_user_interactive()
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_user_interactive_write_timeout",
                    "The catalog did not release the current writer in time; retry the operation",
                )
            })?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        Ok(PriorityTransaction {
            transaction: Some(transaction),
            _permit: permit,
        })
    }

    fn flush_pending_locations(&mut self) -> Result<(), ScanError> {
        if self.pending_locations.is_empty() {
            return Ok(());
        }
        let pending = self.pending_locations.clone();
        let transaction = self.begin_write()?;
        let mut active_projection_changed = false;
        let mut identity_generation_batch = HashMap::new();
        for item in &pending {
            active_projection_changed |= persist_location(
                &transaction,
                &item.scan_id,
                &item.root_id,
                &item.location,
                IdentityGenerationExpectation::Captured(item.identity_group_baseline.clone()),
                &mut identity_generation_batch,
            )?
            .active_projection_changed;
        }
        if active_projection_changed {
            let updated = transaction
                .execute("UPDATE catalog_state SET revision = revision + 1", [])
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_revision_unavailable",
                    "The catalog revision state is missing or invalid",
                ));
            }
        }
        transaction.commit().map_err(database_error)?;
        self.pending_locations.clear();
        Ok(())
    }

    pub(crate) fn publish_authoritative_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
        retry_relative_paths: &[String],
    ) -> Result<(), ScanError> {
        if !self.pending_authoritative_retry_paths.is_empty() {
            return Err(ScanError::new(
                "catalog_authoritative_retry_paths_pending",
                "Another authoritative publication still owns pending retry paths",
            ));
        }
        self.pending_authoritative_retry_paths
            .extend_from_slice(retry_relative_paths);
        let result = <Self as CatalogRepository>::publish_scan(
            self,
            scan_id,
            root_id,
            asset_count,
            issue_count,
        );
        self.pending_authoritative_retry_paths.clear();
        result
    }

    pub(crate) fn pristine_first_import_scan(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<Option<(String, i64)>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT scan.id, scan.started_unix_ms
                 FROM scan_runs AS scan
                 JOIN library_roots AS root ON root.id = scan.root_id
                 JOIN library_change_root_state AS active
                   ON active.root_id = scan.root_id
                  AND active.generation = scan.root_generation_at_start
                 WHERE scan.root_id = ?1 AND scan.root_generation_at_start = ?2
                   AND scan.scan_owner = 'foreground' AND scan.status = 'running'
                   AND scan.visited_entries = 0 AND scan.accepted_items = 0
                   AND root.active_scan_id IS NULL AND active.is_active = 1
                 ORDER BY scan.started_unix_ms, scan.id
                 LIMIT 2",
            )
            .map_err(database_error)?;
        let scan_ids = statement
            .query_map(
                params![
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        match scan_ids.as_slice() {
            [] => Ok(None),
            [scan] => Ok(Some(scan.clone())),
            _ => Err(ScanError::new(
                "catalog_first_import_scan_ambiguous",
                "The root has more than one pristine first-import scan",
            )),
        }
    }

    pub(crate) fn first_import_change_capture_is_ready(
        &self,
        scan_id: &str,
        root_id: &str,
        root_generation: LibraryRootGeneration,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM library_recovery_authorities AS authority
                   JOIN library_persistent_journal_baselines AS baseline
                     ON baseline.change_id = authority.change_id
                   WHERE authority.run_id = ?1 AND authority.root_id = ?2
                     AND authority.root_generation = ?3
                     AND authority.reason = 'first_import_boundary'
                     AND authority.retired_unix_ms IS NULL
                     AND baseline.root_id = authority.root_id
                     AND baseline.root_generation = authority.root_generation
                     AND baseline.phase = 'inventory'
                 ) OR EXISTS(
                   SELECT 1
                   FROM library_persistent_journal_root_state AS journal
                   JOIN library_change_root_state AS active
                     ON active.root_id = journal.root_id
                    AND active.generation = journal.root_generation
                   JOIN scan_runs AS scan
                     ON scan.id = ?1
                    AND scan.root_id = journal.root_id
                    AND scan.root_generation_at_start = journal.root_generation
                   WHERE journal.root_id = ?2 AND journal.root_generation = ?3
                     AND journal.capability_state = 'live_only'
                     AND journal.continuity_state = 'live_only'
                     AND journal.updated_unix_ms >= scan.started_unix_ms
                     AND active.is_active = 1
                     AND scan.scan_owner = 'foreground'
                     AND scan.status = 'running'
                 )",
                params![
                    scan_id,
                    root_id,
                    sqlite_integer(root_generation.value(), "root generation")?,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn preserve_authoritative_retry_evidence(
        &mut self,
        scan_id: &str,
        root_id: &str,
        retry_relative_paths: &[String],
    ) -> Result<u64, ScanError> {
        if root_id.trim().is_empty() || root_id.contains('\0') {
            return Err(ScanError::new(
                "catalog_root_id_invalid",
                "The library root ID must be non-empty and contain no NUL bytes",
            ));
        }
        let retry_path_limit = usize::try_from(LibraryChangeQueuePolicy::MAX_UNRESOLVED_CHANGES)
            .map_err(|_| {
                ScanError::new(
                    "catalog_authoritative_retry_limit_invalid",
                    "The authoritative retry path limit is outside the supported range",
                )
            })?;
        if retry_relative_paths.len() > retry_path_limit
            || retry_relative_paths.iter().any(|relative_path| {
                relative_path.is_empty()
                    || relative_path.contains('\0')
                    || relative_path.contains('\\')
            })
        {
            return Err(ScanError::new(
                "catalog_authoritative_retry_paths_invalid",
                "Authoritative retry evidence must contain bounded normalized relative paths",
            ));
        }
        self.flush_pending_locations()?;
        let transaction = self.begin_write()?;
        let previous_active_scan = transaction
            .query_row(
                "SELECT roots.active_scan_id
                 FROM scan_runs AS scans
                 JOIN library_roots AS roots ON roots.id = scans.root_id
                 WHERE scans.id = ?1 AND scans.root_id = ?2
                   AND scans.status = 'running'
                   AND scans.scan_owner = 'authoritative_recovery'",
                params![scan_id, root_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(database_error)?
            .flatten()
            .filter(|active_scan_id| active_scan_id != scan_id);
        if retry_relative_paths.is_empty() {
            let count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1",
                    [scan_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            transaction.commit().map_err(database_error)?;
            return sqlite_unsigned(count, "staged file state count");
        }
        if !transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE id = ?1 AND root_id = ?2 AND status = 'running'
                     AND scan_owner = 'authoritative_recovery'
                 )",
                params![scan_id, root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?
        {
            return Err(ScanError::new(
                "catalog_authoritative_retry_scan_invalid",
                "Retry evidence can only be preserved for a running authoritative scan",
            ));
        }
        for relative_path in retry_relative_paths {
            let staged_count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM asset_locations
                     WHERE scan_id = ?1 AND root_id = ?2 AND relative_path = ?3",
                    params![scan_id, root_id, relative_path],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if staged_count > 1 {
                return Err(ScanError::new(
                    "catalog_authoritative_retry_path_ambiguous",
                    "A retry path matched more than one staged catalog location",
                ));
            }
            transaction
                .execute(
                    "DELETE FROM asset_locations
                     WHERE scan_id = ?1 AND root_id = ?2 AND relative_path = ?3",
                    params![scan_id, root_id, relative_path],
                )
                .map_err(database_error)?;
            let Some(previous_active_scan) = previous_active_scan.as_deref() else {
                continue;
            };
            let prior_count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM asset_locations
                     WHERE scan_id = ?1 AND root_id = ?2 AND relative_path = ?3",
                    params![previous_active_scan, root_id, relative_path],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if prior_count > 1 {
                return Err(ScanError::new(
                    "catalog_authoritative_retry_prior_ambiguous",
                    "A retry path matched more than one previously published catalog location",
                ));
            }
            transaction
                .execute(
                    "INSERT INTO asset_locations(
                       scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
                       preview_path, file_size, created_unix_ms, modified_unix_ms,
                       file_local_time, parent_relative_path, natural_name_key, width, height,
                       preview_status, preview_issue_code, preview_issue_message,
                       metadata_engine_id, metadata_engine_version, capture_local_time,
                       capture_offset_minutes, capture_time_source, capture_raw_value,
                       file_identity_scheme, file_identity_value, source_revision_token,
                       source_generation
                     )
                     SELECT ?1, asset_id, location_id, root_id, absolute_path, relative_path,
                            preview_path, file_size, created_unix_ms, modified_unix_ms,
                            file_local_time, parent_relative_path, natural_name_key, width, height,
                            preview_status, preview_issue_code, preview_issue_message,
                            metadata_engine_id, metadata_engine_version, capture_local_time,
                            capture_offset_minutes, capture_time_source, capture_raw_value,
                            file_identity_scheme, file_identity_value, source_revision_token,
                            source_generation
                     FROM asset_locations
                     WHERE scan_id = ?2 AND root_id = ?3 AND relative_path = ?4",
                    params![scan_id, previous_active_scan, root_id, relative_path],
                )
                .map_err(database_error)?;
        }
        let count = transaction
            .query_row(
                "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1",
                [scan_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        sqlite_unsigned(count, "staged file state count")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanOwner {
    Foreground,
    AuthoritativeRecovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveGapRecoveryConsumer {
    ExplicitRecoveryRequired,
    ForegroundScan,
}

impl LiveGapRecoveryConsumer {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitRecoveryRequired => "explicit_recovery_required",
            Self::ForegroundScan => "foreground_scan",
        }
    }
}

fn bind_explicit_recovery_claims_to_foreground_scan(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    root_generation: i64,
    bound_unix_ms: i64,
) -> Result<(), ScanError> {
    let claim_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_live_gap_recovery_claims
             WHERE root_id = ?1 AND root_generation = ?2
               AND consumer_kind = ?3",
            params![
                root_id,
                root_generation,
                LiveGapRecoveryConsumer::ExplicitRecoveryRequired.as_str(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if claim_count == 0 {
        return Ok(());
    }
    let leased = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'leased', next_retry_unix_ms = NULL,
                 lease_generation = lease_generation + 1,
                 lease_expires_unix_ms = ?1, authoritative_scan_id = ?2,
                 last_failure_code = 'live_gap_v30_explicit_recovery_in_progress',
                 last_failure_message =
                   'The explicit recovery claim is owned by the foreground library update',
                 updated_unix_ms = ?3
             WHERE root_id = ?4 AND root_generation = ?5
               AND status = 'retry_wait' AND next_retry_unix_ms IS NULL
               AND last_failure_code = 'live_gap_v30_explicit_recovery_required'
               AND id IN (
                 SELECT gap_change_id FROM library_live_gap_recovery_claims
                 WHERE root_id = ?4 AND root_generation = ?5
                   AND consumer_kind = ?6
               )",
            params![
                bound_unix_ms.saturating_add(SCAN_QUEUE_LEASE_MILLIS),
                scan_id,
                bound_unix_ms,
                root_id,
                root_generation,
                LiveGapRecoveryConsumer::ExplicitRecoveryRequired.as_str(),
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(leased).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The explicit recovery claim changed before the foreground scan could own it",
        ));
    }
    let bound = transaction
        .execute(
            "UPDATE library_live_gap_recovery_claims
             SET consumer_kind = ?1, foreground_scan_id = ?2
             WHERE root_id = ?3 AND root_generation = ?4
               AND consumer_kind = ?5 AND foreground_scan_id IS NULL
               AND consumed_unix_ms IS NULL",
            params![
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
                root_id,
                root_generation,
                LiveGapRecoveryConsumer::ExplicitRecoveryRequired.as_str(),
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(bound).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The explicit recovery consumer changed before its scan linkage became durable",
        ));
    }
    Ok(())
}

fn consume_foreground_recovery_claims(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    root_generation: i64,
    published_revision: u64,
    consumed_unix_ms: i64,
) -> Result<(), ScanError> {
    let claim_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_live_gap_recovery_claims
             WHERE root_id = ?1 AND root_generation = ?2
               AND consumer_kind = ?3 AND foreground_scan_id = ?4
               AND consumed_unix_ms IS NULL",
            params![
                root_id,
                root_generation,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if claim_count == 0 {
        return Ok(());
    }
    let completed = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'completed', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                 catalog_revision_at_success = ?1,
                 last_failure_code = NULL, last_failure_message = NULL,
                 updated_unix_ms = ?2
             WHERE root_id = ?3 AND root_generation = ?4
               AND status = 'leased' AND authoritative_scan_id = ?5
               AND last_failure_code = 'live_gap_v30_explicit_recovery_in_progress'
               AND id IN (
                 SELECT gap_change_id FROM library_live_gap_recovery_claims
                 WHERE consumer_kind = ?6 AND foreground_scan_id = ?5
                   AND consumed_unix_ms IS NULL
               )",
            params![
                sqlite_integer(published_revision, "catalog revision")?,
                consumed_unix_ms,
                root_id,
                root_generation,
                scan_id,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(completed).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The foreground scan no longer owns every explicit recovery gap",
        ));
    }
    let consumed = transaction
        .execute(
            "UPDATE library_live_gap_recovery_claims
             SET consumed_unix_ms = ?1
             WHERE root_id = ?2 AND root_generation = ?3
               AND consumer_kind = ?4 AND foreground_scan_id = ?5
               AND consumed_unix_ms IS NULL",
            params![
                consumed_unix_ms,
                root_id,
                root_generation,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(consumed).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The foreground recovery claim changed before consumption was recorded",
        ));
    }
    Ok(())
}

fn restore_explicit_recovery_claims_from_foreground_scan(
    transaction: &Transaction<'_>,
    scan_id: &str,
    restored_unix_ms: i64,
) -> Result<(), ScanError> {
    let claim_count = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_live_gap_recovery_claims
             WHERE consumer_kind = ?1 AND foreground_scan_id = ?2
               AND consumed_unix_ms IS NULL",
            params![LiveGapRecoveryConsumer::ForegroundScan.as_str(), scan_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if claim_count == 0 {
        return Ok(());
    }
    let restored_gaps = transaction
        .execute(
            "UPDATE library_change_queue
             SET status = 'retry_wait', next_retry_unix_ms = NULL,
                 lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                 last_failure_code = 'live_gap_v30_explicit_recovery_required',
                 last_failure_message =
                   'The explicit library update did not finish and must be started again',
                 updated_unix_ms = ?1
             WHERE status = 'leased' AND authoritative_scan_id = ?2
               AND id IN (
                 SELECT gap_change_id FROM library_live_gap_recovery_claims
                 WHERE consumer_kind = ?3 AND foreground_scan_id = ?2
                   AND consumed_unix_ms IS NULL
               )",
            params![
                restored_unix_ms,
                scan_id,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(restored_gaps).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The foreground scan no longer owns every explicit recovery gap",
        ));
    }
    let restored_claims = transaction
        .execute(
            "UPDATE library_live_gap_recovery_claims
             SET consumer_kind = ?1, foreground_scan_id = NULL,
                 consumed_unix_ms = NULL
             WHERE consumer_kind = ?2 AND foreground_scan_id = ?3
               AND consumed_unix_ms IS NULL",
            params![
                LiveGapRecoveryConsumer::ExplicitRecoveryRequired.as_str(),
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
            ],
        )
        .map_err(database_error)?;
    if i64::try_from(restored_claims).ok() != Some(claim_count) {
        return Err(ScanError::new(
            "catalog_live_gap_foreground_claim_conflict",
            "The foreground recovery consumer changed before it could be restored",
        ));
    }
    Ok(())
}

impl ScanOwner {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Foreground => "foreground",
            Self::AuthoritativeRecovery => "authoritative_recovery",
        }
    }
}

impl SqliteCatalog {
    pub(crate) fn begin_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        root_identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(
            request,
            root_id,
            root_path,
            ScanOwner::Foreground,
            Some(root_identity),
        )
    }

    pub(crate) fn resume_scan_with_publication_namespace(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        root_identity: &FileIdentityEvidence,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.resume_scan_owned(
            request,
            root_id,
            root_path,
            ScanOwner::Foreground,
            Some(root_identity),
        )
    }

    pub(crate) fn fail_scan_publication_namespace(
        &mut self,
        scan_id: &str,
        root_id: &str,
        failure: &ScanError,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        let now = unix_time_ms();
        let transaction = self.begin_write()?;
        let root_generation = transaction
            .query_row(
                "SELECT root_generation FROM library_scan_publication_namespace_bindings
                 WHERE scan_id = ?1 AND root_id = ?2",
                params![scan_id, root_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(database_error)?;
        if let Some(root_generation) = root_generation {
            let root_generation =
                LibraryRootGeneration::new(sqlite_unsigned(root_generation, "root generation")?)
                    .ok_or_else(|| {
                        ScanError::new(
                            "catalog_scan_generation_invalid",
                            "The failed publication guard captured an invalid root generation",
                        )
                    })?;
            persistent_journal::mark_persistent_journal_recovery_required(
                &transaction,
                root_id,
                root_generation,
                &failure.code,
                &failure.message,
                now,
            )?;
        }
        transaction.commit().map_err(database_error)?;
        self.abandon_scan(scan_id, "failed", issue_count)
    }

    pub(crate) fn load_single_recoverable_foreground_scan(
        &mut self,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        self.converge_single_recoverable_foreground_scan()?;
        load_scan_with_status(&self.connection, "running", ScanOwner::Foreground)
    }

    pub(crate) fn load_single_paused_foreground_scan(
        &mut self,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        self.converge_single_recoverable_foreground_scan()?;
        load_scan_with_status(&self.connection, "paused", ScanOwner::Foreground)
    }

    fn converge_single_recoverable_foreground_scan(&mut self) -> Result<(), ScanError> {
        let retired_scan_ids = {
            let completed_unix_ms = unix_time_ms();
            let transaction = self.begin_write()?;
            let unfinished_scans = {
                let mut statement = transaction
                    .prepare(
                        "SELECT scans.id, scans.issue_count,
                                roots.active_scan_id IS NOT NULL AS has_published_root
                         FROM scan_runs AS scans
                         JOIN library_roots AS roots ON roots.id = scans.root_id
                         WHERE scans.status IN ('running', 'paused')
                           AND scans.scan_owner = 'foreground'
                         ORDER BY CASE scans.status WHEN 'paused' THEN 0 ELSE 1 END,
                                  scans.started_unix_ms DESC, scans.id DESC",
                    )
                    .map_err(database_error)?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, bool>(2)?,
                        ))
                    })
                    .map_err(database_error)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(database_error)?
            };
            let mut retired_scan_ids = Vec::new();
            let mut retained_initial_import = false;
            for (scan_id, issue_count, has_published_root) in unfinished_scans {
                if !has_published_root && !retained_initial_import {
                    retained_initial_import = true;
                    continue;
                }
                abandon_scan_transaction(
                    &transaction,
                    &scan_id,
                    "failed",
                    issue_count,
                    completed_unix_ms,
                )?;
                retired_scan_ids.push(scan_id);
            }
            if !retired_scan_ids.is_empty() {
                delete_orphan_assets(&transaction)?;
            }
            transaction.commit().map_err(database_error)?;
            retired_scan_ids
        };
        if !retired_scan_ids.is_empty() {
            self.pending_locations.retain(|pending| {
                !retired_scan_ids
                    .iter()
                    .any(|scan_id| scan_id == &pending.scan_id)
            });
        }
        Ok(())
    }

    fn begin_scan_owned(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        owner: ScanOwner,
        publication_identity: Option<&FileIdentityEvidence>,
    ) -> Result<ScanCheckpoint, ScanError> {
        if let Some(identity) = publication_identity {
            validate_root_publication_identity(identity)?;
        }
        let now = unix_time_ms();
        let transaction = self.begin_write()?;
        let scan_exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM scan_runs WHERE id = ?1)",
                [&request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if scan_exists {
            return Err(ScanError::new(
                "catalog_scan_already_exists",
                "A new scan cannot reuse an existing scan identifier",
            ));
        }
        transaction
            .execute(
                "INSERT INTO library_roots(id, path, created_unix_ms)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET path = excluded.path",
                params![root_id, root_path, now],
            )
            .map_err(database_error)?;
        activate_root_change_queue(&transaction, root_id, now)?;
        let has_conflicting_scan = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused') AND id <> ?2
                 )",
                params![root_id, request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if has_conflicting_scan {
            return Err(ScanError::new(
                "catalog_root_scan_in_progress",
                "Another authoritative scan already owns this library root",
            ));
        }
        if let Some(identity) = publication_identity {
            let established = transaction
                .query_row(
                    "SELECT identity_scheme, identity_value
                     FROM library_root_publication_namespaces WHERE root_id = ?1",
                    [root_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(database_error)?;
            if established.as_ref().is_some_and(|(scheme, value)| {
                scheme != &identity.scheme || value != &identity.value
            }) {
                retire_root_change_queue(&transaction, root_id, now)?;
                activate_root_change_queue(&transaction, root_id, now)?;
            }
        }
        let (root_generation_at_start, change_queue_high_watermark) = transaction
            .query_row(
                "SELECT state.generation,
                        MAX(CASE WHEN queue.status IN ('pending', 'leased', 'retry_wait')
                          AND lanes.lane <> 'p0_live' THEN queue.id END)
                 FROM library_change_root_state AS state
                 LEFT JOIN library_change_queue AS queue
                   ON queue.root_id = state.root_id
                  AND queue.root_generation = state.generation
                 LEFT JOIN library_change_queue_lanes AS lanes
                   ON lanes.change_id = queue.id
                  AND NOT EXISTS(
                    SELECT 1 FROM library_live_gap_recovery_claims AS claim
                    WHERE claim.gap_change_id = queue.id
                  )
                  WHERE state.root_id = ?1 AND state.is_active = 1
                 GROUP BY state.generation",
                [root_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO scan_runs(
                   id, root_id, status, started_unix_ms, max_items, max_entries, preview_edge,
                   root_generation_at_start, change_queue_high_watermark, scan_owner
                 ) VALUES (?1, ?2, 'running', ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    request.scan_id,
                    root_id,
                    now,
                    request.max_items.map(i64::from),
                    request.max_entries.map(i64::from),
                    i64::from(request.preview_edge),
                    root_generation_at_start,
                    change_queue_high_watermark,
                    owner.as_str(),
                ],
            )
            .map_err(database_error)?;
        if let Some(identity) = publication_identity {
            transaction
                .execute(
                    "INSERT INTO library_scan_publication_namespace_bindings(
                       scan_id, root_id, root_generation, identity_scheme,
                       identity_value, bound_unix_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        identity.scheme,
                        identity.value,
                        now,
                    ],
                )
                .map_err(database_error)?;
        }
        if owner == ScanOwner::Foreground {
            bind_explicit_recovery_claims_to_foreground_scan(
                &transaction,
                &request.scan_id,
                root_id,
                root_generation_at_start,
                now,
            )?;
        }
        if let Some(high_watermark) = change_queue_high_watermark {
            transaction
                .execute(
                    "UPDATE library_change_queue
                     SET status = 'leased', next_retry_unix_ms = NULL,
                         lease_generation = lease_generation + 1,
                         lease_expires_unix_ms = ?1, updated_unix_ms = ?2,
                         authoritative_scan_id = ?3
                      WHERE root_id = ?4 AND root_generation = ?5 AND id <= ?6
                        AND status IN ('pending', 'retry_wait')
                        AND EXISTS(
                          SELECT 1 FROM library_change_queue_lanes AS lanes
                          WHERE lanes.change_id = library_change_queue.id
                            AND lanes.lane <> 'p0_live'
                        )
                        AND NOT EXISTS(
                          SELECT 1 FROM library_live_gap_recovery_claims AS claim
                          WHERE claim.gap_change_id = library_change_queue.id
                        )",
                    params![
                        now.saturating_add(SCAN_QUEUE_LEASE_MILLIS),
                        now,
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        high_watermark,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO scan_run_catch_up_lineage(
                           scan_id, catch_up_source, catch_up_watermark, enrolled_unix_ms
                         )
                         SELECT ?1, lineage.catch_up_source,
                                lineage.catch_up_watermark,
                                MAX(lineage.enrolled_unix_ms)
                         FROM library_change_queue AS changes
                         JOIN library_change_queue_catch_up_lineage AS lineage
                           ON lineage.change_id = changes.id
                         WHERE changes.root_id = ?2
                           AND changes.root_generation = ?3
                            AND changes.id <= ?4
                            AND changes.authoritative_scan_id = ?1
                            AND changes.status IN ('pending', 'leased', 'retry_wait')
                            AND NOT EXISTS(
                              SELECT 1 FROM library_live_gap_recovery_claims AS claim
                              WHERE claim.gap_change_id = changes.id
                            )
                         GROUP BY lineage.catch_up_source, lineage.catch_up_watermark",
                    params![
                        request.scan_id,
                        root_id,
                        root_generation_at_start,
                        high_watermark,
                    ],
                )
                .map_err(database_error)?;
            let lineage_count = transaction
                .query_row(
                    "SELECT COUNT(*) FROM scan_run_catch_up_lineage
                         WHERE scan_id = ?1",
                    [&request.scan_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(database_error)?;
            if lineage_count > MAX_SCAN_CATCH_UP_LINEAGE {
                return Err(ScanError::new(
                    "catalog_scan_catch_up_lineage_limit_exceeded",
                    "The scan captured too many catch-up watermarks",
                ));
            }
        }
        transaction
            .execute(
                "INSERT INTO scan_directory_frontier(scan_id, relative_path) VALUES (?1, '')",
                [&request.scan_id],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(ScanCheckpoint::default())
    }

    fn resume_scan_owned(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
        owner: ScanOwner,
        publication_identity: Option<&FileIdentityEvidence>,
    ) -> Result<ScanCheckpoint, ScanError> {
        if let Some(identity) = publication_identity {
            validate_root_publication_identity(identity)?;
        }
        let transaction = self.begin_write()?;
        let stored = transaction
            .query_row(
                "SELECT scans.root_id, roots.path, scans.status,
                        scans.max_items, scans.max_entries, scans.preview_edge,
                        scans.last_visited_relative_path, scans.visited_entries,
                        scans.accepted_items, scans.issue_count,
                        scans.root_generation_at_start,
                        scans.requires_previous_snapshot, scans.scan_owner,
                        state.generation, state.is_active
                 FROM scan_runs AS scans
                 JOIN library_roots AS roots ON roots.id = scans.root_id
                 JOIN library_change_root_state AS state ON state.root_id = scans.root_id
                 WHERE scans.id = ?1",
                [&request.scan_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, Option<i64>>(10)?,
                        row.get::<_, bool>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, i64>(13)?,
                        row.get::<_, bool>(14)?,
                    ))
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_resume_missing",
                    "The requested scan checkpoint no longer exists",
                )
            })?;
        let (
            stored_root_id,
            stored_root_path,
            status,
            max_items,
            max_entries,
            preview_edge,
            last_visited_relative_path,
            visited_entries,
            accepted_items,
            issue_count,
            stored_root_generation,
            requires_previous_snapshot,
            stored_owner,
            active_root_generation,
            root_is_active,
        ) = stored;
        let stored_max_items = optional_sqlite_u32(max_items, "item limit")?;
        let stored_max_entries = optional_sqlite_u32(max_entries, "entry limit")?;
        let stored_preview_edge = sqlite_u32(preview_edge, "preview edge")?;
        let is_paused = status == "paused";
        if status != "running" && !is_paused
            || stored_root_id != root_id
            || stored_root_path != root_path
            || stored_max_items != request.max_items
            || stored_max_entries != request.max_entries
            || stored_preview_edge != request.preview_edge
            || stored_root_generation != Some(active_root_generation)
            || !root_is_active
            || stored_owner != owner.as_str()
        {
            return Err(ScanError::new(
                "catalog_scan_resume_mismatch",
                "The stored scan cannot be resumed with different identity, ownership, or parameters",
            ));
        }
        if let Some(identity) = publication_identity {
            let binding = transaction
                .query_row(
                    "SELECT root_id, root_generation, identity_scheme, identity_value
                     FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
                    [&request.scan_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(database_error)?;
            if binding
                .as_ref()
                .is_none_or(|(binding_root, generation, scheme, value)| {
                    binding_root != root_id
                        || Some(*generation) != stored_root_generation
                        || scheme != &identity.scheme
                        || value != &identity.value
                })
            {
                return Err(ScanError::new(
                    "catalog_scan_publication_namespace_mismatch",
                    "The stored scan lacks its original configured-root namespace binding",
                ));
            }
        }
        let checkpoint = ScanCheckpoint {
            last_visited_relative_path,
            visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
            accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
            requires_previous_snapshot,
        };
        let has_conflicting_scan = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM scan_runs
                   WHERE root_id = ?1 AND status IN ('running', 'paused') AND id <> ?2
                 )",
                params![root_id, request.scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if has_conflicting_scan {
            return Err(ScanError::new(
                "catalog_root_scan_in_progress",
                "Another authoritative scan already owns this library root",
            ));
        }
        if is_paused {
            let updated = transaction
                .execute(
                    "UPDATE scan_runs SET status = 'running'
                     WHERE id = ?1 AND status = 'paused'",
                    [&request.scan_id],
                )
                .map_err(database_error)?;
            if updated != 1 {
                return Err(ScanError::new(
                    "catalog_scan_resume_raced",
                    "The scan checkpoint changed before it could be resumed",
                ));
            }
        }
        let checkpoint =
            scan_resumption::resume_checkpoint(&transaction, &request.scan_id, owner, checkpoint)?;
        transaction.commit().map_err(database_error)?;
        Ok(checkpoint)
    }

    pub(crate) fn retire_legacy_automatic_full_scans(
        &mut self,
        retired_unix_ms: i64,
    ) -> Result<u32, ScanError> {
        let scans = {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT scans.id, scans.issue_count
                     FROM scan_runs AS scans
                     WHERE scans.status IN ('running', 'paused')
                       AND scans.scan_owner = 'authoritative_recovery'
                     ORDER BY scans.id",
                )
                .map_err(database_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(database_error)?;
            let mut scans = Vec::new();
            for row in rows {
                let (scan_id, issue_count) = row.map_err(database_error)?;
                scans.push((scan_id, sqlite_unsigned(issue_count, "scan issue count")?));
            }
            scans
        };
        let retired_scan_count = u32::try_from(scans.len()).unwrap_or(u32::MAX);
        for (scan_id, issue_count) in scans {
            self.abandon_scan(&scan_id, "superseded", issue_count)?;
        }
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'superseded', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL, authoritative_scan_id = NULL,
                     superseded_by_change_id = NULL, last_failure_code = NULL,
                     last_failure_message = NULL, updated_unix_ms = ?1
                 WHERE origin = 'consistency_audit'
                   AND intent_kind = 'reconcile' AND scope = 'root'
                   AND status IN ('pending', 'leased', 'retry_wait')",
                [retired_unix_ms],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(retired_scan_count)
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
        self.begin_scan_owned(request, root_id, root_path, ScanOwner::Foreground, None)
    }

    fn resume_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.resume_scan_owned(request, root_id, root_path, ScanOwner::Foreground, None)
    }

    #[cfg(test)]
    fn begin_authoritative_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.begin_scan_owned(
            request,
            root_id,
            root_path,
            ScanOwner::AuthoritativeRecovery,
            None,
        )
    }

    #[cfg(test)]
    fn resume_authoritative_scan(
        &mut self,
        request: &ScanRequest,
        root_id: &str,
        root_path: &str,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.resume_scan_owned(
            request,
            root_id,
            root_path,
            ScanOwner::AuthoritativeRecovery,
            None,
        )
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

    fn load_scan_location_by_file_identity(
        &self,
        scan_id: &str,
        identity: &FileIdentityEvidence,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        if scan_id.is_empty() || scan_id.contains('\0') {
            return Err(ScanError::new(
                "catalog_scan_id_invalid",
                "An authoritative scan identity must be non-empty and contain no NUL bytes",
            ));
        }
        if identity.scheme.is_empty()
            || identity.value.is_empty()
            || identity.scheme.contains('\0')
            || identity.value.contains('\0')
        {
            return Err(ScanError::new(
                "catalog_file_identity_invalid",
                "Authoritative file identity evidence must be non-empty and contain no NUL bytes",
            ));
        }
        catalog_delta::load_scan_location_by_file_identity(self, scan_id, identity)
    }

    fn load_active_location(
        &self,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.scan_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value, locations.source_revision_token,
                        locations.source_generation
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

    fn load_active_location_by_asset_id(
        &self,
        asset_id: &str,
        preferred_location_id: Option<&str>,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.connection
            .query_row(
                "SELECT locations.asset_id, locations.location_id, locations.root_id,
                        locations.scan_id,
                        locations.absolute_path, locations.relative_path,
                        locations.preview_path, locations.file_size,
                        locations.created_unix_ms, locations.modified_unix_ms,
                        locations.width, locations.height,
                        locations.preview_status, locations.preview_issue_code,
                        locations.preview_issue_message, locations.metadata_engine_id,
                        locations.metadata_engine_version, locations.capture_local_time,
                        locations.capture_offset_minutes, locations.capture_time_source,
                        locations.capture_raw_value, locations.file_identity_scheme,
                        locations.file_identity_value, locations.source_revision_token,
                        locations.source_generation
                 FROM library_roots AS roots
                 JOIN asset_locations AS locations
                   ON locations.scan_id = roots.active_scan_id
                 WHERE locations.asset_id = ?1
                 ORDER BY CASE WHEN locations.location_id = ?2 THEN 0 ELSE 1 END,
                          locations.root_id, locations.location_id
                 LIMIT 1",
                params![asset_id, preferred_location_id.unwrap_or_default()],
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
        let location_file_size = sqlite_integer(location.file_size, "file size")?;
        let location_revision_token = location
            .source_revision
            .as_ref()
            .map(source_revision_token)
            .transpose()?;
        let identity_group_baseline = location
            .file_identity
            .as_ref()
            .map(|identity| load_staging_identity_group_state(&self.connection, identity, scan_id))
            .transpose()?
            .flatten();
        if identity_group_baseline.as_ref().is_some_and(|state| {
            state.file_size != location_file_size
                || state.modified_unix_ms != location.modified_unix_ms
                || revisions_conflict(
                    state.source_revision_token.as_deref(),
                    location_revision_token.as_deref(),
                )
        }) {
            revalidate_file_state(&ExpectedFileState {
                absolute_path: location.absolute_path.clone(),
                file_size: location.file_size,
                modified_unix_ms: location.modified_unix_ms,
                file_identity: location.file_identity.clone(),
                source_revision: location.source_revision.clone(),
            })
            .map_err(|issue| ScanError::new(issue.code, issue.message))?;
        }
        self.pending_locations.push(PendingLocation {
            scan_id: scan_id.to_owned(),
            root_id: root_id.to_owned(),
            location: location.clone(),
            identity_group_baseline,
        });
        if self.pending_locations.len() >= LOCATION_STAGE_BATCH {
            self.flush_pending_locations()?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn update_active_preview(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
        request: Option<&PreviewRequest>,
    ) -> Result<(), ScanError> {
        self.update_active_preview_with_authority(location, artifact, request, None)
    }

    fn update_active_preview_with_authority(
        &mut self,
        location: &AssetLocationView,
        artifact: Option<&PreviewArtifact>,
        request: Option<&PreviewRequest>,
        publication_authority: Option<&crate::ports::PreviewPublicationAuthority>,
    ) -> Result<(), ScanError> {
        if let Some(request) = request
            && (request.location_id != location.location_id
                || request.expected_root_id != location.root_id
                || request.expected_scan_id != location.scan_id
                || request.expected_source_generation != location.source_generation
                || (request.expected_source_revision.is_some()
                    && request.expected_source_revision != location.source_revision))
        {
            return Err(ScanError::new(
                "preview_request_context_invalid",
                "The preview result does not belong to its requested source context",
            ));
        }
        let file_size = sqlite_integer(location.file_size, "file size")?;
        let expected_scan_id = request.map_or(location.scan_id.as_str(), |request| {
            request.expected_scan_id.as_str()
        });
        let expected_source_generation = request.map_or(location.source_generation, |request| {
            request.expected_source_generation
        });
        let expected_source_revision = request
            .map_or(location.source_revision.as_ref(), |request| {
                request.expected_source_revision.as_ref()
            })
            .map(source_revision_token)
            .transpose()?;
        let published_source_revision = location
            .source_revision
            .as_ref()
            .map(source_revision_token)
            .transpose()?;
        let expected_root_generation = publication_authority
            .map(|authority| sqlite_integer(authority.root_generation.value(), "root generation"))
            .transpose()?;
        let expected_root_identity_scheme = publication_authority
            .and_then(|authority| authority.root_identity.as_ref())
            .map(|identity| identity.scheme.as_str());
        let expected_root_identity_value = publication_authority
            .and_then(|authority| authority.root_identity.as_ref())
            .map(|identity| identity.value.as_str());
        let transaction = self.begin_write()?;
        if publication_authority.is_some() {
            let authority_is_current = transaction
                .query_row(
                    "SELECT EXISTS(
                       SELECT 1
                       FROM library_change_root_state AS root_state
                       WHERE root_state.root_id = ?1
                         AND root_state.generation = ?2
                         AND root_state.is_active = 1
                         AND (
                           (
                             ?3 IS NULL AND ?4 IS NULL
                             AND NOT EXISTS (
                               SELECT 1
                               FROM library_root_publication_namespaces AS namespace
                               WHERE namespace.root_id = root_state.root_id
                             )
                           )
                           OR EXISTS (
                             SELECT 1
                             FROM library_root_publication_namespaces AS namespace
                             WHERE namespace.root_id = root_state.root_id
                               AND namespace.root_generation = root_state.generation
                               AND namespace.identity_scheme = ?3
                               AND namespace.identity_value = ?4
                           )
                         )
                     )",
                    params![
                        location.root_id,
                        expected_root_generation,
                        expected_root_identity_scheme,
                        expected_root_identity_value,
                    ],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if !authority_is_current {
                return Err(ScanError::new(
                    "active_preview_authority_stale",
                    "The library root publication authority changed before its preview was updated",
                ));
            }
        }
        if let Some(artifact) = artifact {
            if location.source_revision.is_none() || location.source_generation == 0 {
                return Err(ScanError::new(
                    "preview_source_revision_unproven",
                    "A preview cannot be published before the source revision is proven",
                ));
            }
            let artifact_bytes = sqlite_integer(artifact.byte_size, "preview artifact size")?;
            transaction
                .execute(
                    "DELETE FROM preview_artifact_locations
                     WHERE location_id = ?1
                       AND artifact_key IN (
                         SELECT artifact_key FROM preview_artifacts
                         WHERE algorithm_id = ?2
                           AND orientation_contract = ?3
                           AND size_bucket = ?4
                           AND artifact_key <> ?5
                       )",
                    params![
                        location.location_id,
                        artifact.algorithm_id,
                        artifact.orientation_contract,
                        i64::from(artifact.size_bucket),
                        artifact.artifact_key,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "UPDATE preview_artifacts
                     SET lifecycle_state = 'stale'
                     WHERE lifecycle_state = 'ready'
                       AND algorithm_id = ?1
                       AND orientation_contract = ?2
                       AND size_bucket = ?3
                       AND artifact_key <> ?4
                       AND NOT EXISTS (
                         SELECT 1 FROM preview_artifact_locations AS owners
                         WHERE owners.artifact_key = preview_artifacts.artifact_key
                       )
                       AND NOT EXISTS (
                         SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                         WHERE handoffs.preview_status = 'ready'
                           AND handoffs.preview_path = preview_artifacts.artifact_path
                       )
                       AND NOT EXISTS (
                         SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                         WHERE handoffs.preview_status = 'ready'
                           AND handoffs.preview_path = preview_artifacts.artifact_path
                       )",
                    params![
                        artifact.algorithm_id,
                        artifact.orientation_contract,
                        i64::from(artifact.size_bucket),
                        artifact.artifact_key,
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT INTO preview_artifacts(
                       artifact_key, source_file_size, source_modified_unix_ms,
                       source_identity_scheme, source_identity_value, source_revision_token,
                       source_generation, algorithm_id,
                       algorithm_version, orientation_contract, size_bucket, encoded_width,
                       encoded_height, artifact_path, byte_size, lifecycle_state,
                       created_unix_ms, last_used_unix_ms
                     ) VALUES (
                       ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                       ?14, ?15, 'ready', ?16, ?16
                     )
                     ON CONFLICT(artifact_key) DO UPDATE SET
                       source_file_size = excluded.source_file_size,
                       source_modified_unix_ms = excluded.source_modified_unix_ms,
                       source_identity_scheme = excluded.source_identity_scheme,
                       source_identity_value = excluded.source_identity_value,
                       source_revision_token = excluded.source_revision_token,
                       source_generation = excluded.source_generation,
                       algorithm_id = excluded.algorithm_id,
                       algorithm_version = excluded.algorithm_version,
                       orientation_contract = excluded.orientation_contract,
                       size_bucket = excluded.size_bucket,
                       encoded_width = excluded.encoded_width,
                       encoded_height = excluded.encoded_height,
                       artifact_path = excluded.artifact_path,
                       byte_size = excluded.byte_size,
                       lifecycle_state = 'ready',
                       last_used_unix_ms = excluded.last_used_unix_ms",
                    params![
                        artifact.artifact_key,
                        file_size,
                        location.modified_unix_ms,
                        location
                            .file_identity
                            .as_ref()
                            .map(|identity| &identity.scheme),
                        location
                            .file_identity
                            .as_ref()
                            .map(|identity| &identity.value),
                        location
                            .source_revision
                            .as_ref()
                            .map(source_revision_token)
                            .transpose()?,
                        sqlite_integer(location.source_generation, "source generation")?,
                        artifact.algorithm_id,
                        i64::from(artifact.algorithm_version),
                        artifact.orientation_contract,
                        i64::from(artifact.size_bucket),
                        i64::from(artifact.encoded_width),
                        i64::from(artifact.encoded_height),
                        artifact.path,
                        artifact_bytes,
                        unix_time_ms(),
                    ],
                )
                .map_err(database_error)?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO preview_artifact_locations(
                       artifact_key, location_id
                     ) VALUES (?1, ?2)",
                    params![artifact.artifact_key, location.location_id],
                )
                .map_err(database_error)?;
        } else if !matches!(location.preview_status, PreviewStatus::Ready) {
            transaction
                .execute(
                    "DELETE FROM preview_artifact_locations WHERE location_id = ?1",
                    [&location.location_id],
                )
                .map_err(database_error)?;
            mark_unreferenced_preview_artifacts_stale(&transaction)?;
        }
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = ?2, width = ?3, height = ?4,
                      preview_status = ?5, preview_issue_code = ?6,
                      preview_issue_message = ?7, source_revision_token = ?14,
                      metadata_engine_id = ?21, metadata_engine_version = ?22,
                      capture_local_time = ?23, capture_offset_minutes = ?24,
                      capture_time_source = ?25, capture_raw_value = ?26
                 WHERE location_id = ?1 AND file_size = ?8 AND modified_unix_ms = ?9
                    AND root_id = ?10 AND absolute_path = ?11
                    AND file_identity_scheme IS ?12 AND file_identity_value IS ?13
                    AND scan_id = ?15 AND source_generation = ?16
                    AND (
                      source_revision_token IS ?17
                      OR (?17 IS NULL AND source_revision_token = ?14)
                    )
                    AND (
                      ?14 IS NULL OR file_identity_scheme IS NULL OR NOT EXISTS (
                        SELECT 1
                        FROM asset_locations AS sibling
                        JOIN library_roots AS sibling_root
                          ON sibling_root.id = sibling.root_id
                         AND sibling_root.active_scan_id = sibling.scan_id
                        WHERE sibling.file_identity_scheme = asset_locations.file_identity_scheme
                          AND sibling.file_identity_value = asset_locations.file_identity_value
                          AND sibling.source_generation = ?16
                          AND sibling.source_revision_token IS NOT NULL
                          AND sibling.source_revision_token <> ?14
                      )
                    )
                    AND scan_id = (
                      SELECT active_scan_id FROM library_roots WHERE id = ?10
                    )
                    AND (
                      ?18 IS NULL OR (
                        EXISTS (
                          SELECT 1
                          FROM library_change_root_state AS root_state
                          WHERE root_state.root_id = ?10
                            AND root_state.generation = ?18
                            AND root_state.is_active = 1
                        )
                        AND (
                          (
                            ?19 IS NULL AND ?20 IS NULL
                            AND NOT EXISTS (
                              SELECT 1
                              FROM library_root_publication_namespaces AS namespace
                              WHERE namespace.root_id = ?10
                            )
                          )
                          OR EXISTS (
                            SELECT 1
                            FROM library_root_publication_namespaces AS namespace
                            WHERE namespace.root_id = ?10
                              AND namespace.root_generation = ?18
                              AND namespace.identity_scheme = ?19
                              AND namespace.identity_value = ?20
                          )
                        )
                      )
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
                    location.root_id,
                    location.absolute_path,
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.scheme),
                    location
                        .file_identity
                        .as_ref()
                        .map(|identity| &identity.value),
                    published_source_revision,
                    expected_scan_id,
                    sqlite_integer(expected_source_generation, "source generation")?,
                    expected_source_revision,
                    expected_root_generation,
                    expected_root_identity_scheme,
                    expected_root_identity_value,
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
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "active_preview_location_stale",
                "The active catalog location changed before its preview was updated",
            ));
        }
        if let (Some(identity), Some(revision)) = (
            location.file_identity.as_ref(),
            published_source_revision.as_ref(),
        ) {
            transaction
                .execute(
                    "UPDATE asset_locations
                     SET source_revision_token = ?3
                     WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
                       AND source_generation = ?4 AND source_revision_token IS NULL
                       AND EXISTS (
                         SELECT 1 FROM library_roots AS roots
                         WHERE roots.id = asset_locations.root_id
                           AND roots.active_scan_id = asset_locations.scan_id
                       )",
                    params![
                        identity.scheme,
                        identity.value,
                        revision,
                        sqlite_integer(location.source_generation, "source generation")?,
                    ],
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)
    }

    fn reset_all_previews_for_cleanup(&mut self) -> Result<u64, ScanError> {
        self.flush_pending_locations()?;
        let transaction = self.begin_write()?;
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> '' OR preview_status <> 'pending'
                   OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
                [],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_catch_up_handoffs
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> '' OR preview_status <> 'pending'
                   OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
                [],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_scan_handoff_items
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> '' OR preview_status <> 'pending'
                   OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
                [],
            )
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM preview_artifacts", [])
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_cleanup_count_invalid",
                "The preview cleanup update count exceeds the supported range",
            )
        })
    }

    fn reset_previews_outside_root(&mut self, preview_root_prefix: &str) -> Result<u64, ScanError> {
        if preview_root_prefix.is_empty() {
            return Err(ScanError::new(
                "preview_root_prefix_empty",
                "The active preview root prefix is required",
            ));
        }
        self.flush_pending_locations()?;
        let transaction = self.begin_write()?;
        let updated = transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> ''
                   AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_catch_up_handoffs
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> ''
                   AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_scan_handoff_items
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path <> ''
                   AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM preview_artifacts
                 WHERE lower(substr(artifact_path, 1, length(?1))) <> lower(?1)",
                [preview_root_prefix],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_root_reset_count_invalid",
                "The preview-root reset count exceeds the supported range",
            )
        })
    }

    fn is_preview_artifact_path_indexed(
        &self,
        path: &str,
        artifact_key: Option<&str>,
    ) -> Result<bool, ScanError> {
        self.connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM preview_artifacts
                   WHERE lower(artifact_path) = lower(?1)
                      OR (?2 IS NOT NULL AND artifact_key = ?2)
                 )",
                params![path, artifact_key],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn load_preview_recovery_artifacts(
        &self,
        preview_root_prefix: &str,
        after_artifact_key: Option<&str>,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError> {
        if preview_root_prefix.is_empty() || limit == 0 || limit > 4_096 {
            return Err(ScanError::new(
                "preview_recovery_query_invalid",
                "Preview recovery requires a root prefix and a limit between 1 and 4096",
            ));
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT artifact_key, artifact_path
                 FROM preview_artifacts
                 WHERE lower(substr(artifact_path, 1, length(?1))) = lower(?1)
                   AND (?2 IS NULL OR artifact_key > ?2)
                 ORDER BY artifact_key
                 LIMIT ?3",
            )
            .map_err(database_error)?;
        let rows = statement
            .query_map(
                params![preview_root_prefix, after_artifact_key, i64::from(limit)],
                |row| {
                    Ok(PreviewReclamationCandidate {
                        artifact_key: row.get(0)?,
                        path: row.get(1)?,
                    })
                },
            )
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(rows)
    }

    fn try_reconcile_preview_health(
        &mut self,
        target: crate::ports::PreviewHealthTarget<'_>,
        observation: crate::ports::PreviewHealthObservation,
    ) -> Result<crate::ports::PreviewHealthOutcome, ScanError> {
        self.try_reconcile_preview_health_owned(target, observation)
    }

    fn touch_preview_artifacts(
        &mut self,
        artifacts: &[(String, String)],
    ) -> Result<u64, ScanError> {
        if artifacts.len() > 4_096 {
            return Err(ScanError::new(
                "preview_touch_batch_too_large",
                "Preview usage updates are limited to 4096 artifacts per batch",
            ));
        }
        if artifacts.is_empty() {
            return Ok(0);
        }
        let now = unix_time_ms();
        let oldest_retained = now.saturating_sub(60_000);
        let transaction = self.begin_write()?;
        let mut statement = transaction
            .prepare_cached(
                "UPDATE preview_artifacts
                 SET last_used_unix_ms = ?3
                 WHERE artifact_key IN (
                     SELECT artifact_key FROM preview_artifact_locations
                     WHERE location_id = ?1
                 )
                   AND artifact_path = ?2 AND last_used_unix_ms < ?4",
            )
            .map_err(database_error)?;
        let mut updated = 0_usize;
        for (location_id, artifact_path) in artifacts {
            updated = updated.saturating_add(
                statement
                    .execute(params![location_id, artifact_path, now, oldest_retained])
                    .map_err(database_error)?,
            );
        }
        drop(statement);
        transaction.commit().map_err(database_error)?;
        u64::try_from(updated).map_err(|_| {
            ScanError::new(
                "preview_touch_count_invalid",
                "The preview usage update count exceeds the supported range",
            )
        })
    }

    fn load_preview_reclamation_candidates(
        &self,
        protected_location_ids: &[String],
        current_algorithm_id: &str,
        current_algorithm_version: u32,
        current_orientation_contract: &str,
        current_preview_root_prefix: &str,
        limit: u32,
    ) -> Result<Vec<PreviewReclamationCandidate>, ScanError> {
        if limit == 0 || limit > 4_096 {
            return Err(ScanError::new(
                "preview_reclamation_limit_invalid",
                "Preview reclamation candidate limits must be between 1 and 4096",
            ));
        }
        let protected = protected_location_ids
            .iter()
            .filter(|location_id| !location_id.is_empty())
            .collect::<HashSet<_>>();
        let mut values = vec![
            Value::Text(current_algorithm_id.to_owned()),
            Value::Integer(i64::from(current_algorithm_version)),
            Value::Text(current_orientation_contract.to_owned()),
            Value::Text(current_preview_root_prefix.to_owned()),
        ];
        let protected_clause = if protected.is_empty() {
            String::new()
        } else {
            let placeholders = protected
                .iter()
                .map(|location_id| {
                    values.push(Value::Text((*location_id).clone()));
                    format!("?{}", values.len())
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "AND artifact_key NOT IN (
                   SELECT artifact_key FROM preview_artifact_locations
                   WHERE location_id IN ({placeholders})
                 )"
            )
        };
        values.push(Value::Integer(i64::from(limit)));
        let limit_parameter = values.len();
        let query = format!(
            "SELECT artifact_key, artifact_path
             FROM preview_artifacts
             WHERE lower(substr(artifact_path, 1, length(?4))) = lower(?4)
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
             {protected_clause}
             ORDER BY
               CASE
                 WHEN lifecycle_state <> 'ready' THEN 0
                 WHEN algorithm_id <> ?1 OR algorithm_version <> ?2
                   OR orientation_contract <> ?3 THEN 1
                 ELSE 2
               END,
               last_used_unix_ms, artifact_key
             LIMIT ?{limit_parameter}"
        );
        let mut statement = self.connection.prepare(&query).map_err(database_error)?;
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut candidates = Vec::new();
        for row in rows {
            let (artifact_key, path) = row.map_err(database_error)?;
            candidates.push(PreviewReclamationCandidate { artifact_key, path });
        }
        Ok(candidates)
    }

    fn remove_reclaimed_preview(
        &mut self,
        candidate: &PreviewReclamationCandidate,
    ) -> Result<bool, ScanError> {
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "UPDATE asset_locations
                 SET preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE preview_path = ?1
                   AND location_id IN (
                     SELECT location_id FROM preview_artifact_locations
                     WHERE artifact_key = ?2
                   )",
                params![candidate.path, candidate.artifact_key],
            )
            .map_err(database_error)?;
        let deleted = transaction
            .execute(
                "DELETE FROM preview_artifacts
                 WHERE artifact_key = ?1 AND artifact_path = ?2
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                     WHERE handoffs.preview_status = 'ready'
                       AND handoffs.preview_path = preview_artifacts.artifact_path
                   )
                   AND NOT EXISTS (
                     SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                     WHERE handoffs.preview_status = 'ready'
                       AND handoffs.preview_path = preview_artifacts.artifact_path
                   )",
                params![candidate.artifact_key, candidate.path],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Ok(false);
        }
        transaction.commit().map_err(database_error)?;
        Ok(true)
    }

    fn record_issue(&mut self, scan_id: &str, issue: &ScanIssue) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO scan_issues(scan_id, path, code, message)
                 VALUES (?1, ?2, ?3, ?4)",
                params![scan_id, issue.path, issue.code, issue.message],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
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
        let transaction = self.begin_write()?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET last_visited_relative_path = ?2, visited_entries = ?3,
                     accepted_items = ?4, issue_count = ?5,
                     requires_previous_snapshot = ?6
                 WHERE id = ?1 AND status IN ('running', 'paused')",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_checkpointable",
                "The scan is no longer in a checkpointable running state",
            ));
        }
        transaction.commit().map_err(database_error)?;
        Ok(())
    }

    #[cfg(test)]
    fn load_recoverable_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "running", ScanOwner::Foreground)
    }

    #[cfg(test)]
    fn load_paused_scan(&self) -> Result<Option<RecoverableScan>, ScanError> {
        load_scan_with_status(&self.connection, "paused", ScanOwner::Foreground)
    }

    #[cfg(test)]
    fn load_authoritative_recoverable_scan_after(
        &self,
        after_scan_id: Option<&str>,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        load_scans_with_status(
            &self.connection,
            "running",
            ScanOwner::AuthoritativeRecovery,
            after_scan_id,
            1,
        )
        .map(|mut scans| scans.pop())
    }

    fn claim_next_directory(&mut self, scan_id: &str) -> Result<Option<String>, ScanError> {
        let transaction = self.begin_write()?;
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
        let transaction = self.begin_write()?;
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
        let transaction = self.begin_write()?;
        let updated = transaction
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
        transaction.commit().map_err(database_error)?;
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
        let page = if after.is_some() {
            "AND relative_path > ?3 ORDER BY relative_path LIMIT ?4"
        } else {
            "ORDER BY relative_path LIMIT ?3"
        };
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT relative_path FROM scan_directory_entries
                 WHERE scan_id = ?1 AND directory_relative_path = ?2
                   {page}",
            ))
            .map_err(database_error)?;
        let map_row = |row: &rusqlite::Row<'_>| row.get::<_, String>(0);
        let rows = match after {
            Some(after) => statement.query_map(
                params![scan_id, relative_directory, after, i64::from(limit)],
                map_row,
            ),
            None => statement.query_map(
                params![scan_id, relative_directory, i64::from(limit)],
                map_row,
            ),
        }
        .map_err(database_error)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(database_error)?);
        }
        Ok(entries)
    }

    fn enqueue_directory(&mut self, scan_id: &str, relative_path: &str) -> Result<(), ScanError> {
        let transaction = self.begin_write()?;
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
        let transaction = self.begin_write()?;
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
                     visited_entries = ?2, accepted_items = ?3, issue_count = ?4,
                     requires_previous_snapshot = ?5
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
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
        let transaction = self.begin_write()?;
        let updated = transaction
            .execute(
                "UPDATE scan_runs
                 SET status = 'paused', last_visited_relative_path = ?2,
                     visited_entries = ?3, accepted_items = ?4, issue_count = ?5,
                     requires_previous_snapshot = ?6
                 WHERE id = ?1 AND status = 'running'",
                params![
                    scan_id,
                    checkpoint.last_visited_relative_path,
                    visited_entries,
                    accepted_items,
                    issue_count,
                    checkpoint.requires_previous_snapshot,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_scan_not_pausable",
                "The scan is no longer in a pausable running state",
            ));
        }
        transaction.commit().map_err(database_error)?;
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
    ) -> Result<Vec<(String, String, ExpectedFileState)>, ScanError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT location_id, relative_path, absolute_path, file_size, modified_unix_ms,
                        file_identity_scheme, file_identity_value, source_revision_token
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
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                },
            )
            .map_err(database_error)?;
        let mut states = Vec::new();
        for row in rows {
            let (
                location_id,
                relative_path,
                absolute_path,
                file_size,
                modified_unix_ms,
                identity_scheme,
                identity_value,
                source_revision_token,
            ) = row.map_err(database_error)?;
            let file_size = u64::try_from(file_size).map_err(|_| {
                ScanError::new(
                    "catalog_integer_invalid",
                    "A staged file size is outside the supported range",
                )
            })?;
            states.push((
                location_id,
                relative_path,
                ExpectedFileState {
                    absolute_path,
                    file_size,
                    modified_unix_ms,
                    file_identity: stored_file_identity(identity_scheme, identity_value)?,
                    source_revision: stored_source_revision(source_revision_token)?,
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
        scan_publication::publish_scan(self, scan_id, root_id, asset_count, issue_count)
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
        let now = unix_time_ms();
        let transaction = self.begin_write()?;
        abandon_scan_transaction(&transaction, scan_id, status, issue_count, now)?;
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
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
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
                "A gallery request accepts only one page cursor or anchor",
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
        } else if effective_after.is_some() || anchor.is_some() {
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
            query_anchor_resolution: None,
        })
    }

    fn load_snapshot_around_location(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor_location_id: &str,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        for _ in 0..3 {
            let transaction = self.connection.transaction().map_err(database_error)?;
            let revision = load_catalog_revision(&transaction)?;
            let (resolution, predecessor) = resolve_gallery_location_anchor(
                &transaction,
                revision,
                query,
                query_id,
                anchor_location_id,
                max_items,
            )?;
            transaction.commit().map_err(database_error)?;
            match self.load_snapshot(max_items, query, query_id, predecessor.as_ref(), None, None) {
                Ok(mut snapshot) if snapshot.revision == revision => {
                    snapshot.query_anchor_resolution = Some(resolution);
                    return Ok(snapshot);
                }
                Ok(_) => continue,
                Err(error) if error.code == "catalog_cursor_stale" => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ScanError::new(
            "catalog_cursor_stale",
            "The catalog kept changing while the gallery location anchor was resolved",
        ))
    }

    fn load_snapshot_around_asset(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        requested_location_id: &str,
        anchor_asset_id: &str,
        fallback_ordinal: u64,
    ) -> Result<CatalogSnapshot, ScanError> {
        if max_items == 0 || max_items > MAX_CATALOG_PAGE_ITEMS {
            return Err(ScanError::new(
                "catalog_page_limit_invalid",
                format!(
                    "The catalog page limit must be between 1 and {MAX_CATALOG_PAGE_ITEMS} items"
                ),
            ));
        }
        validate_gallery_query(query)?;
        for _ in 0..3 {
            let transaction = self.connection.transaction().map_err(database_error)?;
            let revision = load_catalog_revision(&transaction)?;
            let (resolution, predecessor) = resolve_gallery_asset_anchor(
                &transaction,
                revision,
                query,
                query_id,
                max_items,
                GalleryAssetAnchor {
                    requested_location_id,
                    asset_id: anchor_asset_id,
                    fallback_ordinal,
                },
            )?;
            transaction.commit().map_err(database_error)?;
            match self.load_snapshot(max_items, query, query_id, predecessor.as_ref(), None, None) {
                Ok(mut snapshot) if snapshot.revision == revision => {
                    snapshot.query_anchor_resolution = Some(resolution);
                    return Ok(snapshot);
                }
                Ok(_) => continue,
                Err(error) if error.code == "catalog_cursor_stale" => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ScanError::new(
            "catalog_cursor_stale",
            "The catalog kept changing while the gallery asset anchor was resolved",
        ))
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
        let transaction = self.begin_user_interactive_write()?;
        let root_exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1)",
                [root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !root_exists {
            transaction.commit().map_err(database_error)?;
            return Ok(false);
        }
        retire_root_change_queue(&transaction, root_id, unix_time_ms())?;
        spool_retirement::retire_owned_spools(
            &transaction,
            spool_retirement::SpoolOwner::Root(root_id),
        )?;
        persistent_journal::remove_root_persistent_journal_state(&transaction, root_id)?;
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
        prepare_root_unregister_scope(&transaction, root_id)?;
        detach_preview_references_for_root_locations(&transaction, root_id, None)?;
        mark_root_unregister_preview_artifacts_stale(&transaction)?;
        transaction
            .execute("DELETE FROM asset_locations WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        transaction
            .execute("DELETE FROM scan_runs WHERE root_id = ?1", [root_id])
            .map_err(database_error)?;
        let removed = transaction
            .execute("DELETE FROM library_roots WHERE id = ?1", [root_id])
            .map_err(database_error)?;
        if removed != 1 {
            return Err(ScanError::new(
                "catalog_root_unregister_failed",
                "The registered library root could not be removed",
            ));
        }
        change_queue::root_retirement::retire_removed_root_recovery_authorities(
            &transaction,
            root_id,
        )?;
        delete_root_unregister_orphan_assets(&transaction)?;
        let revision_updated = transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        if revision_updated != 1 {
            return Err(ScanError::new(
                "catalog_revision_unavailable",
                "The catalog revision state is missing or invalid",
            ));
        }
        clear_root_unregister_scope(&transaction)?;
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

fn validate_root_publication_identity(identity: &FileIdentityEvidence) -> Result<(), ScanError> {
    let value = identity.value.as_bytes();
    let valid = identity.scheme == "windows-file-id-128-v1"
        && value.len() == 49
        && value.get(16) == Some(&b':')
        && value.iter().enumerate().all(|(index, byte)| {
            index == 16 || byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
        });
    if valid {
        Ok(())
    } else {
        Err(ScanError::new(
            "catalog_root_publication_namespace_identity_invalid",
            "Configured-root publication requires canonical full Windows file identity evidence",
        ))
    }
}

pub(super) fn establish_root_publication_namespace(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: i64,
    identity: &FileIdentityEvidence,
    authority_kind: &str,
    catalog_revision: u64,
    established_unix_ms: i64,
) -> Result<(), ScanError> {
    validate_root_publication_identity(identity)?;
    if root_generation <= 0
        || established_unix_ms < 0
        || !matches!(authority_kind, "metadata_inventory" | "foreground_scan")
    {
        return Err(ScanError::new(
            "catalog_root_publication_namespace_invalid",
            "Configured-root publication authority is outside the supported contract",
        ));
    }
    let is_current = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_change_root_state
               WHERE root_id = ?1 AND generation = ?2 AND is_active = 1
             )",
            params![root_id, root_generation],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !is_current {
        return Err(ScanError::new(
            "catalog_root_publication_namespace_generation_stale",
            "Configured-root publication authority no longer belongs to the active generation",
        ));
    }
    let existing = transaction
        .query_row(
            "SELECT root_generation, identity_scheme, identity_value
             FROM library_root_publication_namespaces WHERE root_id = ?1",
            [root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    if existing
        .as_ref()
        .is_some_and(|(generation, scheme, value)| {
            *generation != root_generation || scheme != &identity.scheme || value != &identity.value
        })
    {
        return Err(ScanError::new(
            "catalog_root_publication_namespace_conflict",
            "Configured-root publication evidence conflicts with the active authority",
        ));
    }
    let catalog_revision = sqlite_integer(catalog_revision, "catalog revision")?;
    if existing.is_some() {
        let updated = transaction
            .execute(
                "UPDATE library_root_publication_namespaces
                 SET authority_kind = ?2, updated_unix_ms = MAX(updated_unix_ms, ?3)
                 WHERE root_id = ?1 AND root_generation = ?4
                   AND identity_scheme = ?5 AND identity_value = ?6",
                params![
                    root_id,
                    authority_kind,
                    established_unix_ms,
                    root_generation,
                    identity.scheme,
                    identity.value,
                ],
            )
            .map_err(database_error)?;
        if updated != 1 {
            return Err(ScanError::new(
                "catalog_root_publication_namespace_raced",
                "Configured-root publication authority changed during atomic publication",
            ));
        }
    } else {
        transaction
            .execute(
                "INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value,
                   authority_kind, established_catalog_revision,
                   established_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    root_id,
                    root_generation,
                    identity.scheme,
                    identity.value,
                    authority_kind,
                    catalog_revision,
                    established_unix_ms,
                ],
            )
            .map_err(database_error)?;
    }
    Ok(())
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

fn load_active_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_identity_group_state_query(
        connection,
        "SELECT MIN(locations.source_generation), MAX(locations.source_generation),
                COUNT(DISTINCT locations.source_revision_token),
                MAX(locations.source_revision_token),
                MIN(locations.file_size), MAX(locations.file_size),
                MIN(locations.modified_unix_ms), MAX(locations.modified_unix_ms)
         FROM asset_locations AS locations
         JOIN library_roots AS roots
           ON roots.id = locations.root_id
          AND roots.active_scan_id = locations.scan_id
         WHERE locations.file_identity_scheme = ?1
           AND locations.file_identity_value = ?2",
        params![identity.scheme, identity.value],
    )
}

fn load_scan_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
    scan_id: &str,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_identity_group_state_query(
        connection,
        "SELECT MIN(source_generation), MAX(source_generation),
                COUNT(DISTINCT source_revision_token), MAX(source_revision_token),
                MIN(file_size), MAX(file_size),
                MIN(modified_unix_ms), MAX(modified_unix_ms)
         FROM asset_locations
         WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
           AND scan_id = ?3",
        params![identity.scheme, identity.value, scan_id],
    )
}

fn load_staging_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
    scan_id: &str,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_scan_identity_group_state(connection, identity, scan_id)?.map_or_else(
        || load_active_identity_group_state(connection, identity),
        |state| Ok(Some(state)),
    )
}

fn load_identity_group_state_query<P>(
    connection: &Connection,
    query: &str,
    parameters: P,
) -> Result<Option<StoredIdentityGroupState>, ScanError>
where
    P: rusqlite::Params,
{
    let (
        minimum_generation,
        maximum_generation,
        known_revision_count,
        source_revision_token,
        minimum_file_size,
        maximum_file_size,
        minimum_modified_unix_ms,
        maximum_modified_unix_ms,
    ) = connection
        .query_row(query, parameters, |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })
        .map_err(database_error)?;
    let Some(generation) = minimum_generation else {
        return Ok(None);
    };
    if maximum_generation != Some(generation)
        || known_revision_count > 1
        || minimum_file_size != maximum_file_size
        || minimum_modified_unix_ms != maximum_modified_unix_ms
    {
        return Err(ScanError::new(
            "catalog_source_identity_state_unverifiable",
            "The catalog contains conflicting source state for one physical file identity",
        ));
    }
    Ok(Some(StoredIdentityGroupState {
        generation,
        source_revision_token,
        file_size: minimum_file_size.expect("an identity group has a file size"),
        modified_unix_ms: minimum_modified_unix_ms
            .expect("an identity group has a modification time"),
    }))
}

fn revisions_conflict(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

fn source_observation_superseded() -> ScanError {
    ScanError::new(
        "catalog_source_observation_superseded",
        "A newer observation already owns this physical file identity",
    )
}

fn persist_location(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    location: &AssetLocationView,
    identity_generation_expectation: IdentityGenerationExpectation,
    identity_generation_batch: &mut HashMap<(String, String), IdentityGenerationBatchState>,
) -> Result<PersistLocationOutcome, ScanError> {
    let file_size = sqlite_integer(location.file_size, "file size")?;
    let source_revision_token = location
        .source_revision
        .as_ref()
        .map(source_revision_token)
        .transpose()?;
    let requested_generation = (location.source_generation != 0)
        .then(|| sqlite_integer(location.source_generation, "source generation"))
        .transpose()?;
    let is_scan_staging = !matches!(
        &identity_generation_expectation,
        IdentityGenerationExpectation::ResolveCurrent
    );
    let mut should_fan_out = false;
    let source_generation = if let Some(identity) = location.file_identity.as_ref() {
        let key = (identity.scheme.clone(), identity.value.clone());
        if let Some(batch_state) = identity_generation_batch.get_mut(&key) {
            if let IdentityGenerationExpectation::Captured(expected) =
                &identity_generation_expectation
                && expected != &batch_state.baseline_state
            {
                return Err(source_observation_superseded());
            }
            if batch_state.file_size != file_size
                || batch_state.modified_unix_ms != location.modified_unix_ms
                || revisions_conflict(
                    batch_state.source_revision_token.as_deref(),
                    source_revision_token.as_deref(),
                )
            {
                return Err(source_observation_superseded());
            }
            if batch_state.source_revision_token.is_none() {
                batch_state
                    .source_revision_token
                    .clone_from(&source_revision_token);
            }
            if requested_generation.is_some_and(|requested| {
                requested
                    != batch_state
                        .baseline_state
                        .as_ref()
                        .map_or(requested, |state| state.generation)
                    && requested != batch_state.assigned_generation
            }) {
                return Err(source_observation_superseded());
            }
            batch_state.assigned_generation
        } else {
            let group_state = if is_scan_staging {
                load_staging_identity_group_state(transaction, identity, scan_id)?
            } else {
                load_active_identity_group_state(transaction, identity)?
            };
            let current_generation = group_state.as_ref().map(|state| state.generation);
            if let IdentityGenerationExpectation::Captured(expected) =
                &identity_generation_expectation
                && expected != &group_state
            {
                return Err(source_observation_superseded());
            }
            if let Some(requested) = requested_generation {
                let incompatible_state = group_state.as_ref().is_some_and(|state| {
                    state.file_size != file_size
                        || state.modified_unix_ms != location.modified_unix_ms
                        || revisions_conflict(
                            state.source_revision_token.as_deref(),
                            source_revision_token.as_deref(),
                        )
                });
                let generation_conflicts = match identity_generation_expectation {
                    IdentityGenerationExpectation::MirrorCurrentActive => current_generation
                        .is_some_and(|current| {
                            current > requested || current == requested && incompatible_state
                        }),
                    IdentityGenerationExpectation::Captured(_)
                    | IdentityGenerationExpectation::ResolveCurrent => {
                        current_generation.is_some_and(|current| current != requested)
                            || incompatible_state
                    }
                };
                if generation_conflicts {
                    return Err(source_observation_superseded());
                }
            }
            let observation_matches_group = group_state.as_ref().is_some_and(|state| {
                state.file_size == file_size
                    && state.modified_unix_ms == location.modified_unix_ms
                    && !revisions_conflict(
                        state.source_revision_token.as_deref(),
                        source_revision_token.as_deref(),
                    )
            });
            let assigned_generation = match requested_generation {
                Some(requested) => requested,
                None if is_scan_staging && observation_matches_group => {
                    current_generation.expect("a matching group has a source generation")
                }
                None => {
                    should_fan_out = !is_scan_staging;
                    allocate_source_generation(transaction)?
                }
            };
            identity_generation_batch.insert(
                key,
                IdentityGenerationBatchState {
                    baseline_state: group_state,
                    assigned_generation,
                    source_revision_token: source_revision_token.clone(),
                    file_size,
                    modified_unix_ms: location.modified_unix_ms,
                },
            );
            assigned_generation
        }
    } else if let Some(requested) = requested_generation {
        requested
    } else {
        allocate_source_generation(transaction)?
    };
    let mut active_projection_changed = false;
    if should_fan_out && let Some(identity) = location.file_identity.as_ref() {
        active_projection_changed = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM asset_locations AS locations
                   JOIN library_roots AS roots
                     ON roots.id = locations.root_id
                    AND roots.active_scan_id = locations.scan_id
                   WHERE locations.file_identity_scheme = ?1
                     AND locations.file_identity_value = ?2
                 )",
                params![identity.scheme, identity.value],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM preview_artifact_locations
                 WHERE location_id IN (
                   SELECT locations.location_id
                   FROM asset_locations AS locations
                   JOIN library_roots AS roots
                     ON roots.id = locations.root_id
                    AND roots.active_scan_id = locations.scan_id
                   WHERE locations.file_identity_scheme = ?1
                     AND locations.file_identity_value = ?2
                 )",
                params![identity.scheme, identity.value],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "DELETE FROM library_terminal_media_evidence
                 WHERE file_identity_scheme = ?1 AND file_identity_value = ?2",
                params![identity.scheme, identity.value],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE asset_locations
                 SET file_size = ?3, created_unix_ms = ?4, modified_unix_ms = ?5,
                     file_local_time = strftime(
                       '%Y-%m-%dT%H:%M:%f', COALESCE(?4, ?5) / 1000.0,
                       'unixepoch', 'localtime'
                     ),
                     width = ?6, height = ?7,
                     metadata_engine_id = ?8, metadata_engine_version = ?9,
                     capture_local_time = ?10, capture_offset_minutes = ?11,
                     capture_time_source = ?12, capture_raw_value = ?13,
                     source_revision_token = ?14, source_generation = ?15,
                     preview_path = '', preview_status = 'pending',
                     preview_issue_code = NULL, preview_issue_message = NULL
                 WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
                   AND (
                     EXISTS (
                       SELECT 1 FROM library_roots AS roots
                       WHERE roots.id = asset_locations.root_id
                         AND roots.active_scan_id = asset_locations.scan_id
                     )
                     OR EXISTS (
                       SELECT 1 FROM scan_runs AS running
                       WHERE running.id = asset_locations.scan_id
                         AND running.root_id = asset_locations.root_id
                         AND running.status IN ('running', 'paused')
                     )
                   )",
                params![
                    identity.scheme,
                    identity.value,
                    file_size,
                    location.created_unix_ms,
                    location.modified_unix_ms,
                    i64::from(location.width),
                    i64::from(location.height),
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
                    source_revision_token,
                    source_generation,
                ],
            )
            .map_err(database_error)?;
    }
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
               file_identity_scheme, file_identity_value, source_revision_token,
               source_generation
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                strftime(
                  '%Y-%m-%dT%H:%M:%f', COALESCE(?9, ?10) / 1000.0,
                  'unixepoch', 'localtime'
                ),
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25, ?26, ?27
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
               file_identity_value = excluded.file_identity_value,
               source_revision_token = excluded.source_revision_token,
               source_generation = excluded.source_generation",
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
                source_revision_token,
                source_generation,
            ],
        )
        .map_err(database_error)?;
    Ok(PersistLocationOutcome {
        active_projection_changed,
        source_generation,
    })
}

fn allocate_source_generation(transaction: &Transaction<'_>) -> Result<i64, ScanError> {
    let generation = transaction
        .query_row(
            "SELECT next_source_generation FROM catalog_state",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    if generation <= 0 || generation == i64::MAX {
        return Err(ScanError::new(
            "catalog_source_generation_exhausted",
            "The catalog source-generation allocator is exhausted",
        ));
    }
    transaction
        .execute(
            "UPDATE catalog_state SET next_source_generation = ?1",
            [generation + 1],
        )
        .map_err(database_error)?;
    Ok(generation)
}

struct StoredAssetRow {
    asset_id: String,
    location_id: String,
    root_id: String,
    scan_id: String,
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
    source_revision_token: Option<String>,
    source_generation: Option<i64>,
}

fn read_stored_asset(row: &Row<'_>) -> rusqlite::Result<StoredAssetRow> {
    Ok(StoredAssetRow {
        asset_id: row.get(0)?,
        location_id: row.get(1)?,
        root_id: row.get(2)?,
        scan_id: row.get(3)?,
        absolute_path: row.get(4)?,
        relative_path: row.get(5)?,
        preview_path: row.get(6)?,
        file_size: row.get(7)?,
        created_unix_ms: row.get(8)?,
        modified_unix_ms: row.get(9)?,
        width: row.get(10)?,
        height: row.get(11)?,
        preview_status: row.get(12)?,
        preview_issue_code: row.get(13)?,
        preview_issue_message: row.get(14)?,
        metadata_engine_id: row.get(15)?,
        metadata_engine_version: row.get(16)?,
        capture_local_time: row.get(17)?,
        capture_offset_minutes: row.get(18)?,
        capture_time_source: row.get(19)?,
        capture_raw_value: row.get(20)?,
        file_identity_scheme: row.get(21)?,
        file_identity_value: row.get(22)?,
        source_revision_token: row.get(23)?,
        source_generation: row.get(24)?,
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
        scan_id: stored.scan_id,
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
        source_revision: stored_source_revision(stored.source_revision_token)?,
        source_generation: sqlite_unsigned(
            stored.source_generation.ok_or_else(|| {
                ScanError::new(
                    "catalog_source_generation_missing",
                    "The catalog location has no source generation",
                )
            })?,
            "source generation",
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

fn source_revision_token(evidence: &SourceRevisionEvidence) -> Result<String, ScanError> {
    if evidence.scheme != "windows-file-change-time-100ns-v1"
        || evidence.value.len() != 16
        || !evidence
            .value
            .bytes()
            .all(|value| value.is_ascii_hexdigit() && !value.is_ascii_uppercase())
    {
        return Err(ScanError::new(
            "catalog_source_revision_invalid",
            "Source revision evidence is not a canonical Windows ChangeTime token",
        ));
    }
    Ok(format!("{}:{}", evidence.scheme, evidence.value))
}

fn stored_source_revision(
    token: Option<String>,
) -> Result<Option<SourceRevisionEvidence>, ScanError> {
    let Some(token) = token else { return Ok(None) };
    let Some((scheme, value)) = token.rsplit_once(':') else {
        return Err(ScanError::new(
            "catalog_source_revision_invalid",
            "The stored source revision token is malformed",
        ));
    };
    let evidence = SourceRevisionEvidence {
        scheme: scheme.to_owned(),
        value: value.to_owned(),
    };
    source_revision_token(&evidence)?;
    Ok(Some(evidence))
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
    owner: ScanOwner,
) -> Result<Option<RecoverableScan>, ScanError> {
    let stored = connection
        .query_row(
            "SELECT scans.id, roots.path, scans.max_items, scans.max_entries,
                    scans.preview_edge, scans.visited_entries, scans.accepted_items,
                    scans.issue_count
             FROM scan_runs AS scans
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.status = ?1 AND scans.scan_owner = ?2
             ORDER BY scans.started_unix_ms DESC, scans.id DESC
             LIMIT 1",
            params![status, owner.as_str()],
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

#[cfg(test)]
fn load_scans_with_status(
    connection: &Connection,
    status: &str,
    owner: ScanOwner,
    after_scan_id: Option<&str>,
    limit: u32,
) -> Result<Vec<RecoverableScan>, ScanError> {
    if !(1..=64).contains(&limit) {
        return Err(ScanError::new(
            "catalog_recoverable_scan_limit_invalid",
            "Recoverable scan queries require a limit between 1 and 64",
        ));
    }
    let mut statement = connection
        .prepare(
            "SELECT scans.id, roots.path, scans.max_items, scans.max_entries,
                    scans.preview_edge, scans.visited_entries, scans.accepted_items,
                    scans.issue_count
             FROM scan_runs AS scans
             JOIN library_roots AS roots ON roots.id = scans.root_id
             WHERE scans.status = ?1 AND scans.scan_owner = ?2
               AND (?3 IS NULL OR scans.id > ?3)
             ORDER BY scans.id
             LIMIT ?4",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(
            params![status, owner.as_str(), after_scan_id, i64::from(limit)],
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
        .map_err(database_error)?;
    let mut recoverable = Vec::new();
    for row in rows {
        let (
            scan_id,
            root_path,
            max_items,
            max_entries,
            preview_edge,
            visited_entries,
            accepted_items,
            issue_count,
        ) = row.map_err(database_error)?;
        recoverable.push(RecoverableScan {
            scan_id,
            display_root_path: user_visible_path(&root_path),
            root_path,
            max_items: optional_sqlite_u32(max_items, "item limit")?,
            max_entries: optional_sqlite_u32(max_entries, "entry limit")?,
            preview_edge: sqlite_u32(preview_edge, "preview edge")?,
            visited_entries: sqlite_unsigned(visited_entries, "visited entry count")?,
            accepted_items: sqlite_unsigned(accepted_items, "accepted item count")?,
            issue_count: sqlite_unsigned(issue_count, "issue count")?,
        });
    }
    Ok(recoverable)
}

fn database_error(error: rusqlite::Error) -> ScanError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error {
        read_retry::record_database_error_code(failure.code);
        match failure.code {
            rusqlite::ErrorCode::OperationInterrupted => {
                return ScanError::new(
                    "catalog_database_interrupted",
                    "The catalog database operation was interrupted",
                );
            }
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
            rusqlite::ErrorCode::FileLockingProtocolFailed => {
                return ScanError::new(
                    "catalog_database_protocol",
                    format!(
                        "The catalog database read protocol could not acquire WAL state: {error}"
                    ),
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
             ) AND NOT EXISTS (
               SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
               WHERE handoffs.asset_id = assets.id
             ) AND NOT EXISTS (
               SELECT 1 FROM library_change_scan_handoff_items AS handoffs
               WHERE handoffs.asset_id = assets.id
             )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn detach_preview_references_for_root_locations(
    transaction: &Transaction<'_>,
    root_id: &str,
    retained_scan_id: Option<&str>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM asset_locations AS locations
               WHERE locations.root_id = ?1
                 AND (?2 IS NULL OR locations.scan_id <> ?2)
             )",
            params![root_id, retained_scan_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn prepare_root_unregister_scope(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS ame_root_unregister_assets (
               asset_id TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             CREATE TEMP TABLE IF NOT EXISTS ame_root_unregister_preview_artifacts (
               artifact_key TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             DELETE FROM temp.ame_root_unregister_assets;
             DELETE FROM temp.ame_root_unregister_preview_artifacts;",
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO temp.ame_root_unregister_assets(asset_id)
             SELECT asset_id
             FROM asset_locations
             WHERE root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO temp.ame_root_unregister_preview_artifacts(artifact_key)
             SELECT owners.artifact_key
             FROM preview_artifact_locations AS owners
             JOIN asset_locations AS locations
               ON locations.location_id = owners.location_id
             WHERE locations.root_id = ?1",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn mark_root_unregister_preview_artifacts_stale(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND artifact_key IN (
                 SELECT artifact_key
                 FROM temp.ame_root_unregister_preview_artifacts
               )
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn delete_root_unregister_orphan_assets(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute(
            "DELETE FROM assets
             WHERE id IN (
               SELECT asset_id FROM temp.ame_root_unregister_assets
             )
               AND NOT EXISTS (
                 SELECT 1 FROM asset_locations WHERE asset_locations.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.asset_id = assets.id
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn clear_root_unregister_scope(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "DELETE FROM temp.ame_root_unregister_assets;
             DELETE FROM temp.ame_root_unregister_preview_artifacts;",
        )
        .map_err(database_error)
}

fn mark_unreferenced_preview_artifacts_stale(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE preview_artifacts
             SET lifecycle_state = 'stale'
             WHERE lifecycle_state = 'ready'
               AND NOT EXISTS (
                 SELECT 1 FROM preview_artifact_locations AS owners
                 WHERE owners.artifact_key = preview_artifacts.artifact_key
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_catch_up_handoffs AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )
               AND NOT EXISTS (
                 SELECT 1 FROM library_change_scan_handoff_items AS handoffs
                 WHERE handoffs.preview_status = 'ready'
                   AND handoffs.preview_path = preview_artifacts.artifact_path
               )",
            [],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_scan_catch_up_lineage(
    transaction: &Transaction<'_>,
    scan_id: &str,
) -> Result<Vec<(String, String)>, ScanError> {
    let mut statement = transaction
        .prepare_cached(
            "SELECT catch_up_source, catch_up_watermark
             FROM scan_run_catch_up_lineage
             WHERE scan_id = ?1
             ORDER BY enrolled_unix_ms DESC, catch_up_source, catch_up_watermark
             LIMIT ?2",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![scan_id, MAX_SCAN_CATCH_UP_LINEAGE + 1], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(database_error)?;
    let mut lineage = Vec::new();
    for row in rows {
        lineage.push(row.map_err(database_error)?);
    }
    if i64::try_from(lineage.len()).unwrap_or(i64::MAX) > MAX_SCAN_CATCH_UP_LINEAGE {
        return Err(ScanError::new(
            "catalog_scan_catch_up_lineage_limit_exceeded",
            "The authoritative scan contains too many catch-up watermarks",
        ));
    }
    Ok(lineage)
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
