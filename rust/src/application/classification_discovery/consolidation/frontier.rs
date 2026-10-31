use std::collections::{BTreeMap, BTreeSet, VecDeque, btree_map::Entry};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{ConceptId, VocabularyConcept};
use crate::ports::classification_vocabulary::{ClassificationVocabulary, ConceptNeighborhood};

use super::super::{check_cancelled, check_revision, read_page};
use super::{ConceptConsolidationRequest, limit_error};

pub(super) struct ResolvedFrontier {
    pub concepts: BTreeMap<ConceptId, VocabularyConcept>,
    pub groups: BTreeMap<ConceptId, BTreeSet<ConceptId>>,
    pub unresolved: Vec<ConceptId>,
    pub visited_edges: usize,
}

pub(super) fn resolve(
    request: ConceptConsolidationRequest<'_>,
    vocabulary: &mut impl ClassificationVocabulary,
    cancelled: &AtomicBool,
) -> Result<ResolvedFrontier, ScanError> {
    let ConceptConsolidationRequest {
        retrieved,
        frontier,
        limits,
        ..
    } = request;
    let revision = retrieved.vocabulary_revision;
    let mut concepts = BTreeMap::new();
    for candidate in retrieved.nearest.iter().chain(&retrieved.distinctive) {
        remember(&mut concepts, candidate.concept.clone(), limits.concepts)?;
    }
    let seeds: BTreeSet<_> = concepts.keys().copied().collect();
    let mut pending: VecDeque<_> = seeds.iter().map(|&id| (id, id, 0)).collect();
    let mut scheduled: BTreeSet<_> = seeds.iter().map(|&id| (id, id)).collect();
    let mut parents = BTreeMap::<ConceptId, Vec<ConceptId>>::new();
    let mut groups = BTreeMap::<ConceptId, BTreeSet<ConceptId>>::new();
    let mut read_edges = 0;
    let mut visited_edges = 0;
    while let Some((id, seed, depth)) = pending.pop_front() {
        check_cancelled(cancelled)?;
        check_revision(vocabulary, revision)?;
        if frontier.contains(&id) {
            groups.entry(id).or_default().insert(seed);
            continue;
        }
        if depth == limits.hops {
            continue;
        }
        if let Entry::Vacant(entry) = parents.entry(id) {
            let mut ids = Vec::new();
            let mut after = None;
            loop {
                if read_edges == limits.edges {
                    return Err(limit_error());
                }
                let page = read_page(
                    vocabulary,
                    revision,
                    ConceptNeighborhood::Broader(id),
                    after,
                    limits.edges - read_edges,
                    cancelled,
                )?;
                after = page.concepts.last().map(VocabularyConcept::id);
                for concept in page.concepts {
                    ids.push(concept.id());
                    remember(&mut concepts, concept, limits.concepts)?;
                    read_edges += 1;
                }
                if !page.has_more {
                    break;
                }
            }
            entry.insert(ids);
        }
        for &parent in &parents[&id] {
            if visited_edges == limits.edges {
                return Err(limit_error());
            }
            visited_edges += 1;
            if scheduled.insert((parent, seed)) {
                pending.push_back((parent, seed, depth + 1));
            }
        }
    }
    let covered: BTreeSet<_> = groups.values().flatten().copied().collect();
    Ok(ResolvedFrontier {
        concepts,
        groups,
        unresolved: seeds.difference(&covered).copied().collect(),
        visited_edges,
    })
}

fn remember(
    concepts: &mut BTreeMap<ConceptId, VocabularyConcept>,
    concept: VocabularyConcept,
    limit: usize,
) -> Result<(), ScanError> {
    if let Some(previous) = concepts.get(&concept.id()) {
        if previous.revision() != concept.revision() {
            return Err(ScanError::new(
                "classification_vocabulary_changed",
                "A broader concept changed within the naming vocabulary",
            ));
        }
    } else {
        if concepts.len() == limit {
            return Err(limit_error());
        }
        concepts.insert(concept.id(), concept);
    }
    Ok(())
}
