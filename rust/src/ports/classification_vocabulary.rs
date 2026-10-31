use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;
use crate::domain::classification::discovery::{ConceptId, VocabularyConcept};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConceptNeighborhood {
    EntryPoints,
    Related(ConceptId),
}

pub struct ConceptPage {
    pub concepts: Vec<VocabularyConcept>,
    pub has_more: bool,
}

pub trait ClassificationVocabulary {
    fn revision(&self) -> ClassificationRevision;

    /// Returns a strictly ID-ordered window from an immutable vocabulary revision.
    fn read_concepts(
        &mut self,
        neighborhood: ConceptNeighborhood,
        after: Option<ConceptId>,
        limit: usize,
    ) -> Result<ConceptPage, ScanError>;
}
