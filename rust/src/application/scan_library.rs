use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use blake3::Hasher;

use crate::adapters::{
    FileDiscovery, FileVisitOutcome, LocalMediaInspector, SqliteCatalog,
    is_current_preview_artifact, revalidate_file_state, user_visible_path,
};
use crate::domain::{
    AssetLocationView, DiscoveredFile, PreviewStatus, RecoverableScan, ScanError, ScanEvent,
    ScanIssue, ScanRequest,
};
use crate::ports::{CatalogRepository, MediaInspector};

use super::{StoragePaths, storage_paths};

static ACTIVE_SCANS: OnceLock<Mutex<HashMap<String, Arc<AtomicU8>>>> = OnceLock::new();
const CHECKPOINT_INTERVAL: u64 = 128;
const DIRECTORY_ENTRY_BATCH: usize = 256;
const DIRECTORY_ENTRY_WINDOW: u32 = 256;
const CONTROL_RUNNING: u8 = 0;
const CONTROL_PAUSE: u8 = 1;
const CONTROL_CANCEL: u8 = 2;

pub fn run_scan(
    request: ScanRequest,
    publish: impl FnMut(ScanEvent) -> bool,
) -> Result<(), ScanError> {
    let storage = storage_paths()?;
    run_scan_with_storage(request, publish, storage)
}

pub fn load_recoverable_scan() -> Result<Option<RecoverableScan>, ScanError> {
    let storage = storage_paths()?;
    SqliteCatalog::open(storage.catalog_path)?.load_recoverable_scan()
}

pub fn load_paused_scan() -> Result<Option<RecoverableScan>, ScanError> {
    let storage = storage_paths()?;
    SqliteCatalog::open(storage.catalog_path)?.load_paused_scan()
}

fn run_scan_with_storage(
    request: ScanRequest,
    mut publish: impl FnMut(ScanEvent) -> bool,
    storage: StoragePaths,
) -> Result<(), ScanError> {
    validate_request(&request)?;
    let media_inspector = LocalMediaInspector::new();
    let discovery = FileDiscovery::new(&request.root_path)?;
    let canonical_root = discovery.canonical_root()?;
    let root_path = canonical_root.to_string_lossy().into_owned();
    let root_id = stable_id("library-root-v1", &root_path);
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    let mut checkpoint = catalog.begin_scan(&request, &root_id, &root_path)?;
    let has_active_locations = catalog.has_active_locations()?;
    let control = register_scan(&request.scan_id)?;
    let _registration = ScanRegistration {
        scan_id: request.scan_id.clone(),
    };

    if !publish(ScanEvent::Started {
        scan_id: request.scan_id.clone(),
        root_path: user_visible_path(&root_path),
        item_limit: request.max_items,
        entry_limit: request.max_entries,
    }) {
        catalog.abandon_scan(&request.scan_id, "detached", 0)?;
        return Ok(());
    }

    if checkpoint.visited_entries > 0
        && !publish(ScanEvent::Progress {
            scan_id: request.scan_id.clone(),
            visited_entries: checkpoint.visited_entries,
            accepted_items: checkpoint.accepted_items,
            issue_count: checkpoint.issue_count,
        })
    {
        catalog.abandon_scan(&request.scan_id, "detached", checkpoint.issue_count)?;
        return Ok(());
    }

    let mut visited_entries = checkpoint.visited_entries;
    let mut accepted_items = checkpoint.accepted_items;
    let mut issue_count = checkpoint.issue_count;
    let mut was_limited = false;
    'traversal: loop {
        if finish_if_controlled(
            control.load(Ordering::Relaxed),
            &mut catalog,
            &request,
            &checkpoint,
            &mut publish,
        )? {
            return Ok(());
        }
        let Some(relative_directory) = catalog.claim_next_directory(&request.scan_id)? else {
            break;
        };
        if !catalog.is_current_directory_enumerated(&request.scan_id, &relative_directory)? {
            let entries = match discovery.entry_paths_in_directory(&relative_directory) {
                Ok(entries) => entries,
                Err(issue) => {
                    issue_count += 1;
                    catalog.record_issue(&request.scan_id, &issue)?;
                    checkpoint.last_visited_relative_path = None;
                    checkpoint.issue_count = issue_count;
                    if !publish(ScanEvent::Issue {
                        scan_id: request.scan_id.clone(),
                        issue: user_visible_issue(issue),
                    }) {
                        catalog.abandon_scan(&request.scan_id, "detached", issue_count)?;
                        return Ok(());
                    }
                    catalog.complete_directory(&request.scan_id, &checkpoint)?;
                    continue;
                }
            };
            let mut batch = Vec::with_capacity(DIRECTORY_ENTRY_BATCH);
            for relative_path in entries {
                batch.push(relative_path);
                if batch.len() == DIRECTORY_ENTRY_BATCH {
                    catalog.stage_directory_entries(
                        &request.scan_id,
                        &relative_directory,
                        &batch,
                    )?;
                    batch.clear();
                    if finish_if_controlled(
                        control.load(Ordering::Relaxed),
                        &mut catalog,
                        &request,
                        &checkpoint,
                        &mut publish,
                    )? {
                        return Ok(());
                    }
                }
            }
            catalog.stage_directory_entries(&request.scan_id, &relative_directory, &batch)?;
            catalog.complete_directory_enumeration(&request.scan_id, &relative_directory)?;
        }

        if let Some(saved_path) = checkpoint.last_visited_relative_path.as_deref()
            && !catalog.has_directory_entry(&request.scan_id, &relative_directory, saved_path)?
        {
            let issue = ScanIssue {
                path: checkpoint.last_visited_relative_path.clone(),
                code: "scan_checkpoint_unavailable".to_owned(),
                message: "The saved position no longer exists in the current directory".to_owned(),
            };
            issue_count += 1;
            catalog.record_issue(&request.scan_id, &issue)?;
            publish(ScanEvent::Issue {
                scan_id: request.scan_id.clone(),
                issue: user_visible_issue(issue),
            });
            catalog.abandon_scan(&request.scan_id, "stale", issue_count)?;
            publish(ScanEvent::Stale {
                scan_id: request.scan_id,
                accepted_items,
                issue_count,
            });
            return Ok(());
        }

        if request
            .max_entries
            .is_some_and(|limit| visited_entries >= u64::from(limit))
            || request
                .max_items
                .is_some_and(|limit| accepted_items >= u64::from(limit))
        {
            was_limited = true;
            break 'traversal;
        }

        loop {
            let relative_paths = catalog.load_directory_entry_window(
                &request.scan_id,
                &relative_directory,
                checkpoint.last_visited_relative_path.as_deref(),
                DIRECTORY_ENTRY_WINDOW,
            )?;
            if relative_paths.is_empty() {
                break;
            }

            for relative_path in relative_paths {
                if finish_if_controlled(
                    control.load(Ordering::Relaxed),
                    &mut catalog,
                    &request,
                    &checkpoint,
                    &mut publish,
                )? {
                    return Ok(());
                }

                let visit = discovery.visit_relative_path(&relative_path);

                visited_entries = visited_entries.checked_add(1).ok_or_else(|| {
                    ScanError::new(
                        "entry_count_overflow",
                        "The directory entry count exceeded the supported range",
                    )
                })?;
                if request
                    .max_entries
                    .is_some_and(|limit| visited_entries > u64::from(limit))
                {
                    was_limited = true;
                    break 'traversal;
                }

                let mut discovered_event = None;
                match visit.outcome {
                    FileVisitOutcome::Directory => {
                        catalog.enqueue_directory(&request.scan_id, &visit.relative_path)?;
                    }
                    FileVisitOutcome::Ignored => {}
                    FileVisitOutcome::Issue(issue) => {
                        issue_count += 1;
                        catalog.record_issue(&request.scan_id, &issue)?;
                        discovered_event = Some(ScanEvent::Issue {
                            scan_id: request.scan_id.clone(),
                            issue: user_visible_issue(issue),
                        });
                    }
                    FileVisitOutcome::File(file) => {
                        for issue in &file.issues {
                            issue_count += 1;
                            catalog.record_issue(&request.scan_id, issue)?;
                            if !publish(ScanEvent::Issue {
                                scan_id: request.scan_id.clone(),
                                issue: user_visible_issue(issue.clone()),
                            }) {
                                catalog.abandon_scan(&request.scan_id, "detached", issue_count)?;
                                return Ok(());
                            }
                        }
                        let location_id = stable_id(
                            "asset-location-v1",
                            &format!("{root_id}\0{}", file.relative_path),
                        );
                        let candidate_asset_id = file.file_identity.as_ref().map_or_else(
                            || {
                                stable_id(
                                    "asset-v1",
                                    &format!("{}\0{location_id}", request.scan_id),
                                )
                            },
                            |identity| {
                                stable_id(
                                    "asset-file-identity-v1",
                                    &format!(
                                        "{}\0{}\0{}",
                                        request.scan_id, identity.scheme, identity.value
                                    ),
                                )
                            },
                        );
                        let path_prior = has_active_locations
                            .then(|| catalog.load_active_location(&location_id))
                            .transpose()?
                            .flatten();
                        let identity_prior =
                            if file.file_identity.as_ref().is_some_and(|identity| {
                                path_prior
                                    .as_ref()
                                    .and_then(|prior| prior.file_identity.as_ref())
                                    == Some(identity)
                            }) {
                                path_prior.clone()
                            } else if has_active_locations {
                                file.file_identity
                                    .as_ref()
                                    .map(|identity| {
                                        catalog.load_active_location_by_file_identity(identity)
                                    })
                                    .transpose()?
                                    .flatten()
                            } else {
                                None
                            };
                        let path_is_unchanged = path_prior.as_ref().is_some_and(|prior| {
                            same_file_state(prior, &file)
                                && (file.file_identity.is_none()
                                    || prior.file_identity.is_none()
                                    || prior.file_identity == file.file_identity)
                        });
                        let asset_id = identity_prior
                            .as_ref()
                            .map(|prior| prior.asset_id.clone())
                            .or_else(|| {
                                path_is_unchanged
                                    .then(|| {
                                        path_prior.as_ref().map(|prior| prior.asset_id.clone())
                                    })
                                    .flatten()
                            })
                            .unwrap_or(candidate_asset_id);
                        let prior = identity_prior
                            .filter(|prior| same_file_state(prior, &file))
                            .or_else(|| path_is_unchanged.then_some(path_prior).flatten());
                        let compatible_metadata = prior.as_ref().filter(|prior| {
                            prior.metadata_engine_id == media_inspector.metadata_engine_id()
                                && prior.metadata_engine_version
                                    == media_inspector.metadata_engine_version()
                        });
                        let inspection = if let Some(prior) = compatible_metadata {
                            Ok(crate::domain::MediaInspection {
                                width: prior.width,
                                height: prior.height,
                                metadata: crate::domain::MetadataInspection {
                                    engine_id: prior.metadata_engine_id.clone(),
                                    engine_version: prior.metadata_engine_version.clone(),
                                    capture_time: prior.capture_time.clone(),
                                    issues: Vec::new(),
                                },
                            })
                        } else {
                            media_inspector.inspect(&file)
                        };
                        match inspection {
                            Ok(inspection) => {
                                for issue in inspection.metadata.issues {
                                    issue_count += 1;
                                    catalog.record_issue(&request.scan_id, &issue)?;
                                    if !publish(ScanEvent::Issue {
                                        scan_id: request.scan_id.clone(),
                                        issue: user_visible_issue(issue),
                                    }) {
                                        catalog.abandon_scan(
                                            &request.scan_id,
                                            "detached",
                                            issue_count,
                                        )?;
                                        return Ok(());
                                    }
                                }
                                let (preview_path, preview_status) = prior
                                    .as_ref()
                                    .filter(|prior| {
                                        compatible_metadata.is_some()
                                            && matches!(prior.preview_status, PreviewStatus::Ready)
                                            && !prior.preview_path.is_empty()
                                            && Path::new(&prior.preview_path).is_file()
                                            && is_current_preview_artifact(&prior.preview_path)
                                    })
                                    .map(|prior| (prior.preview_path.clone(), PreviewStatus::Ready))
                                    .unwrap_or_else(|| (String::new(), PreviewStatus::Pending));
                                let asset = AssetLocationView {
                                    asset_id,
                                    location_id,
                                    root_id: root_id.clone(),
                                    display_path: user_visible_path(&file.absolute_path),
                                    absolute_path: file.absolute_path,
                                    relative_path: file.relative_path,
                                    preview_path,
                                    file_size: file.file_size,
                                    created_unix_ms: file.created_unix_ms,
                                    modified_unix_ms: file.modified_unix_ms,
                                    file_identity: file.file_identity,
                                    width: inspection.width,
                                    height: inspection.height,
                                    preview_status,
                                    preview_issue_code: None,
                                    preview_issue_message: None,
                                    metadata_engine_id: inspection.metadata.engine_id,
                                    metadata_engine_version: inspection.metadata.engine_version,
                                    capture_time: inspection.metadata.capture_time,
                                };
                                catalog.stage_location(&request.scan_id, &root_id, &asset)?;
                                accepted_items += 1;
                                discovered_event = Some(ScanEvent::AssetDiscovered {
                                    scan_id: request.scan_id.clone(),
                                    asset: Box::new(asset),
                                });
                            }
                            Err(issue) => {
                                issue_count += 1;
                                catalog.record_issue(&request.scan_id, &issue)?;
                                discovered_event = Some(ScanEvent::Issue {
                                    scan_id: request.scan_id.clone(),
                                    issue: user_visible_issue(issue),
                                });
                            }
                        }
                    }
                }

                checkpoint.last_visited_relative_path = Some(visit.relative_path);
                checkpoint.visited_entries = visited_entries;
                checkpoint.accepted_items = accepted_items;
                checkpoint.issue_count = issue_count;
                if visited_entries.is_multiple_of(CHECKPOINT_INTERVAL) {
                    catalog.checkpoint_scan(&request.scan_id, &checkpoint)?;
                }

                let did_accept_asset =
                    matches!(&discovered_event, Some(ScanEvent::AssetDiscovered { .. }));
                if discovered_event.is_some_and(|event| !publish(event)) {
                    catalog.abandon_scan(&request.scan_id, "detached", issue_count)?;
                    return Ok(());
                }
                let should_publish_progress = visited_entries == 1
                    || visited_entries.is_multiple_of(CHECKPOINT_INTERVAL)
                    || did_accept_asset && accepted_items > 0 && accepted_items.is_multiple_of(25);
                if should_publish_progress
                    && !publish(ScanEvent::Progress {
                        scan_id: request.scan_id.clone(),
                        visited_entries,
                        accepted_items,
                        issue_count,
                    })
                {
                    catalog.abandon_scan(&request.scan_id, "detached", issue_count)?;
                    return Ok(());
                }
                if request
                    .max_items
                    .is_some_and(|limit| accepted_items >= u64::from(limit))
                {
                    was_limited = true;
                    break 'traversal;
                }
            }
        }

        checkpoint.last_visited_relative_path = None;
        catalog.complete_directory(&request.scan_id, &checkpoint)?;
    }

    catalog.checkpoint_scan(&request.scan_id, &checkpoint)?;

    if finish_if_controlled(
        control.load(Ordering::Relaxed),
        &mut catalog,
        &request,
        &checkpoint,
        &mut publish,
    )? {
        return Ok(());
    }

    for expected in catalog.staged_file_states(&request.scan_id)? {
        if finish_if_controlled(
            control.load(Ordering::Relaxed),
            &mut catalog,
            &request,
            &checkpoint,
            &mut publish,
        )? {
            return Ok(());
        }
        if let Err(issue) = revalidate_file_state(&expected) {
            issue_count += 1;
            catalog.record_issue(&request.scan_id, &issue)?;
            if !publish(ScanEvent::Issue {
                scan_id: request.scan_id.clone(),
                issue: user_visible_issue(issue),
            }) {
                catalog.abandon_scan(&request.scan_id, "detached", issue_count)?;
                return Ok(());
            }
            catalog.abandon_scan(&request.scan_id, "stale", issue_count)?;
            publish(ScanEvent::Stale {
                scan_id: request.scan_id,
                accepted_items,
                issue_count,
            });
            return Ok(());
        }
    }

    if finish_if_controlled(
        control.load(Ordering::Relaxed),
        &mut catalog,
        &request,
        &checkpoint,
        &mut publish,
    )? {
        return Ok(());
    }

    catalog.publish_scan(&request.scan_id, &root_id, accepted_items, issue_count)?;
    publish(ScanEvent::Completed {
        scan_id: request.scan_id,
        root_id,
        asset_count: accepted_items,
        issue_count,
        catalog_path: catalog.catalog_path().to_string_lossy().into_owned(),
        was_limited,
    });
    Ok(())
}

fn user_visible_issue(mut issue: ScanIssue) -> ScanIssue {
    issue.path = issue.path.as_deref().map(user_visible_path);
    issue
}

pub fn cancel_scan(scan_id: &str) -> bool {
    let Ok(scans) = active_scans().lock() else {
        return false;
    };
    let Some(token) = scans.get(scan_id) else {
        return false;
    };
    token.store(CONTROL_CANCEL, Ordering::Relaxed);
    true
}

pub fn pause_scan(scan_id: &str) -> bool {
    let Ok(scans) = active_scans().lock() else {
        return false;
    };
    let Some(token) = scans.get(scan_id) else {
        return false;
    };
    token.store(CONTROL_PAUSE, Ordering::Relaxed);
    true
}

fn finish_if_controlled(
    control: u8,
    catalog: &mut SqliteCatalog,
    request: &ScanRequest,
    checkpoint: &crate::domain::ScanCheckpoint,
    publish: &mut impl FnMut(ScanEvent) -> bool,
) -> Result<bool, ScanError> {
    match control {
        CONTROL_PAUSE => {
            catalog.pause_scan(&request.scan_id, checkpoint)?;
            publish(ScanEvent::Paused {
                scan_id: request.scan_id.clone(),
                visited_entries: checkpoint.visited_entries,
                accepted_items: checkpoint.accepted_items,
                issue_count: checkpoint.issue_count,
            });
            Ok(true)
        }
        CONTROL_CANCEL => {
            catalog.checkpoint_scan(&request.scan_id, checkpoint)?;
            catalog.abandon_scan(&request.scan_id, "cancelled", checkpoint.issue_count)?;
            publish(ScanEvent::Cancelled {
                scan_id: request.scan_id.clone(),
                accepted_items: checkpoint.accepted_items,
                issue_count: checkpoint.issue_count,
            });
            Ok(true)
        }
        _ => Ok(false),
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

fn active_scans() -> &'static Mutex<HashMap<String, Arc<AtomicU8>>> {
    ACTIVE_SCANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_scan(scan_id: &str) -> Result<Arc<AtomicU8>, ScanError> {
    let mut scans = active_scans()
        .lock()
        .map_err(|_| ScanError::new("scan_registry_unavailable", "Scan registry is poisoned"))?;
    if scans.contains_key(scan_id) {
        return Err(ScanError::new(
            "scan_already_active",
            "A scan with this identifier is already active",
        ));
    }
    let token = Arc::new(AtomicU8::new(CONTROL_RUNNING));
    scans.insert(scan_id.to_owned(), Arc::clone(&token));
    Ok(token)
}

struct ScanRegistration {
    scan_id: String,
}

impl Drop for ScanRegistration {
    fn drop(&mut self) {
        if let Ok(mut scans) = active_scans().lock() {
            scans.remove(&self.scan_id);
        }
    }
}

fn stable_id(namespace: &str, value: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(namespace.as_bytes());
    hasher.update(&[0]);
    hasher.update(value.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn same_file_state(prior: &AssetLocationView, file: &DiscoveredFile) -> bool {
    prior.file_size == file.file_size && prior.modified_unix_ms == file.modified_unix_ms
}

#[cfg(test)]
mod tests;
