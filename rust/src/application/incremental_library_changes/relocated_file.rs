use crate::domain::{
    AssetLocationView, LeasedLibraryChange, LibraryChangeFailure, LibraryChangeIntentKind,
    LibraryChangeOrigin,
};
use crate::ports::IncrementalCatalogRepository;
use crate::ports::media_inspection::GuardedMediaInspector;
use crate::ports::source_publication::PublicationFileSource;

use super::preparation_catalog::PreparationCatalog;
use super::{
    InspectedPath, PathChangeContext, PreparedChange, RevalidationTarget, failure, inspect_path,
    issue_failure, prepare_path_change, push_unique,
};

pub(super) fn prepare_relocated_file<Repository>(
    repository: &mut PreparationCatalog<'_, Repository>,
    discovery: &impl PublicationFileSource,
    inspector: &impl GuardedMediaInspector,
    leased: &LeasedLibraryChange,
    prior: &AssetLocationView,
    removals: &[String],
) -> Result<Option<PreparedChange>, LibraryChangeFailure>
where
    Repository: IncrementalCatalogRepository,
{
    let intent = &leased.change.intent;
    if intent.origin != LibraryChangeOrigin::LiveNotification
        || intent.kind != LibraryChangeIntentKind::Reconcile
    {
        return Ok(None);
    }
    let Some(identity) = prior.file_identity.as_ref() else {
        return Ok(None);
    };
    let Some(relative_path) = discovery
        .locate_relative_file(identity)
        .map_err(|issue| issue_failure(&issue))?
    else {
        return Ok(None);
    };
    if relative_path == prior.relative_path {
        return Err(failure(
            "source_file_changed",
            "The source path changed while checking the removed location",
        ));
    }
    let observed = inspect_path(discovery, &relative_path);
    match &observed {
        InspectedPath::File(file) if file.file_identity.as_ref() == Some(identity) => {}
        InspectedPath::PreservedIssue(issue) | InspectedPath::Retry(issue) => {
            return Err(issue.clone());
        }
        InspectedPath::TerminalMedia { .. } | InspectedPath::CatalogAbsent => return Ok(None),
        InspectedPath::File(_) => {
            return Err(failure(
                "source_file_changed",
                "The relocated source no longer matches its retained identity",
            ));
        }
    }
    let mut removals = removals.to_vec();
    push_unique(&mut removals, prior.location_id.clone());
    let mut prepared = prepare_path_change(
        repository,
        discovery,
        inspector,
        leased,
        PathChangeContext {
            relative_path: &relative_path,
            observed: Some(observed),
            candidate_prior: Some(prior.clone()),
            may_remove_candidate_prior: true,
            removals,
        },
    )?;
    // Terminal media evidence belongs to the destination's own lease, not an old-path removal.
    if !prepared.terminal_media_evidence.is_empty() {
        return Ok(None);
    }
    prepared
        .revalidation
        .push(RevalidationTarget::CatalogAbsent(
            prior.relative_path.clone(),
        ));
    Ok(Some(prepared))
}
