#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use rusqlite::{OptionalExtension, Transaction, params};

#[cfg(test)]
use crate::domain::LibraryChangeLane;
use crate::domain::{
    FileIdentityEvidence, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration, ScanError,
};

use super::change_queue;
#[cfg(test)]
mod pagination_tests;
#[cfg(test)]
mod performance;
mod rejected_input_validation;
mod retained_issues;
pub(crate) use retained_issues::{
    load_retained_scan_issue_window, retained_scan_has_unclassified_issues,
};
#[cfg(test)]
mod tests;
mod transaction;
mod validation;
use super::{
    LiveGapRecoveryConsumer, ScanOwner, SqliteCatalog, StoredIdentityGroupState,
    allocate_source_generation, catalog_delta, consume_foreground_recovery_claims, database_error,
    delete_orphan_assets, detach_preview_references_for_root_locations,
    establish_root_publication_namespace, load_active_identity_group_state, load_catalog_revision,
    load_scan_identity_group_state, mark_unreferenced_preview_artifacts_stale, revisions_conflict,
    sqlite_integer, sqlite_unsigned,
};
use crate::ports::ScanPublicationControl;
pub(crate) use rejected_input_validation::RejectedInputValidationRoster;
use transaction::PublicationInterruption;
pub(crate) use transaction::publish_scan_with_proof;
pub(crate) use validation::{
    StagedValidationOutcome, StagedValidationRoster, ValidatedStagingProof,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScanPublicationReceipt {
    pub(crate) asset_count: u64,
}

const IDENTITY_RECONCILIATION_WINDOW: i64 = 128;

struct PublicationAuthority {
    previous_active_scan: Option<String>,
    root_generation: i64,
    change_queue_high_watermark: Option<i64>,
    scan_owner: String,
    namespace_identity: Option<FileIdentityEvidence>,
}

struct StoredIdentityObservationPayload {
    created_unix_ms: Option<i64>,
    width: i64,
    height: i64,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
}

struct StoredActiveIdentityProjectionPayload {
    created_unix_ms: Option<i64>,
    width: i64,
    height: i64,
    metadata_engine_id: String,
    metadata_engine_version: String,
    capture_local_time: Option<String>,
    capture_offset_minutes: Option<i64>,
    capture_time_source: Option<String>,
    capture_raw_value: Option<String>,
    preview_status: String,
    preview_issue_code: Option<String>,
    preview_issue_message: Option<String>,
}

#[cfg(test)]
type BeforeProjectionReplacementHook = Box<dyn FnOnce(&AtomicBool) + Send + 'static>;

#[cfg(test)]
static BEFORE_PROJECTION_REPLACEMENT_HOOKS: OnceLock<
    Mutex<HashMap<String, BeforeProjectionReplacementHook>>,
> = OnceLock::new();

#[cfg(test)]
pub(crate) struct BeforeProjectionReplacementHookGuard {
    scan_id: String,
}

#[cfg(test)]
impl Drop for BeforeProjectionReplacementHookGuard {
    fn drop(&mut self) {
        BEFORE_PROJECTION_REPLACEMENT_HOOKS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("scan projection replacement hooks")
            .remove(&self.scan_id);
    }
}

#[cfg(test)]
pub(crate) fn set_before_scan_projection_replacement_hook(
    scan_id: &str,
    hook: impl FnOnce(&AtomicBool) + Send + 'static,
) -> BeforeProjectionReplacementHookGuard {
    let replaced = BEFORE_PROJECTION_REPLACEMENT_HOOKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("scan projection replacement hooks")
        .insert(scan_id.to_owned(), Box::new(hook));
    assert!(
        replaced.is_none(),
        "a scan projection replacement hook already exists for this scan"
    );
    BeforeProjectionReplacementHookGuard {
        scan_id: scan_id.to_owned(),
    }
}

#[cfg(test)]
fn run_before_projection_replacement_hook(scan_id: &str, preempted: &AtomicBool) {
    let hook = BEFORE_PROJECTION_REPLACEMENT_HOOKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("scan projection replacement hooks")
        .remove(scan_id);
    if let Some(hook) = hook {
        hook(preempted);
    }
}

pub(super) fn publish_scan(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    asset_count: u64,
    issue_count: u64,
) -> Result<(), ScanError> {
    publish_scan_with_proof(
        catalog,
        scan_id,
        root_id,
        asset_count,
        issue_count,
        None,
        &ScanPublicationControl::default(),
    )
    .map(|_| ())
}

fn load_publication_authority(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    has_retry_paths: bool,
) -> Result<PublicationAuthority, ScanError> {
    let (
        previous_active_scan,
        root_generation,
        change_queue_high_watermark,
        requires_previous_snapshot,
        scan_owner,
    ) = transaction
        .query_row(
            "SELECT roots.active_scan_id, scans.root_generation_at_start,
                    scans.change_queue_high_watermark,
                    scans.requires_previous_snapshot, scans.scan_owner
             FROM library_roots AS roots
             JOIN scan_runs AS scans ON scans.id = ?1 AND scans.root_id = roots.id
             WHERE roots.id = ?2",
            params![scan_id, root_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .map_err(database_error)?;
    if requires_previous_snapshot {
        return Err(ScanError::new(
            "catalog_scan_requires_previous_snapshot",
            "The scan encountered evidence that requires retaining the previous catalog snapshot",
        ));
    }
    if has_retry_paths && scan_owner != ScanOwner::AuthoritativeRecovery.as_str() {
        return Err(ScanError::new(
            "catalog_scan_retry_paths_owner_invalid",
            "Only an authoritative recovery scan may publish durable retry paths",
        ));
    }
    let root_generation = root_generation.ok_or_else(|| {
        ScanError::new(
            "catalog_scan_generation_unverifiable",
            "The scan cannot prove the root generation captured at start",
        )
    })?;
    let root_generation_is_current = transaction
        .query_row(
            "SELECT generation = ?2 AND is_active = 1
             FROM library_change_root_state WHERE root_id = ?1",
            params![root_id, root_generation],
            |row| row.get::<_, bool>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or(false);
    if !root_generation_is_current {
        return Err(ScanError::new(
            "catalog_scan_root_generation_changed",
            "The library root changed while the authoritative scan was running",
        ));
    }
    if previous_active_scan.is_none() {
        ensure_first_import_change_capture(transaction, scan_id, root_id, root_generation)?;
    }
    let namespace_binding = transaction
        .query_row(
            "SELECT root_generation, identity_scheme, identity_value
             FROM library_scan_publication_namespace_bindings
             WHERE scan_id = ?1 AND root_id = ?2",
            params![scan_id, root_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    FileIdentityEvidence {
                        scheme: row.get(1)?,
                        value: row.get(2)?,
                    },
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    if namespace_binding
        .as_ref()
        .is_some_and(|(generation, _)| *generation != root_generation)
    {
        return Err(ScanError::new(
            "catalog_scan_publication_namespace_mismatch",
            "The scan namespace binding no longer belongs to its root generation",
        ));
    }
    Ok(PublicationAuthority {
        previous_active_scan,
        root_generation,
        change_queue_high_watermark,
        scan_owner,
        namespace_identity: namespace_binding.map(|(_, identity)| identity),
    })
}

fn ensure_first_import_change_capture(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    root_generation: i64,
) -> Result<(), ScanError> {
    let ready = transaction
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
               JOIN scan_runs AS scan
                 ON scan.id = ?1
                AND scan.root_id = journal.root_id
                AND scan.root_generation_at_start = journal.root_generation
               WHERE journal.root_id = ?2 AND journal.root_generation = ?3
                 AND journal.capability_state = 'live_only'
                 AND journal.continuity_state = 'live_only'
                 AND journal.updated_unix_ms >= scan.started_unix_ms
                 AND scan.scan_owner = 'foreground'
                 AND scan.status = 'running'
             )",
            params![scan_id, root_id, root_generation],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if ready {
        Ok(())
    } else {
        Err(ScanError::new(
            "catalog_first_import_change_capture_unproven",
            "The first catalog snapshot has no durable journal or healthy live-observer handoff",
        ))
    }
}

fn ensure_change_queue_is_publishable(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    authority: &PublicationAuthority,
) -> Result<(), ScanError> {
    // Live reconciliation needs an active snapshot. The first import has already proved its
    // opening change boundary; publishing its baseline neither consumes newer P0 nor proves
    // Current. A replacement must instead keep the existing snapshot until P0 can publish.
    if authority.previous_active_scan.is_none() {
        return Ok(());
    }
    let has_blocking_change = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM library_change_queue AS changes
               JOIN library_change_queue_lanes AS lanes ON lanes.change_id = changes.id
               WHERE changes.root_id = ?1 AND changes.root_generation = ?2
                 AND lanes.lane = 'p0_live'
                 AND (
                   changes.status IN ('pending', 'leased')
                   OR (changes.status = 'retry_wait' AND NOT (
                     changes.scope = 'path'
                     AND changes.intent_kind <> 'freshness_unknown'
                   ))
                 )
                 AND NOT EXISTS (
                   SELECT 1 FROM library_live_gap_recovery_claims AS claims
                   WHERE claims.gap_change_id = changes.id
                     AND claims.consumer_kind = ?3
                     AND claims.foreground_scan_id = ?4
                     AND claims.consumed_unix_ms IS NULL
                 )
             )",
            params![
                root_id,
                authority.root_generation,
                LiveGapRecoveryConsumer::ForegroundScan.as_str(),
                scan_id,
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if has_blocking_change {
        Err(ScanError::new(
            "catalog_scan_live_changes_pending",
            "The scan cannot publish while newer live changes are still unfinished",
        ))
    } else {
        Ok(())
    }
}

fn count_staged_assets(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    interruption: &PublicationInterruption,
) -> Result<i64, ScanError> {
    interruption.ensure_running()?;
    transaction
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE scan_id = ?1 AND root_id = ?2",
            params![scan_id, root_id],
            |row| row.get(0),
        )
        .map_err(database_error)
}

fn retain_previous_snapshot_handoffs(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    previous_active_scan: Option<&str>,
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    if let Some(previous_active_scan) = previous_active_scan
        && previous_active_scan != scan_id
    {
        catalog_delta::retain_scan_handoff_snapshots(
            transaction,
            scan_id,
            previous_active_scan,
            root_id,
            completed_unix_ms,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn complete_scan_and_replace_projection(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    authority: &PublicationAuthority,
    asset_count: i64,
    issue_count: i64,
    completed_unix_ms: i64,
    interruption: &PublicationInterruption,
) -> Result<(), ScanError> {
    let updated = transaction
        .execute(
            "UPDATE scan_runs
             SET status = 'completed', completed_unix_ms = ?2,
                 asset_count = ?3, issue_count = ?4,
                 current_directory_relative_path = NULL,
                 current_directory_enumerated = 0,
                 last_visited_relative_path = NULL
             WHERE id = ?1 AND status = 'running'",
            params![scan_id, completed_unix_ms, asset_count, issue_count],
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
    detach_preview_references_for_root_locations(transaction, root_id, Some(scan_id))?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO preview_artifact_locations(artifact_key, location_id)
             SELECT artifacts.artifact_key, locations.location_id
             FROM asset_locations AS locations
             JOIN preview_artifacts AS artifacts
               ON artifacts.artifact_path = locations.preview_path
             WHERE locations.root_id = ?1 AND locations.scan_id = ?2
               AND locations.preview_status = 'ready'",
            params![root_id, scan_id],
        )
        .map_err(database_error)?;
    mark_unreferenced_preview_artifacts_stale(transaction)?;

    interruption.ensure_running()?;
    #[cfg(test)]
    run_before_projection_replacement_hook(scan_id, interruption.preemption_flag());
    if let Some(previous_active_scan) = authority.previous_active_scan.as_deref()
        && previous_active_scan != scan_id
    {
        transaction
            .execute(
                "DELETE FROM asset_locations WHERE scan_id = ?1",
                [previous_active_scan],
            )
            .map_err(database_error)?;
    }
    delete_orphan_assets(transaction)
}

fn publish_root_authority(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    authority: &PublicationAuthority,
    completed_unix_ms: i64,
    interruption: &PublicationInterruption,
) -> Result<u64, ScanError> {
    let revision_updated = transaction
        .execute("UPDATE catalog_state SET revision = revision + 1", [])
        .map_err(database_error)?;
    if revision_updated != 1 {
        return Err(ScanError::new(
            "catalog_revision_unavailable",
            "The catalog revision state is missing or invalid",
        ));
    }
    let published_revision = load_catalog_revision(transaction)?;
    if let Some(identity) = authority.namespace_identity.as_ref() {
        establish_root_publication_namespace(
            transaction,
            root_id,
            authority.root_generation,
            identity,
            "foreground_scan",
            published_revision,
            completed_unix_ms,
        )?;
        let deleted = transaction
            .execute(
                "DELETE FROM library_scan_publication_namespace_bindings
                 WHERE scan_id = ?1 AND root_id = ?2 AND root_generation = ?3",
                params![scan_id, root_id, authority.root_generation],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(ScanError::new(
                "catalog_scan_publication_namespace_raced",
                "The scan namespace binding changed during catalog publication",
            ));
        }
    }
    if authority.scan_owner == ScanOwner::Foreground.as_str() {
        consume_foreground_recovery_claims(
            transaction,
            scan_id,
            root_id,
            authority.root_generation,
            published_revision,
            completed_unix_ms,
        )?;
    }
    interruption.ensure_running()?;
    Ok(published_revision)
}

#[allow(clippy::too_many_arguments)]
fn settle_change_lineage(
    transaction: &Transaction<'_>,
    scan_id: &str,
    root_id: &str,
    authority: &PublicationAuthority,
    retry_relative_paths: &[String],
    catch_up_lineage: &[(String, String)],
    published_revision: u64,
    completed_unix_ms: i64,
    interruption: &PublicationInterruption,
) -> Result<(), ScanError> {
    if let Some(high_watermark) = authority.change_queue_high_watermark {
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET status = 'completed', next_retry_unix_ms = NULL,
                     lease_expires_unix_ms = NULL,
                     catalog_revision_at_success = ?1, updated_unix_ms = ?2,
                     authoritative_scan_id = NULL
                 WHERE root_id = ?3 AND root_generation = ?4
                   AND authoritative_scan_id = ?6 AND id <= ?5
                   AND status IN ('pending', 'leased', 'retry_wait')
                   AND NOT (
                     status = 'retry_wait' AND scope = 'path'
                     AND intent_kind <> 'freshness_unknown'
                   )
                   AND NOT EXISTS(
                     SELECT 1 FROM library_live_gap_recovery_claims AS claim
                     WHERE claim.gap_change_id = library_change_queue.id
                   )",
                params![
                    sqlite_integer(published_revision, "catalog revision")?,
                    completed_unix_ms,
                    root_id,
                    authority.root_generation,
                    high_watermark,
                    scan_id,
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "UPDATE library_change_queue
                 SET authoritative_scan_id = NULL
                 WHERE authoritative_scan_id = ?1",
                [scan_id],
            )
            .map_err(database_error)?;
    }
    enqueue_authoritative_retry_paths(
        transaction,
        root_id,
        authority.root_generation,
        retry_relative_paths,
        completed_unix_ms,
    )?;
    for (source, watermark) in catch_up_lineage {
        catalog_delta::cleanup_terminal_catch_up_handoffs(transaction, source, watermark)?;
    }
    transaction
        .execute(
            "DELETE FROM scan_run_catch_up_lineage WHERE scan_id = ?1",
            [scan_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM library_scan_publication_namespace_bindings WHERE scan_id = ?1",
            [scan_id],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_root_state
             SET last_consistency_audit_unix_ms = ?2, updated_unix_ms = ?2
             WHERE root_id = ?1 AND generation = ?3 AND is_active = 1",
            params![root_id, completed_unix_ms, authority.root_generation],
        )
        .map_err(database_error)?;
    interruption.ensure_running()
}

fn enqueue_authoritative_retry_paths(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_generation: i64,
    retry_relative_paths: &[String],
    completed_unix_ms: i64,
) -> Result<(), ScanError> {
    if retry_relative_paths.is_empty() {
        return Ok(());
    }
    let root_generation =
        LibraryRootGeneration::new(sqlite_unsigned(root_generation, "root generation")?)
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_scan_generation_invalid",
                    "The authoritative scan captured an invalid root generation",
                )
            })?;
    let retry_intents = retry_relative_paths
        .iter()
        .zip(1_u64..)
        .map(|(relative_path, sequence)| LibraryChangeIntent {
            root_id: root_id.to_owned(),
            root_generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Path,
            relative_path: relative_path.clone(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
            first_observed_unix_ms: completed_unix_ms,
            most_recent_observed_unix_ms: completed_unix_ms,
            first_sequence: sequence,
            most_recent_sequence: sequence,
            coalesced_observation_count: 1,
        })
        .collect::<Vec<_>>();
    change_queue::validate_enqueue_batch(&retry_intents)?;
    let report = change_queue::enqueue_intents_in_transaction(
        transaction,
        &retry_intents,
        None,
        completed_unix_ms,
        LibraryChangeQueuePolicy::default(),
    )?;
    if report.stale_generation_count > 0 {
        return Err(ScanError::new(
            "catalog_scan_retry_generation_stale",
            "The authoritative retry paths no longer belong to the current root generation",
        ));
    }
    Ok(())
}

fn reconcile_identity_pages(
    transaction: &Transaction<'_>,
    scan_id: &str,
    interruption: &PublicationInterruption,
) -> Result<(), ScanError> {
    let mut cursor: Option<FileIdentityEvidence> = None;
    loop {
        interruption.ensure_running()?;
        let identities = load_identity_page(transaction, scan_id, cursor.as_ref())?;
        if identities.is_empty() {
            return Ok(());
        }
        for identity in identities {
            interruption.ensure_running()?;
            reconcile_identity_group(transaction, scan_id, &identity)?;
            cursor = Some(identity);
        }
    }
}

fn load_identity_page(
    transaction: &Transaction<'_>,
    scan_id: &str,
    after: Option<&FileIdentityEvidence>,
) -> Result<Vec<FileIdentityEvidence>, ScanError> {
    let page = if after.is_some() {
        "AND (file_identity_scheme, file_identity_value) > (?2, ?3)
         ORDER BY file_identity_scheme, file_identity_value LIMIT ?4"
    } else {
        "ORDER BY file_identity_scheme, file_identity_value LIMIT ?2"
    };
    let mut statement = transaction
        .prepare(&format!(
            "SELECT DISTINCT file_identity_scheme, file_identity_value
             FROM asset_locations
             WHERE scan_id = ?1 AND file_identity_scheme IS NOT NULL
               {page}",
        ))
        .map_err(database_error)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(FileIdentityEvidence {
            scheme: row.get(0)?,
            value: row.get(1)?,
        })
    };
    let rows = match after {
        Some(after) => statement.query_map(
            params![
                scan_id,
                after.scheme,
                after.value,
                IDENTITY_RECONCILIATION_WINDOW,
            ],
            map_row,
        ),
        None => statement.query_map(params![scan_id, IDENTITY_RECONCILIATION_WINDOW], map_row),
    }
    .map_err(database_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(database_error)
}

fn reconcile_identity_group(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
) -> Result<(), ScanError> {
    let staged =
        load_scan_identity_group_state(transaction, identity, scan_id)?.ok_or_else(|| {
            ScanError::new(
                "catalog_scan_source_state_missing",
                "A staged physical source identity has no coherent observation",
            )
        })?;
    let Some(active) = load_active_identity_group_state(transaction, identity)? else {
        return Ok(());
    };
    let compatible = staged.file_size == active.file_size
        && staged.modified_unix_ms == active.modified_unix_ms
        && !revisions_conflict(
            staged.source_revision_token.as_deref(),
            active.source_revision_token.as_deref(),
        );
    if compatible {
        return adopt_compatible_identity(transaction, scan_id, identity, &staged, &active);
    }
    if active.generation > staged.generation {
        return mirror_newer_active_identity(transaction, scan_id, identity, &active);
    }
    if active.generation == staged.generation {
        return Err(ScanError::new(
            "catalog_scan_source_state_conflict",
            "The active and staged catalogs disagree at the same source generation",
        ));
    }
    supersede_active_identity(transaction, scan_id, identity, &staged)
}

fn adopt_compatible_identity(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
    staged: &StoredIdentityGroupState,
    active: &StoredIdentityGroupState,
) -> Result<(), ScanError> {
    let adopted_revision = staged
        .source_revision_token
        .as_ref()
        .or(active.source_revision_token.as_ref());
    transaction
        .execute(
            "UPDATE asset_locations
             SET source_revision_token = ?3, source_generation = ?4
             WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
               AND (
                 scan_id = ?5 OR EXISTS (
                   SELECT 1 FROM library_roots AS roots
                   WHERE roots.id = asset_locations.root_id
                     AND roots.active_scan_id = asset_locations.scan_id
                 )
               )",
            params![
                identity.scheme,
                identity.value,
                adopted_revision,
                active.generation.max(staged.generation),
                scan_id,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn mirror_newer_active_identity(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
    active: &StoredIdentityGroupState,
) -> Result<(), ScanError> {
    let payload = load_active_identity_projection_payload(transaction, identity)?;
    let preview_status = if payload.preview_status == "failed" {
        "failed"
    } else {
        "pending"
    };
    let preview_issue_code = (preview_status == "failed")
        .then_some(payload.preview_issue_code.as_deref())
        .flatten();
    let preview_issue_message = (preview_status == "failed")
        .then_some(payload.preview_issue_message.as_deref())
        .flatten();
    transaction
        .execute(
            "UPDATE asset_locations
             SET file_size = ?4, created_unix_ms = ?5, modified_unix_ms = ?6,
                 file_local_time = strftime(
                   '%Y-%m-%dT%H:%M:%f', COALESCE(?5, ?6) / 1000.0,
                   'unixepoch', 'localtime'
                 ),
                 width = ?7, height = ?8,
                 metadata_engine_id = ?9, metadata_engine_version = ?10,
                 capture_local_time = ?11, capture_offset_minutes = ?12,
                 capture_time_source = ?13, capture_raw_value = ?14,
                 source_revision_token = ?15, source_generation = ?16,
                 preview_path = '', preview_status = ?17,
                 preview_issue_code = ?18, preview_issue_message = ?19
             WHERE scan_id = ?3
               AND file_identity_scheme = ?1 AND file_identity_value = ?2",
            params![
                identity.scheme,
                identity.value,
                scan_id,
                active.file_size,
                payload.created_unix_ms,
                active.modified_unix_ms,
                payload.width,
                payload.height,
                payload.metadata_engine_id,
                payload.metadata_engine_version,
                payload.capture_local_time,
                payload.capture_offset_minutes,
                payload.capture_time_source,
                payload.capture_raw_value,
                active.source_revision_token,
                active.generation,
                preview_status,
                preview_issue_code,
                preview_issue_message,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn supersede_active_identity(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
    staged: &StoredIdentityGroupState,
) -> Result<(), ScanError> {
    let payload = load_staged_identity_payload(transaction, scan_id, identity)?;
    let generation = allocate_source_generation(transaction)?;
    transaction
        .execute(
            "DELETE FROM preview_artifact_locations
             WHERE location_id IN (
               SELECT locations.location_id
               FROM asset_locations AS locations
               WHERE locations.file_identity_scheme = ?1
                 AND locations.file_identity_value = ?2
                 AND (
                   locations.scan_id = ?3 OR EXISTS (
                     SELECT 1 FROM library_roots AS roots
                     WHERE roots.id = locations.root_id
                       AND roots.active_scan_id = locations.scan_id
                   )
                 )
             )",
            params![identity.scheme, identity.value, scan_id],
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
             SET file_size = ?4, created_unix_ms = ?5, modified_unix_ms = ?6,
                 file_local_time = strftime(
                   '%Y-%m-%dT%H:%M:%f', COALESCE(?5, ?6) / 1000.0,
                   'unixepoch', 'localtime'
                 ),
                 width = ?7, height = ?8,
                 metadata_engine_id = ?9, metadata_engine_version = ?10,
                 capture_local_time = ?11, capture_offset_minutes = ?12,
                 capture_time_source = ?13, capture_raw_value = ?14,
                 source_revision_token = ?15, source_generation = ?16,
                 preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
               AND (
                 scan_id = ?3 OR EXISTS (
                   SELECT 1 FROM library_roots AS roots
                   WHERE roots.id = asset_locations.root_id
                     AND roots.active_scan_id = asset_locations.scan_id
                 )
               )",
            params![
                identity.scheme,
                identity.value,
                scan_id,
                staged.file_size,
                payload.created_unix_ms,
                staged.modified_unix_ms,
                payload.width,
                payload.height,
                payload.metadata_engine_id,
                payload.metadata_engine_version,
                payload.capture_local_time,
                payload.capture_offset_minutes,
                payload.capture_time_source,
                payload.capture_raw_value,
                staged.source_revision_token,
                generation,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}

fn load_active_identity_projection_payload(
    transaction: &Transaction<'_>,
    identity: &FileIdentityEvidence,
) -> Result<StoredActiveIdentityProjectionPayload, ScanError> {
    transaction
        .query_row(
            "SELECT locations.created_unix_ms, locations.width, locations.height,
                    locations.metadata_engine_id, locations.metadata_engine_version,
                    locations.capture_local_time, locations.capture_offset_minutes,
                    locations.capture_time_source, locations.capture_raw_value,
                    locations.preview_status, locations.preview_issue_code,
                    locations.preview_issue_message
             FROM asset_locations AS locations
             JOIN library_roots AS roots
               ON roots.id = locations.root_id
              AND roots.active_scan_id = locations.scan_id
             WHERE locations.file_identity_scheme = ?1
               AND locations.file_identity_value = ?2
             ORDER BY locations.root_id, locations.location_id
             LIMIT 1",
            params![identity.scheme, identity.value],
            |row| {
                Ok(StoredActiveIdentityProjectionPayload {
                    created_unix_ms: row.get(0)?,
                    width: row.get(1)?,
                    height: row.get(2)?,
                    metadata_engine_id: row.get(3)?,
                    metadata_engine_version: row.get(4)?,
                    capture_local_time: row.get(5)?,
                    capture_offset_minutes: row.get(6)?,
                    capture_time_source: row.get(7)?,
                    capture_raw_value: row.get(8)?,
                    preview_status: row.get(9)?,
                    preview_issue_code: row.get(10)?,
                    preview_issue_message: row.get(11)?,
                })
            },
        )
        .map_err(database_error)
}

fn load_staged_identity_payload(
    transaction: &Transaction<'_>,
    scan_id: &str,
    identity: &FileIdentityEvidence,
) -> Result<StoredIdentityObservationPayload, ScanError> {
    transaction
        .query_row(
            "SELECT created_unix_ms, width, height, metadata_engine_id,
                    metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value
             FROM asset_locations
             WHERE scan_id = ?1 AND file_identity_scheme = ?2
               AND file_identity_value = ?3
             ORDER BY location_id LIMIT 1",
            params![scan_id, identity.scheme, identity.value],
            |row| {
                Ok(StoredIdentityObservationPayload {
                    created_unix_ms: row.get(0)?,
                    width: row.get(1)?,
                    height: row.get(2)?,
                    metadata_engine_id: row.get(3)?,
                    metadata_engine_version: row.get(4)?,
                    capture_local_time: row.get(5)?,
                    capture_offset_minutes: row.get(6)?,
                    capture_time_source: row.get(7)?,
                    capture_raw_value: row.get(8)?,
                })
            },
        )
        .map_err(database_error)
}
