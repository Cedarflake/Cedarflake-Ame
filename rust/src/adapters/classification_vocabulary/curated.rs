use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;

use serde::Deserialize;

use crate::domain::ScanError;
use crate::domain::classification::discovery::{
    ConceptId, ConceptPurposeLabels, NamingContext, VocabularyConcept,
};
use crate::domain::classification::maintenance::{
    ManagedCategoryCatalog, managed_category_identity,
};
use crate::domain::classification::{CategoryId, ClassificationRevision};
use crate::ports::classification_vocabulary::{
    ClassificationVocabulary, ConceptNeighborhood, ConceptPage, ManagedClassificationVocabulary,
};

use super::{check_cancelled, invalid, resource};

const CATALOG: &str = include_str!("../../../../resources/classification/curated-topics.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    label: String,
    subject: String,
    purposes: Vec<PurposeLabel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PurposeLabel {
    id: String,
    label: String,
}

pub struct CuratedClassificationVocabulary {
    concepts: BTreeMap<ConceptId, VocabularyConcept>,
    revision: ClassificationRevision,
    managed_catalog: ManagedCategoryCatalog,
}

impl CuratedClassificationVocabulary {
    pub fn bundled(cancelled: &AtomicBool) -> Result<Self, ScanError> {
        Self::parse(CATALOG, cancelled)
    }

    fn parse(document: &str, cancelled: &AtomicBool) -> Result<Self, ScanError> {
        check_cancelled(cancelled)?;
        if document.len() > 128 * 1024 {
            return Err(invalid());
        }
        let topics: Vec<Topic> = serde_json::from_str(document).map_err(|_| invalid())?;
        if topics.is_empty() || topics.len() > 64 {
            return Err(invalid());
        }
        let mut concepts = BTreeMap::new();
        let mut categories = BTreeSet::new();
        for topic in topics {
            check_cancelled(cancelled)?;
            resource::validate_key(&topic.id)?;
            let id =
                resource::identity("ame.managed-naming-context", &format!("topic-{}", topic.id))?;
            let purposes = topic
                .purposes
                .into_iter()
                .map(|purpose| {
                    Ok((
                        CategoryId::new(purpose.id).map_err(|_| invalid())?,
                        purpose.label,
                    ))
                })
                .collect::<Result<Vec<_>, ScanError>>()?;
            for (purpose, _) in &purposes {
                categories
                    .insert(managed_category_identity(id, Some(purpose)).map_err(|_| invalid())?);
            }
            let concept =
                VocabularyConcept::new(id, topic.label, format!("An image of {}.", topic.subject))
                    .and_then(|concept| concept.with_subject(topic.subject))
                    .map_err(|_| invalid())?
                    .with_purpose_labels(
                        ConceptPurposeLabels::new(purposes).map_err(|_| invalid())?,
                    );
            if concepts.insert(id, concept).is_some() {
                return Err(invalid());
            }
        }
        let revisions: Vec<_> = concepts.values().map(VocabularyConcept::revision).collect();
        let revision = ClassificationRevision::derive(
            "ame.classification.curated-topics.v1",
            revisions.iter().map(|revision| revision.bytes().as_slice()),
        );
        check_cancelled(cancelled)?;
        let managed_catalog = ManagedCategoryCatalog::new(revision, categories)?;
        Ok(Self {
            concepts,
            revision,
            managed_catalog,
        })
    }
}

impl ManagedClassificationVocabulary for CuratedClassificationVocabulary {
    fn managed_catalog(&self) -> &ManagedCategoryCatalog {
        &self.managed_catalog
    }

    fn naming_contexts(&self) -> Vec<NamingContext> {
        self.concepts
            .values()
            .map(|concept| NamingContext {
                concept: concept.clone(),
                cues: Vec::new(),
                broader: Vec::new(),
            })
            .collect()
    }
}

impl ClassificationVocabulary for CuratedClassificationVocabulary {
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
        if neighborhood != ConceptNeighborhood::EntryPoints {
            return Ok(ConceptPage {
                concepts: Vec::new(),
                has_more: false,
            });
        }
        let mut rows = self
            .concepts
            .values()
            .filter(|concept| after.is_none_or(|after| concept.id() > after));
        let concepts = rows.by_ref().take(limit).cloned().collect();
        Ok(ConceptPage {
            concepts,
            has_more: rows.next().is_some(),
        })
    }
}

#[cfg(test)]
mod tests;
