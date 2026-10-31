use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::{ConceptId, ConceptNamingScope, VocabularyConcept};
use crate::ports::classification_vocabulary::{
    ClassificationVocabulary, ConceptNeighborhood, ConceptPage,
};

use super::{check_cancelled, check_revision, read_page};

pub struct NamingContext {
    pub concept: VocabularyConcept,
    pub cues: Vec<ConceptId>,
    pub broader: Vec<ConceptId>,
}

pub struct NamingContextCatalog {
    contexts: BTreeMap<ConceptId, VocabularyConcept>,
    cues: BTreeMap<ConceptId, Vec<ConceptId>>,
    broader: BTreeMap<ConceptId, Vec<ConceptId>>,
    revision: ClassificationRevision,
}

impl NamingContextCatalog {
    pub fn new(definitions: Vec<NamingContext>) -> Result<Self, ScanError> {
        if definitions.len() > 4096 {
            return Err(invalid_catalog());
        }
        let mut contexts = BTreeMap::new();
        let mut cues = BTreeMap::<ConceptId, Vec<ConceptId>>::new();
        let mut broader = BTreeMap::new();
        let mut edges = 0_usize;
        for definition in definitions {
            let id = definition.concept.id();
            edges = edges
                .checked_add(definition.cues.len())
                .and_then(|value| value.checked_add(definition.broader.len()))
                .ok_or_else(invalid_catalog)?;
            if edges > 65_536
                || definition.concept.naming_scope() != ConceptNamingScope::Candidate
                || contexts.insert(id, definition.concept).is_some()
            {
                return Err(invalid_catalog());
            }
            for cue in unique_ids(definition.cues)? {
                cues.entry(cue).or_default().push(id);
            }
            broader.insert(id, unique_ids(definition.broader)?);
        }
        if cues.keys().any(|id| contexts.contains_key(id))
            || broader
                .values()
                .flatten()
                .any(|id| !contexts.contains_key(id))
        {
            return Err(invalid_catalog());
        }
        for targets in cues.values_mut() {
            targets.sort_unstable();
        }
        let definition_revisions: Vec<_> =
            contexts.values().map(VocabularyConcept::revision).collect();
        let revision = ClassificationRevision::derive(
            "ame.classification.naming-context-catalog.v1",
            definition_revisions
                .iter()
                .map(|revision| revision.bytes().as_slice())
                .chain(std::iter::once(b"cues".as_slice()))
                .chain(cues.iter().flat_map(|(from, targets)| {
                    targets
                        .iter()
                        .flat_map(move |to| [from.bytes().as_slice(), to.bytes().as_slice()])
                }))
                .chain(std::iter::once(b"broader".as_slice()))
                .chain(broader.iter().flat_map(|(from, targets)| {
                    targets
                        .iter()
                        .flat_map(move |to| [from.bytes().as_slice(), to.bytes().as_slice()])
                })),
        );
        Ok(Self {
            contexts,
            cues,
            broader,
            revision,
        })
    }

    pub fn revision(&self) -> ClassificationRevision {
        self.revision
    }
}

pub struct ContextualVocabulary<'a, V> {
    source: V,
    source_revision: ClassificationRevision,
    catalog: NamingContextCatalog,
    cancelled: &'a AtomicBool,
}

impl<'a, V: ClassificationVocabulary> ContextualVocabulary<'a, V> {
    pub fn new(source: V, catalog: NamingContextCatalog, cancelled: &'a AtomicBool) -> Self {
        let source_revision = source.revision();
        Self {
            source,
            source_revision,
            catalog,
            cancelled,
        }
    }
}

impl<V: ClassificationVocabulary> ClassificationVocabulary for ContextualVocabulary<'_, V> {
    fn revision(&self) -> ClassificationRevision {
        ClassificationRevision::derive(
            "ame.classification.contextual-vocabulary.v1",
            [
                self.source.revision().bytes().as_slice(),
                self.catalog.revision.bytes().as_slice(),
            ],
        )
    }

    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> Result<ConceptPage, ScanError> {
        check_cancelled(self.cancelled)?;
        check_revision(&self.source, self.source_revision)?;
        if !(1..=64).contains(&limit) {
            return Err(super::invalid_page());
        }
        let (source_neighborhood, additions) = match neighborhood {
            ConceptNeighborhood::Broader(id) if self.catalog.contexts.contains_key(&id) => {
                (None, self.catalog.broader.get(&id))
            }
            ConceptNeighborhood::Related(id) if self.catalog.contexts.contains_key(&id) => {
                (None, None)
            }
            ConceptNeighborhood::Broader(id) => (Some(neighborhood), self.catalog.cues.get(&id)),
            _ => (Some(neighborhood), None),
        };
        let mut rows = BTreeMap::new();
        let mut has_more = false;
        if let Some(neighborhood) = source_neighborhood {
            let page = read_page(
                &mut self.source,
                self.source_revision,
                neighborhood,
                after,
                limit,
                self.cancelled,
            )?;
            has_more = page.has_more;
            for concept in page.concepts {
                if self.catalog.contexts.contains_key(&concept.id()) {
                    return Err(ScanError::new(
                        "classification_context_identity_conflict",
                        "Retrieval cues and naming contexts must have separate identities",
                    ));
                }
                let concept = concept.with_naming_scope(ConceptNamingScope::TraversalOnly);
                rows.insert(concept.id(), concept);
            }
        }
        if let Some(additions) = additions {
            let start = after
                .map(|after| additions.partition_point(|id| *id <= after))
                .unwrap_or(0);
            for id in additions[start..].iter().take(limit + 1) {
                rows.insert(*id, self.catalog.contexts[id].clone());
            }
        }
        has_more |= rows.len() > limit;
        check_cancelled(self.cancelled)?;
        check_revision(&self.source, self.source_revision)?;
        Ok(ConceptPage {
            concepts: rows.into_values().take(limit).collect(),
            has_more,
        })
    }
}

fn unique_ids(ids: Vec<ConceptId>) -> Result<Vec<ConceptId>, ScanError> {
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    if ids.len() != unique.len() {
        return Err(invalid_catalog());
    }
    Ok(unique.into_iter().collect())
}

fn invalid_catalog() -> ScanError {
    ScanError::new(
        "classification_context_catalog_invalid",
        "Naming contexts require bounded unique definitions and valid context relations",
    )
}
