use std::path::Path;
use std::sync::atomic::Ordering;
#[cfg(not(test))]
use std::thread;
#[cfg(not(test))]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use blake3::Hasher;

use crate::adapters::{FileDiscovery, LocalMediaInspector, SqliteCatalog, user_visible_path};
#[cfg(test)]
use crate::domain::{AssetLocationView, PreviewStatus};
use crate::domain::{
    LibraryChangeLane, RecoverableScan, ScanCheckpoint, ScanError, ScanEvent, ScanIssue,
    ScanRequest,
};
#[cfg(test)]
use crate::domain::{
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalContinuityState, PersistentJournalFailure,
};
#[cfg(test)]
use crate::ports::PersistentJournalRepository;
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::storage::catalog_admission::with_scan_start;
use super::{StoragePaths, storage_paths};

macro_rules! measure_scan_operation {
    ($operation:ident, $call:expr) => {{
        #[cfg(test)]
        let _measurement = $crate::application::scan_library::operation_cost::OperationTimer::start(
            $crate::application::scan_library::operation_cost::Operation::$operation,
        );
        $call
    }};
}

#[cfg(test)]
mod admission_tests;
mod entry_processing;
mod execution_registry;
mod file_preparation;
mod finalization;
mod inspection_failure;
#[cfg(all(test, windows))]
mod media_input_tests;
#[cfg(test)]
mod operation_cost;
mod publication;
#[cfg(test)]
mod resumption_tests;
mod retained_cancellation;
mod traversal;

#[cfg(test)]
pub(crate) use execution_registry::hold_first_import_capture;

use execution_registry::{
    CONTROL_CANCEL, CONTROL_PAUSE, CONTROL_SUSPEND, ScanRegistration, bind_first_import_capture,
    bind_scan_catalog, register_scan,
};
pub(super) use execution_registry::{
    FirstImportCaptureLease, catalog_has_active_scan, first_import_capture_lease,
    protect_scan_catalog_session,
};
pub use execution_registry::{cancel_scan, pause_scan, suspend_scan};
pub use retained_cancellation::cancel_retained_scan;

use finalization::{FinalizationContext, FinalizationMode, FinalizationPlan};
use publication::{
    ForegroundPublicationContext, ForegroundPublicationOutcome, publish_foreground_scan,
};
use traversal::{ScanTraversalContext, ScanTraversalOutcome, traverse_scan};

const CHECKPOINT_INTERVAL: u64 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FullScanReason {
    ExplicitUserRequest,
    ResumeForegroundCheckpoint,
    #[cfg(test)]
    ResumeAuthoritativeCheckpoint,
}

pub fn run_scan(
    request: ScanRequest,
    publish: impl FnMut(ScanEvent) -> bool,
) -> Result<(), ScanError> {
    run_scan_with_storage_reason(
        request,
        publish,
        storage_paths,
        FullScanReason::ExplicitUserRequest,
    )
}

pub fn resume_scan(
    request: ScanRequest,
    publish: impl FnMut(ScanEvent) -> bool,
) -> Result<(), ScanError> {
    run_scan_with_storage_reason(
        request,
        publish,
        storage_paths,
        FullScanReason::ResumeForegroundCheckpoint,
    )
}

pub fn load_recoverable_scan() -> Result<Option<RecoverableScan>, ScanError> {
    let storage = storage_paths()?;
    load_recoverable_scan_from_path(&storage.catalog_path)
}

pub fn load_paused_scan() -> Result<Option<RecoverableScan>, ScanError> {
    let storage = storage_paths()?;
    load_paused_scan_from_path(&storage.catalog_path)
}

fn load_recoverable_scan_from_path(path: &Path) -> Result<Option<RecoverableScan>, ScanError> {
    super::catalog_session::with_recovery_catalog_if_idle(path, |catalog| {
        catalog.load_single_recoverable_foreground_scan()
    })
    .map(Option::flatten)
}

fn load_paused_scan_from_path(path: &Path) -> Result<Option<RecoverableScan>, ScanError> {
    super::catalog_session::with_recovery_catalog_if_idle(path, |catalog| {
        catalog.load_single_paused_foreground_scan()
    })
    .map(Option::flatten)
}

#[cfg(test)]
pub(super) fn run_scan_with_storage(
    request: ScanRequest,
    publish: impl FnMut(ScanEvent) -> bool,
    storage: StoragePaths,
) -> Result<(), ScanError> {
    run_scan_with_storage_reason(
        request,
        publish,
        || Ok(storage),
        FullScanReason::ExplicitUserRequest,
    )
}

#[cfg(test)]
pub(super) fn resume_scan_with_storage(
    request: ScanRequest,
    publish: impl FnMut(ScanEvent) -> bool,
    storage: StoragePaths,
) -> Result<(), ScanError> {
    run_scan_with_storage_reason(
        request,
        publish,
        || Ok(storage),
        FullScanReason::ResumeForegroundCheckpoint,
    )
}

fn run_scan_with_storage_reason(
    request: ScanRequest,
    mut publish: impl FnMut(ScanEvent) -> bool,
    resolve_storage: impl FnOnce() -> Result<StoragePaths, ScanError>,
    reason: FullScanReason,
) -> Result<(), ScanError> {
    validate_request(&request)?;
    let control = register_scan(&request.scan_id)?;
    let _registration = ScanRegistration {
        scan_id: request.scan_id.clone(),
    };
    let storage = resolve_storage()?;
    bind_scan_catalog(&request.scan_id, &storage.catalog_path)?;
    let media_inspector = LocalMediaInspector::new();
    let discovery = FileDiscovery::new(&request.root_path)?;
    let canonical_root = discovery.canonical_root()?;
    let publication_root_identity =
        discovery
            .metadata_inventory_root_identity()?
            .ok_or_else(|| {
                ScanError::new(
                    "root_publication_namespace_unavailable",
                    "The configured root lacks full Windows identity evidence",
                )
            })?;
    let root_path = canonical_root.to_string_lossy().into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    #[cfg(test)]
    let is_authoritative_recovery = reason == FullScanReason::ResumeAuthoritativeCheckpoint;
    #[cfg(not(test))]
    let is_authoritative_recovery = false;
    let (mut catalog, mut checkpoint, had_published_root) =
        with_scan_start(&storage, &canonical_root, || {
            let mut catalog = super::catalog_session::open_catalog_for_foreground_scan(
                &storage.catalog_path,
                LibraryChangeLane::Recovery,
                &request.scan_id,
            )?;
            let had_published_root = catalog
                .load_incremental_catalog_root(&root_id)?
                .is_some_and(|root| root.active_scan_id.is_some());
            let checkpoint = match reason {
                FullScanReason::ExplicitUserRequest => catalog
                    .begin_scan_with_publication_namespace(
                        &request,
                        &root_id,
                        &root_path,
                        &publication_root_identity,
                    )?,
                FullScanReason::ResumeForegroundCheckpoint => catalog
                    .resume_scan_with_publication_namespace(
                        &request,
                        &root_id,
                        &root_path,
                        &publication_root_identity,
                    )?,
                #[cfg(test)]
                FullScanReason::ResumeAuthoritativeCheckpoint => {
                    catalog.resume_authoritative_scan(&request, &root_id, &root_path)?
                }
            };
            Ok((catalog, checkpoint, had_published_root))
        })?;
    let mut issue_count = checkpoint.issue_count;
    let result = (|| -> Result<(), ScanError> {
        let root_generation = catalog
            .load_incremental_catalog_root(&root_id)?
            .map(|root| root.root_generation)
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_first_import_root_missing",
                    "The scan root was not registered before change-capture admission",
                )
            })?;
        if !had_published_root {
            bind_first_import_capture(
                &request.scan_id,
                &storage.catalog_path,
                &root_id,
                root_generation,
            )?;
        }
        if finish_if_controlled(
            control.load(Ordering::Relaxed),
            &mut catalog,
            &request,
            &checkpoint,
            issue_count,
            &mut publish,
            had_published_root,
        )? {
            return Ok(());
        }
        let finalization_mode = if is_authoritative_recovery {
            FinalizationMode::AuthoritativeRecovery
        } else {
            FinalizationMode::Foreground
        };
        let mut finalization = FinalizationPlan::new(finalization_mode);
        if is_authoritative_recovery {
            let evidence_allows_publication = finalization.restore_retained_scan_issues(
                &mut catalog,
                &request.scan_id,
                &canonical_root,
                Path::new(&request.root_path),
            )?;
            if checkpoint.requires_previous_snapshot && evidence_allows_publication {
                checkpoint.requires_previous_snapshot = false;
                catalog.checkpoint_scan(&request.scan_id, &checkpoint)?;
            }
        } else if reason == FullScanReason::ResumeForegroundCheckpoint {
            finalization.restore_retained_scan_issues(
                &mut catalog,
                &request.scan_id,
                &canonical_root,
                Path::new(&request.root_path),
            )?;
        }
        let has_active_locations = catalog.has_active_locations()?;

        if !publish(ScanEvent::Started {
            scan_id: request.scan_id.clone(),
            root_path: user_visible_path(&root_path),
            item_limit: request.max_items,
            entry_limit: request.max_entries,
        }) {
            retain_detached_scan(
                &mut catalog,
                control.load(Ordering::Relaxed),
                &request,
                &checkpoint,
                0,
                had_published_root,
            )?;
            return Ok(());
        }

        if (checkpoint.visited_entries > 0 || reason == FullScanReason::ResumeForegroundCheckpoint)
            && !publish(ScanEvent::Progress {
                scan_id: request.scan_id.clone(),
                visited_entries: checkpoint.visited_entries,
                accepted_items: checkpoint.accepted_items,
                issue_count: checkpoint.issue_count,
            })
        {
            retain_detached_scan(
                &mut catalog,
                control.load(Ordering::Relaxed),
                &request,
                &checkpoint,
                checkpoint.issue_count,
                had_published_root,
            )?;
            return Ok(());
        }

        if !had_published_root && !is_authoritative_recovery {
            #[cfg(test)]
            if !catalog.first_import_change_capture_is_ready(
                &request.scan_id,
                &root_id,
                root_generation,
            )? {
                catalog.save_persistent_journal_capability(&PersistentJournalCapability {
                    root_id: root_id.clone(),
                    root_generation,
                    protocol_version: crate::journal_broker::PROTOCOL_VERSION,
                    contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
                    state: PersistentJournalCapabilityState::LiveOnly,
                    continuity: PersistentJournalContinuityState::LiveOnly,
                    failure: Some(PersistentJournalFailure {
                        code: "test_live_observer_only".to_owned(),
                        message: "The fixed scan fixture uses a trusted live observer seam"
                            .to_owned(),
                    }),
                    updated_unix_ms: current_unix_ms()?,
                })?;
            }
            #[cfg(not(test))]
            {
                const CHANGE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(15);
                const CHANGE_CAPTURE_MAX_RETRY_DELAY: Duration = Duration::from_millis(250);
                let deadline = Instant::now()
                    .checked_add(CHANGE_CAPTURE_TIMEOUT)
                    .ok_or_else(|| {
                        ScanError::new(
                            "first_import_change_capture_deadline_invalid",
                            "The first-import change-capture deadline is invalid",
                        )
                    })?;
                let mut retry_delay = Duration::from_millis(25);
                loop {
                    if finish_if_controlled(
                        control.load(Ordering::Relaxed),
                        &mut catalog,
                        &request,
                        &checkpoint,
                        issue_count,
                        &mut publish,
                        had_published_root,
                    )? {
                        return Ok(());
                    }
                    if super::library_synchronization::poll_production_first_import_change_capture(
                        &request.scan_id,
                        &root_id,
                        root_generation,
                        &storage.catalog_path,
                    )? {
                        break;
                    }
                    let now = Instant::now();
                    if now >= deadline {
                        return Err(ScanError::new(
                            "first_import_continuity_preflight_timeout",
                            "The first import did not establish a healthy durable change boundary before enumeration",
                        ));
                    }
                    thread::sleep(retry_delay.min(deadline.saturating_duration_since(now)));
                    retry_delay = retry_delay
                        .saturating_mul(2)
                        .min(CHANGE_CAPTURE_MAX_RETRY_DELAY);
                }
            }
        }

        let ScanTraversalOutcome::Traversed {
            visited_entries,
            mut accepted_items,
            was_limited,
        } = traverse_scan(
            &mut catalog,
            &mut finalization,
            ScanTraversalContext {
                request: &request,
                control: &control,
                discovery: &discovery,
                inspector: &media_inspector,
                root_id: &root_id,
                had_published_root,
                has_active_locations,
            },
            &mut checkpoint,
            &mut issue_count,
            &mut publish,
        )?
        else {
            return Ok(());
        };

        if checkpoint.requires_previous_snapshot {
            catalog.abandon_scan(&request.scan_id, "stale", issue_count)?;
            publish(ScanEvent::Stale {
                scan_id: request.scan_id.clone(),
                accepted_items,
                issue_count,
            });
            return Ok(());
        }

        if was_limited && had_published_root {
            catalog.abandon_scan(&request.scan_id, "stale", issue_count)?;
            publish(ScanEvent::Stale {
                scan_id: request.scan_id.clone(),
                accepted_items,
                issue_count,
            });
            return Ok(());
        }

        if finish_if_controlled(
            control.load(Ordering::Relaxed),
            &mut catalog,
            &request,
            &checkpoint,
            issue_count,
            &mut publish,
            had_published_root,
        )? {
            return Ok(());
        }

        let Some(validated) = finalization::validate_staged_locations(
            &mut catalog,
            &mut finalization,
            FinalizationContext {
                control: &control,
                request: &request,
                checkpoint: &checkpoint,
                root_id: &root_id,
                root_path: &root_path,
                publication_root_identity: &publication_root_identity,
                had_published_root,
                visited_entries,
                accepted_items,
            },
            &mut issue_count,
            &mut publish,
        )?
        else {
            return Ok(());
        };

        if is_authoritative_recovery {
            validated
                .publication_guard
                .require_metadata_inventory_root_identity(&publication_root_identity)?;
            let retry_paths = finalization.authoritative_retry_paths();
            accepted_items = catalog.preserve_authoritative_retry_evidence(
                &request.scan_id,
                &root_id,
                &retry_paths,
            )?;
            if accepted_items != validated.total_items
                && !publish(ScanEvent::Finalizing {
                    scan_id: request.scan_id.clone(),
                    validated_items: accepted_items,
                    total_items: accepted_items,
                    visited_entries,
                    accepted_items,
                    issue_count,
                })
            {
                retain_detached_scan(
                    &mut catalog,
                    control.load(Ordering::Relaxed),
                    &request,
                    &checkpoint,
                    issue_count,
                    had_published_root,
                )?;
                return Ok(());
            }
            catalog.publish_authoritative_scan(
                &request.scan_id,
                &root_id,
                accepted_items,
                issue_count,
                &retry_paths,
            )?;
        } else {
            validated
                .publication_guard
                .require_metadata_inventory_root_identity(&publication_root_identity)?;
            let outcome = publish_foreground_scan(
                &mut catalog,
                &finalization,
                ForegroundPublicationContext {
                    control: &control,
                    request: &request,
                    checkpoint: &checkpoint,
                    root_id: &root_id,
                    root_generation,
                    accepted_items,
                    validated_items: validated.validated_items,
                    total_items: validated.total_items,
                    visited_entries,
                    issue_count,
                    had_published_root,
                    validation_proof: validated.foreground_proof.as_ref().ok_or_else(|| {
                        ScanError::new(
                            "catalog_scan_validation_proof_missing",
                            "Foreground publication requires a complete validation roster",
                        )
                    })?,
                },
                &mut publish,
            )?;
            match outcome {
                ForegroundPublicationOutcome::Interrupted => return Ok(()),
                ForegroundPublicationOutcome::Published { asset_count } => {
                    accepted_items = asset_count
                }
            }
        }
        publish(ScanEvent::Completed {
            scan_id: request.scan_id.clone(),
            root_id: root_id.clone(),
            asset_count: accepted_items,
            issue_count,
            catalog_path: catalog.catalog_path().to_string_lossy().into_owned(),
            was_limited,
        });
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(primary) => Err(abandon_failed_scan(
            &mut catalog,
            &request.scan_id,
            issue_count,
            primary,
        )),
    }
}

fn abandon_failed_scan(
    catalog: &mut impl CatalogRepository,
    scan_id: &str,
    issue_count: u64,
    primary: ScanError,
) -> ScanError {
    match catalog.abandon_scan(scan_id, "failed", issue_count) {
        Ok(()) => primary,
        Err(cleanup) => scan_cleanup_failure(&primary, &cleanup),
    }
}

fn scan_cleanup_failure(primary: &ScanError, cleanup: &ScanError) -> ScanError {
    let primary_code = bounded_error_text(&primary.code, 256);
    let cleanup_code = bounded_error_text(&cleanup.code, 256);
    let primary_message = bounded_error_text(&primary.message, 1_600);
    let cleanup_message = bounded_error_text(&cleanup.message, 1_600);
    ScanError::new(
        "scan_failure_cleanup_failed",
        bounded_error_text(
            &format!(
                "Scan failed [{primary_code}]; cleanup failed [{cleanup_code}]; primary: \
                 {primary_message}; cleanup: {cleanup_message}",
            ),
            4_096,
        ),
    )
}

fn bounded_error_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn user_visible_issue(mut issue: ScanIssue) -> ScanIssue {
    issue.path = issue.path.as_deref().map(user_visible_path);
    issue
}

fn current_unix_ms() -> Result<i64, ScanError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is earlier than the Unix epoch",
        )
    })?;
    i64::try_from(elapsed.as_millis()).map_err(|_| {
        ScanError::new(
            "system_clock_invalid",
            "The system clock is outside the supported range",
        )
    })
}

fn finish_if_controlled(
    control: u8,
    catalog: &mut SqliteCatalog,
    request: &ScanRequest,
    checkpoint: &crate::domain::ScanCheckpoint,
    issue_count: u64,
    publish: &mut impl FnMut(ScanEvent) -> bool,
    had_published_root: bool,
) -> Result<bool, ScanError> {
    if !matches!(control, CONTROL_PAUSE | CONTROL_CANCEL | CONTROL_SUSPEND) {
        return Ok(false);
    }
    let mut current_checkpoint = checkpoint.clone();
    current_checkpoint.issue_count = issue_count;
    let checkpoint = &current_checkpoint;
    match control {
        CONTROL_PAUSE => {
            if had_published_root {
                catalog.abandon_scan(&request.scan_id, "cancelled", issue_count)?;
                publish(ScanEvent::Cancelled {
                    scan_id: request.scan_id.clone(),
                    accepted_items: checkpoint.accepted_items,
                    issue_count,
                });
            } else {
                catalog.pause_scan(&request.scan_id, checkpoint)?;
                publish(ScanEvent::Paused {
                    scan_id: request.scan_id.clone(),
                    visited_entries: checkpoint.visited_entries,
                    accepted_items: checkpoint.accepted_items,
                    issue_count,
                });
            }
            Ok(true)
        }
        CONTROL_CANCEL => {
            catalog.checkpoint_scan(&request.scan_id, checkpoint)?;
            catalog.abandon_scan(&request.scan_id, "cancelled", issue_count)?;
            publish(ScanEvent::Cancelled {
                scan_id: request.scan_id.clone(),
                accepted_items: checkpoint.accepted_items,
                issue_count,
            });
            Ok(true)
        }
        CONTROL_SUSPEND => {
            if had_published_root {
                catalog.abandon_scan(&request.scan_id, "cancelled", issue_count)?;
            } else {
                catalog.checkpoint_scan(&request.scan_id, checkpoint)?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn retain_detached_scan(
    catalog: &mut impl CatalogRepository,
    control: u8,
    request: &ScanRequest,
    checkpoint: &ScanCheckpoint,
    issue_count: u64,
    had_published_root: bool,
) -> Result<(), ScanError> {
    if had_published_root {
        return catalog.abandon_scan(&request.scan_id, "cancelled", issue_count);
    }
    let mut retained = checkpoint.clone();
    retained.issue_count = issue_count;
    match control {
        CONTROL_CANCEL => {
            catalog.checkpoint_scan(&request.scan_id, &retained)?;
            catalog.abandon_scan(&request.scan_id, "cancelled", issue_count)
        }
        CONTROL_PAUSE => catalog.pause_scan(&request.scan_id, &retained),
        _ => catalog.checkpoint_scan(&request.scan_id, &retained),
    }
}

fn validate_request(request: &ScanRequest) -> Result<(), ScanError> {
    if request.scan_id.trim().is_empty() {
        return Err(ScanError::new(
            "scan_id_empty",
            "The scan identifier is required",
        ));
    }
    if request.preview_edge < 96 || request.preview_edge > 1024 {
        return Err(ScanError::new(
            "preview_edge_invalid",
            "Preview edge must be between 96 and 1024 pixels",
        ));
    }
    if request.max_items == Some(0) {
        return Err(ScanError::new(
            "item_limit_invalid",
            "The item limit must be greater than zero when supplied",
        ));
    }
    if request.max_entries == Some(0) {
        return Err(ScanError::new(
            "entry_limit_invalid",
            "The directory-entry limit must be greater than zero when supplied",
        ));
    }
    Ok(())
}

pub(super) fn stable_id(namespace: &str, value: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(namespace.as_bytes());
    hasher.update(&[0]);
    hasher.update(value.as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub(super) fn stable_location_id(root_id: &str, relative_path: &str) -> String {
    stable_id("asset-location-v1", &format!("{root_id}\0{relative_path}"))
}

#[cfg(test)]
mod tests;
