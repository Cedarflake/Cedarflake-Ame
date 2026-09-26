use std::sync::atomic::AtomicBool;

use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery};
use crate::domain::{IncrementalCatalogRoot, LeasedLibraryChange, LibraryChangeFailure};
use crate::ports::IncrementalCatalogRepository;

use super::preparation_catalog::{PreparationCatalog, PreparationReadSet};
use super::{
    PreparedChange, RevalidationTarget, cancellation_requested, prepare_change, revalidate_change,
};

pub(super) struct RebaseContext<'a> {
    pub(super) previous_root: &'a IncrementalCatalogRoot,
    pub(super) current_root: &'a IncrementalCatalogRoot,
    pub(super) leased: &'a [LeasedLibraryChange],
    pub(super) cancelled: Option<&'a AtomicBool>,
}

pub(super) enum RebasedPreparation {
    Ready {
        changes: Vec<PreparedChange>,
        reads: PreparationReadSet,
        retries: Vec<(LeasedLibraryChange, LibraryChangeFailure)>,
    },
    Cancelled,
}

pub(super) fn refresh_preparation(
    repository: &impl IncrementalCatalogRepository,
    discovery: &PublicationGuardedFileDiscovery,
    inspector: &LocalMediaInspector,
    prepared: Vec<PreparedChange>,
    reads: PreparationReadSet,
    context: RebaseContext<'_>,
) -> RebasedPreparation {
    let mut previous_root = context.previous_root.clone();
    previous_root.catalog_revision = context.current_root.catalog_revision;
    let bindings_match = previous_root == *context.current_root
        && reads.still_matches(repository, context.cancelled);
    let sources_match = bindings_match
        && prepared.iter().all(|change| {
            !cancellation_requested(context.cancelled)
                && !change.revalidation.is_empty()
                && change.revalidation.iter().all(|target| match target {
                    RevalidationTarget::Present { expected, .. } => {
                        expected.file_identity.is_some() && expected.source_revision.is_some()
                    }
                    RevalidationTarget::CatalogAbsent(_) => true,
                })
                && revalidate_change(discovery, change).is_ok()
        });
    if cancellation_requested(context.cancelled) {
        return RebasedPreparation::Cancelled;
    }
    if sources_match {
        return RebasedPreparation::Ready {
            changes: prepared,
            reads,
            retries: Vec::new(),
        };
    }
    drop(reads);
    let mut catalog = PreparationCatalog::new(repository);
    let mut changes = Vec::with_capacity(prepared.len());
    let mut retries = Vec::new();
    for change in prepared {
        if cancellation_requested(context.cancelled) {
            return RebasedPreparation::Cancelled;
        }
        let leased = context
            .leased
            .iter()
            .find(|leased| leased.change.id == change.completion.change_id)
            .expect("prepared changes originate from the leased batch");
        match prepare_change(&mut catalog, discovery, inspector, leased).and_then(|prepared| {
            revalidate_change(discovery, &prepared)?;
            Ok(prepared)
        }) {
            Ok(prepared) => changes.push(prepared),
            Err(issue) => retries.push((leased.clone(), issue)),
        }
    }
    RebasedPreparation::Ready {
        changes,
        reads: catalog.finish(),
        retries,
    }
}
