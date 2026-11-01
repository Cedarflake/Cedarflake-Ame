use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::{
    ConceptId, ConceptNamingScope, NamingContext, VocabularyConcept,
};
use crate::ports::classification_vocabulary::{
    ClassificationVocabulary, ConceptNeighborhood, ConceptPage,
};

mod resource;

pub struct LocalClassificationVocabulary {
    concepts: BTreeMap<ConceptId, VocabularyConcept>,
    entries: Vec<ConceptId>,
    related: BTreeMap<ConceptId, Vec<ConceptId>>,
    broader: BTreeMap<ConceptId, Vec<ConceptId>>,
    revision: ClassificationRevision,
}

impl LocalClassificationVocabulary {
    pub fn open(
        path: &Path,
        expected_digest: &[u8; 32],
        cancelled: &AtomicBool,
    ) -> Result<(Self, Vec<NamingContext>), ScanError> {
        let document = resource::load(path, expected_digest, cancelled)?;
        let namespace = &document.source_namespace;
        let mut concepts = BTreeMap::new();
        let mut related = BTreeMap::new();
        let mut broader = BTreeMap::new();
        let mut edge_count = 0;
        for entry in document.concepts {
            check_cancelled(cancelled)?;
            let id = resource::identity(namespace, &entry.id)?;
            let concept = entry
                .definition
                .into_concept(id, ConceptNamingScope::TraversalOnly)?;
            if concepts.insert(id, concept).is_some() {
                return Err(invalid());
            }
            let neighbors = references(namespace, entry.related, &mut edge_count)?;
            let parents = references(namespace, entry.broader, &mut edge_count)?;
            related.insert(id, neighbors);
            broader.insert(id, parents);
        }
        let entries = references(namespace, document.entries, &mut edge_count)?;
        for id in entries
            .iter()
            .chain(related.values().flatten())
            .chain(broader.values().flatten())
        {
            check_cancelled(cancelled)?;
            if !concepts.contains_key(id) {
                return Err(invalid());
            }
        }
        let contexts = resource::contexts(document.contexts, namespace, &concepts, cancelled)?;
        let mut vocabulary = Self {
            concepts,
            entries,
            related,
            broader,
            revision: ClassificationRevision::from_bytes([0; 32]),
        };
        vocabulary.revision = vocabulary.compute_revision(cancelled)?;
        Ok((vocabulary, contexts))
    }

    fn compute_revision(
        &self,
        cancelled: &AtomicBool,
    ) -> Result<ClassificationRevision, ScanError> {
        let mut hash = blake3::Hasher::new_derive_key("ame.classification.local-vocabulary.v1");
        for concept in self.concepts.values() {
            check_cancelled(cancelled)?;
            hash.update(concept.revision().bytes());
        }
        hash.update(b"entries");
        for id in &self.entries {
            hash.update(id.bytes());
        }
        for (kind, relations) in [
            (b"related".as_slice(), &self.related),
            (b"broader", &self.broader),
        ] {
            hash.update(kind);
            for (from, targets) in relations {
                check_cancelled(cancelled)?;
                hash.update(from.bytes());
                hash.update(&(targets.len() as u64).to_le_bytes());
                for to in targets {
                    hash.update(to.bytes());
                }
            }
        }
        check_cancelled(cancelled)?;
        Ok(ClassificationRevision::from_bytes(
            *hash.finalize().as_bytes(),
        ))
    }
}

impl ClassificationVocabulary for LocalClassificationVocabulary {
    fn revision(&self) -> ClassificationRevision {
        self.revision
    }

    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> Result<ConceptPage, ScanError> {
        if !(1..=64).contains(&limit) {
            return Err(invalid());
        }
        let ids = match neighborhood {
            ConceptNeighborhood::EntryPoints => self.entries.as_slice(),
            ConceptNeighborhood::Related(id) => {
                self.related.get(&id).map(Vec::as_slice).unwrap_or(&[])
            }
            ConceptNeighborhood::Broader(id) => {
                self.broader.get(&id).map(Vec::as_slice).unwrap_or(&[])
            }
        };
        let start = after
            .map(|id| ids.partition_point(|candidate| *candidate <= id))
            .unwrap_or(0);
        let end = start.saturating_add(limit).min(ids.len());
        Ok(ConceptPage {
            concepts: ids[start..end]
                .iter()
                .map(|id| self.concepts[id].clone())
                .collect(),
            has_more: end < ids.len(),
        })
    }
}

fn references(
    namespace: &str,
    values: Vec<String>,
    total: &mut usize,
) -> Result<Vec<ConceptId>, ScanError> {
    *total = total.checked_add(values.len()).ok_or_else(invalid)?;
    if values.len() > 16_384 || *total > 1_048_576 {
        return Err(invalid());
    }
    let count = values.len();
    let ids = values
        .iter()
        .map(|value| resource::identity(namespace, value))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if ids.len() != count {
        return Err(invalid());
    }
    Ok(ids.into_iter().collect())
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

fn invalid() -> ScanError {
    ScanError::new(
        "classification_vocabulary_invalid",
        "The classification vocabulary resource is invalid",
    )
}

#[cfg(test)]
mod tests;
