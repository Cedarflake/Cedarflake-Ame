use std::collections::HashMap;

use rusqlite::{Transaction, params};

use crate::domain::{AssetLocationView, FileIdentityEvidence, ScanError};

use super::identity_group_state::{
    StoredIdentityGroupState, load_active_identity_group_state, load_staging_identity_group_state,
};
use super::{capture_time_source_text, database_error, source_revision_token, sqlite_integer};

#[derive(Clone)]
pub(super) enum IdentityGenerationExpectation {
    Captured(Option<StoredIdentityGroupState>),
    MirrorCurrentActive,
    ResolveCurrent,
}

pub(super) struct IdentityGenerationBatchState {
    baseline_state: Option<StoredIdentityGroupState>,
    assigned_generation: i64,
    source_revision_token: Option<String>,
    file_size: i64,
    modified_unix_ms: i64,
}

pub(super) struct ReconciledLocationSource {
    pub(super) file_size: i64,
    pub(super) source_revision_token: Option<String>,
    pub(super) source_generation: i64,
    pub(super) active_projection_changed: bool,
}

enum AliasPropagation<'a> {
    None,
    StagedScan(&'a str),
    CurrentLocations,
}

pub(super) fn revisions_conflict(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

fn source_observation_superseded() -> ScanError {
    ScanError::new(
        "catalog_source_observation_superseded",
        "A newer observation already owns this physical file identity",
    )
}

pub(super) fn reconcile_location_source(
    transaction: &Transaction<'_>,
    scan_id: &str,
    location: &AssetLocationView,
    identity_generation_expectation: IdentityGenerationExpectation,
    identity_generation_batch: &mut HashMap<(String, String), IdentityGenerationBatchState>,
) -> Result<ReconciledLocationSource, ScanError> {
    location.camera.validate()?;
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
    let mut propagation = AliasPropagation::None;
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
                    propagation = match (is_scan_staging, group_state.is_some()) {
                        (true, true) => AliasPropagation::StagedScan(scan_id),
                        (true, false) => AliasPropagation::None,
                        (false, _) => AliasPropagation::CurrentLocations,
                    };
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
    let active_projection_changed = match location.file_identity.as_ref() {
        Some(identity) => propagate_observation(
            transaction,
            identity,
            location,
            file_size,
            source_revision_token.as_deref(),
            source_generation,
            propagation,
        )?,
        None => false,
    };
    Ok(ReconciledLocationSource {
        file_size,
        source_revision_token,
        active_projection_changed,
        source_generation,
    })
}

fn propagate_observation(
    transaction: &Transaction<'_>,
    identity: &FileIdentityEvidence,
    location: &AssetLocationView,
    file_size: i64,
    source_revision_token: Option<&str>,
    source_generation: i64,
    propagation: AliasPropagation<'_>,
) -> Result<bool, ScanError> {
    let (staged_scan_id, active_projection_changed) = match propagation {
        AliasPropagation::None => return Ok(false),
        AliasPropagation::StagedScan(scan_id) => (Some(scan_id), false),
        AliasPropagation::CurrentLocations => {
            super::gallery::retained_membership::rebind_unchanged_display(
                transaction,
                location,
                source_generation,
            )?;
            let active_projection_changed = transaction
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
            (None, active_projection_changed)
        }
    };
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
                     preview_issue_code = NULL, preview_issue_message = NULL,
                     camera_make = ?17, camera_model = ?18
                 WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
                   AND (?16 IS NULL OR scan_id = ?16)
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
                staged_scan_id,
                location.camera.make,
                location.camera.model,
            ],
        )
        .map_err(database_error)?;
    Ok(active_projection_changed)
}

pub(super) fn allocate_source_generation(transaction: &Transaction<'_>) -> Result<i64, ScanError> {
    let generation = transaction
        .prepare_cached("SELECT next_source_generation FROM catalog_state")
        .map_err(database_error)?
        .query_row([], |row| row.get::<_, i64>(0))
        .map_err(database_error)?;
    if generation <= 0 || generation == i64::MAX {
        return Err(ScanError::new(
            "catalog_source_generation_exhausted",
            "The catalog source-generation allocator is exhausted",
        ));
    }
    transaction
        .prepare_cached("UPDATE catalog_state SET next_source_generation = ?1")
        .map_err(database_error)?
        .execute([generation + 1])
        .map_err(database_error)?;
    Ok(generation)
}
