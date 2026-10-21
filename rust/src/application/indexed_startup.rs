use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use crate::adapters::{PublicationGuardedFileDiscovery, SqliteCatalog};
use crate::domain::{
    IncrementalCatalogRoot, LeasedLibraryChange, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRecoveryAuthorityReason, MetadataInventoryScope, MetadataInventoryStartRequest,
    ScanError,
};
use crate::ports::{
    IncrementalCatalogRepository, MetadataInventoryRepository, search_index::SearchIndex,
};

use super::metadata_inventory::unchanged_windows_recovery_location;

pub(crate) fn permits_indexed_startup(reason: LibraryRecoveryAuthorityReason) -> bool {
    matches!(
        reason,
        LibraryRecoveryAuthorityReason::JournalGap
            | LibraryRecoveryAuthorityReason::JournalReset
            | LibraryRecoveryAuthorityReason::JournalTrim
            | LibraryRecoveryAuthorityReason::JournalReconstructionFailure
            | LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
    )
}

pub(crate) fn prepare(
    catalog: &mut SqliteCatalog,
    index: &impl SearchIndex,
    root: &IncrementalCatalogRoot,
    lease: &LeasedLibraryChange,
    now_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<u32, ScanError> {
    let started = Instant::now();
    if lease.change.intent.scope != LibraryChangeScope::Root || cancelled.load(Ordering::Acquire) {
        return Ok(0);
    }
    let Some(authority) = catalog.load_metadata_inventory_recovery_authority(lease.change.id)?
    else {
        return Ok(0);
    };
    if authority.retired_unix_ms.is_some()
        || authority.root_id != root.root_id
        || authority.root_generation != root.root_generation
        || !permits_indexed_startup(authority.reason)
    {
        return Ok(0);
    }
    catalog.begin_search_index_hints()?;
    let rows = index.read(&root.root_path, cancelled, &mut |page| {
        catalog.stage_search_index_hints(page)
    })?;
    if rows == 0 || cancelled.load(Ordering::Acquire) {
        return Ok(0);
    }
    let limit = policy
        .lane_capacity(crate::domain::LibraryChangeLane::Recovery)
        .saturating_sub(1)
        .min(128);
    if limit == 0 {
        return Ok(0);
    }
    let paths = catalog.changed_search_index_paths(root, limit)?;
    if paths.is_empty() {
        eprintln!(
            "[Ame sync] indexed startup rows={rows} candidates=0 elapsed_ms={}",
            started.elapsed().as_millis()
        );
        return Ok(0);
    }
    let identity = root.publication_root_identity.as_ref().ok_or_else(|| {
        ScanError::new(
            "root_publication_namespace_unproven",
            "The configured root has no trustworthy persistent namespace identity",
        )
    })?;
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        identity,
    )?;
    let paths = changed_actual_paths(catalog, root, &discovery, paths, cancelled)?;
    if paths.is_empty() {
        eprintln!(
            "[Ame sync] indexed startup rows={rows} candidates=0 elapsed_ms={}",
            started.elapsed().as_millis()
        );
        return Ok(0);
    }
    let run = catalog.begin_next_metadata_inventory(&MetadataInventoryStartRequest {
        run_id: authority.run_id,
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        scope: MetadataInventoryScope::Root,
        started_unix_ms: now_unix_ms,
    })?;
    if let Some(existing) = catalog.load_metadata_inventory_root_identity(&run.request.run_id)? {
        discovery.require_metadata_inventory_root_identity(&existing)?;
    } else {
        // Candidate publication needs a durable namespace proof, not a completed inventory.
        catalog.initialize_metadata_inventory_spool(
            &run,
            lease,
            identity,
            None,
            Some(""),
            now_unix_ms,
        )?;
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(0);
    }
    let intents: Vec<_> = paths
        .into_iter()
        .map(|relative_path| LibraryChangeIntent {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Path,
            relative_path,
            previous_relative_path: None,
            origin: LibraryChangeOrigin::MetadataInventory,
            first_observed_unix_ms: now_unix_ms,
            most_recent_observed_unix_ms: now_unix_ms,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        })
        .collect();
    let count = catalog.publish_indexed_recovery_candidates(
        lease,
        &run.request.run_id,
        &intents,
        now_unix_ms,
        policy,
    )?;
    eprintln!(
        "[Ame sync] indexed startup rows={rows} candidates={count} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(count)
}

fn changed_actual_paths(
    catalog: &SqliteCatalog,
    root: &IncrementalCatalogRoot,
    discovery: &PublicationGuardedFileDiscovery,
    paths: Vec<String>,
    cancelled: &AtomicBool,
) -> Result<Vec<String>, ScanError> {
    if paths.is_empty() {
        return Ok(paths);
    }
    let priors: BTreeMap<_, _> = catalog
        .load_incremental_locations_by_relative_paths(&root.root_id, &paths)?
        .into_iter()
        .map(|prior| (prior.relative_path.clone(), prior))
        .collect();
    let mut changed = Vec::with_capacity(paths.len());
    for path in paths {
        if cancelled.load(Ordering::Acquire) {
            return Ok(Vec::new());
        }
        // A stale index must not turn an unchanged source into a known-dirty candidate.
        if let Some(prior) = priors.get(&path)
            && let Ok(entry) = discovery.metadata_inventory_entry(&path)
            && unchanged_windows_recovery_location(&entry, prior)
        {
            continue;
        }
        changed.push(path);
    }
    Ok(changed)
}
