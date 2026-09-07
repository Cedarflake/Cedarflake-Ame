use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery};
use crate::domain::{LeasedLibraryChange, LibraryChangeFailure};
use crate::ports::IncrementalCatalogRepository;

use super::{
    InspectedPath, PathChangeContext, PreparedChange, failure, inspect_path, prepare_path_change,
    push_unique, scan_failure, windows_case_alias,
};

pub(super) fn prepare_rename_change<Repository>(
    repository: &Repository,
    discovery: &PublicationGuardedFileDiscovery,
    inspector: &LocalMediaInspector,
    leased: &LeasedLibraryChange,
) -> Result<PreparedChange, LibraryChangeFailure>
where
    Repository: IncrementalCatalogRepository,
{
    let intent = &leased.change.intent;
    let previous_path = intent.previous_relative_path.as_deref().ok_or_else(|| {
        failure(
            "incremental_rename_previous_path_missing",
            "A paired rename requires its previous relative path",
        )
    })?;
    let previous_prior = repository
        .load_incremental_location_by_relative_path(&intent.root_id, previous_path)
        .map_err(scan_failure)?;
    let previous_observed = inspect_path(discovery, previous_path);
    let previous_identity = match &previous_observed {
        InspectedPath::File(file) => file.file_identity.clone(),
        InspectedPath::TerminalMedia { file, .. } => file.file_identity.clone(),
        _ => None,
    };
    let previous_is_absent = match &previous_observed {
        InspectedPath::CatalogAbsent => true,
        InspectedPath::File(_) | InspectedPath::TerminalMedia { .. } => false,
        InspectedPath::PreservedIssue(issue) | InspectedPath::Retry(issue) => {
            return Err(issue.clone());
        }
    };
    let mut previous = prepare_path_change(
        repository,
        discovery,
        inspector,
        leased,
        PathChangeContext {
            relative_path: previous_path,
            observed: Some(previous_observed),
            candidate_prior: None,
            may_remove_candidate_prior: false,
            removals: Vec::new(),
        },
    )?;
    let mut current = prepare_path_change(
        repository,
        discovery,
        inspector,
        leased,
        PathChangeContext {
            relative_path: &intent.relative_path,
            observed: None,
            candidate_prior: previous_prior.clone(),
            may_remove_candidate_prior: previous_is_absent,
            removals: Vec::new(),
        },
    )?;
    let current_identity = current
        .mutations
        .iter()
        .find_map(|mutation| mutation.upsert_location.as_ref())
        .and_then(|location| location.file_identity.clone());
    if previous_is_absent
        && current
            .mutations
            .iter()
            .any(|mutation| mutation.upsert_location.is_some())
    {
        previous.mutations.clear();
    }
    if windows_case_alias(previous_path, &intent.relative_path)
        && previous_identity.is_some()
        && previous_identity == current_identity
    {
        previous.mutations.clear();
        if let Some(prior) = &previous_prior {
            for mutation in &mut current.mutations {
                if mutation.upsert_location.is_some() {
                    push_unique(&mut mutation.remove_location_ids, prior.location_id.clone());
                }
            }
        }
    }
    previous.mutations.append(&mut current.mutations);
    previous.revalidation.append(&mut current.revalidation);
    // One lease owns terminal evidence only for its current path. Previous-path invalidation
    // remains part of the same delta transaction, not a second evidence owner.
    previous.terminal_media_evidence = current.terminal_media_evidence;
    if previous.completion.issue.is_none() {
        previous.completion.issue = current.completion.issue;
    }
    Ok(previous)
}
