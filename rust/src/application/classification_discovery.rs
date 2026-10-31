use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::DiscoveryCohort;
use crate::ports::classification_encoder::ClassificationEncoderFactory;
use crate::ports::classification_features::ClassificationFeatureRepository;
use crate::ports::classification_vocabulary::{
    ClassificationVocabulary, ConceptNeighborhood, ConceptPage,
};

use super::classification_features::load_or_encode_descriptions;
use super::classification_residency::{ClassificationEncoderResidency, ResidencyClock};

mod admission;
mod consolidation;
mod context_graph;
mod nomination;
mod ranking;
mod search;

pub use admission::{
    CategoryAdmission, CategoryAdmissionCandidate, CategoryAdmissionCriteria,
    CategoryAdmissionDisposition, CategoryAdmissionPolicy, CategoryMemberEvidence, MemberAdmission,
    assess_category_candidates,
};
pub use consolidation::{
    ConceptConsolidation, ConceptConsolidationLimits, ConceptConsolidationRequest,
    ConsolidatedConcept, consolidate_classification_concepts,
};
pub use nomination::{
    ConceptNomination, ConceptNominationRequest, nominate_classification_contexts,
};
pub use ranking::{ConceptMatch, DiscoveryCandidates};
use search::ConceptSearch;
pub use search::{ConceptSearchBound, ConceptSearchLimits};

pub fn retrieve_classification_concepts<F: ClassificationEncoderFactory, C: ResidencyClock>(
    cohort: &DiscoveryCohort,
    vocabulary: &mut impl ClassificationVocabulary,
    limits: ConceptSearchLimits,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<DiscoveryCandidates, ScanError> {
    let result = retrieve(cohort, vocabulary, limits, residency, cache, cancelled);
    residency.finish_descriptions()?;
    result
}

fn retrieve<F: ClassificationEncoderFactory, C: ResidencyClock>(
    cohort: &DiscoveryCohort,
    vocabulary: &mut impl ClassificationVocabulary,
    limits: ConceptSearchLimits,
    residency: &mut ClassificationEncoderResidency<F, C>,
    cache: &mut impl ClassificationFeatureRepository,
    cancelled: &AtomicBool,
) -> Result<DiscoveryCandidates, ScanError> {
    limits.validate()?;
    check_cancelled(cancelled)?;
    if cohort.model() != residency.compatibility() {
        return Err(ScanError::new(
            "classification_feature_mismatch",
            "The discovery samples do not match the active encoder",
        ));
    }
    let revision = vocabulary.revision();
    let mut search = ConceptSearch::new(limits);
    let mut after = None;
    loop {
        let remaining = limits.entry_points - search.scored_concepts();
        if remaining == 0 {
            search.reached(ConceptSearchBound::EntryPoints);
            break;
        }
        let page = read_page(
            vocabulary,
            revision,
            ConceptNeighborhood::EntryPoints,
            after,
            remaining,
            cancelled,
        )?;
        after = page.concepts.last().map(|concept| concept.id());
        let descriptions: Vec<_> = page
            .concepts
            .iter()
            .map(|concept| concept.description())
            .collect();
        let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
        for concept in page.concepts {
            check_cancelled(cancelled)?;
            let revision = ClassificationRevision::description(concept.description());
            search.rank(concept, cohort, &features[&revision])?;
        }
        if !page.has_more {
            break;
        }
    }

    let mut frontier: VecDeque<_> = search.seeds().into_iter().map(|id| (id, 0)).collect();
    let mut scheduled: std::collections::BTreeSet<_> = frontier.iter().map(|(id, _)| *id).collect();
    let mut expanded = 0;
    let mut edges = 0;
    'graph: while let Some((id, depth)) = frontier.pop_front() {
        if depth == limits.hops {
            continue;
        }
        let mut after = None;
        loop {
            if expanded == limits.expanded_concepts {
                search.reached(ConceptSearchBound::ExpandedConcepts);
                break 'graph;
            }
            if edges == limits.edges {
                search.reached(ConceptSearchBound::Edges);
                break 'graph;
            }
            let page = read_page(
                vocabulary,
                revision,
                ConceptNeighborhood::Related(id),
                after,
                limits.edges - edges,
                cancelled,
            )?;
            after = page.concepts.last().map(|concept| concept.id());
            let mut descriptions = Vec::new();
            for concept in &page.concepts {
                if !search.has_concept(concept)? {
                    descriptions.push(concept.description());
                    if descriptions.len() == limits.expanded_concepts - expanded {
                        break;
                    }
                }
            }
            let features = load_or_encode_descriptions(&descriptions, residency, cache, cancelled)?;
            for concept in page.concepts {
                check_cancelled(cancelled)?;
                edges += 1;
                let id = concept.id();
                if !search.has_concept(&concept)? {
                    if expanded == limits.expanded_concepts {
                        search.reached(ConceptSearchBound::ExpandedConcepts);
                        break 'graph;
                    }
                    let revision = ClassificationRevision::description(concept.description());
                    search.rank(concept, cohort, &features[&revision])?;
                    expanded += 1;
                }
                if scheduled.insert(id) {
                    frontier.push_back((id, depth + 1));
                }
            }
            if !page.has_more {
                break;
            }
        }
    }
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    Ok(search.finish(cohort, revision, edges))
}

fn read_page(
    vocabulary: &mut impl ClassificationVocabulary,
    revision: ClassificationRevision,
    neighborhood: ConceptNeighborhood,
    after: Option<crate::domain::classification::discovery::ConceptId>,
    remaining: usize,
    cancelled: &AtomicBool,
) -> Result<ConceptPage, ScanError> {
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    let limit = remaining.min(64);
    let page = vocabulary.read_concepts(neighborhood, after, limit)?;
    check_cancelled(cancelled)?;
    check_revision(vocabulary, revision)?;
    if page.concepts.len() > limit || (page.has_more && page.concepts.is_empty()) {
        return Err(invalid_page());
    }
    let mut previous = after;
    for concept in &page.concepts {
        if previous.is_some_and(|id| concept.id() <= id) {
            return Err(invalid_page());
        }
        previous = Some(concept.id());
    }
    Ok(page)
}

fn check_revision(
    vocabulary: &impl ClassificationVocabulary,
    revision: ClassificationRevision,
) -> Result<(), ScanError> {
    if vocabulary.revision() != revision {
        return Err(ScanError::new(
            "classification_vocabulary_changed",
            "The discovery vocabulary changed during retrieval",
        ));
    }
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(ScanError::new(
            "classification_cancelled",
            "Classification was cancelled",
        ));
    }
    Ok(())
}

fn invalid_page() -> ScanError {
    ScanError::new(
        "classification_vocabulary_invalid",
        "The vocabulary returned an invalid concept window",
    )
}

#[cfg(test)]
mod tests;
