use super::{ConceptNamingScope, DiscoveryError, VocabularyConcept};
use crate::domain::classification::{
    CategoryDefinition, CategoryId, ClassificationRevision, Taxonomy,
};

const SUBJECT: &str = "{subject}";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CategoryNaming {
    Unconditioned,
    WithinCategory(CategoryNamingPurpose),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryNamingPurpose {
    category: CategoryId,
    category_revision: ClassificationRevision,
    label_pattern: String,
    comparison_pattern: String,
}

impl CategoryNamingPurpose {
    pub fn new(
        category: &CategoryDefinition,
        label_pattern: String,
        comparison_pattern: String,
    ) -> Result<Self, DiscoveryError> {
        Ok(Self {
            category: category.id().clone(),
            category_revision: category.semantic_revision(),
            label_pattern: validated_pattern(label_pattern, 80)?,
            comparison_pattern: validated_pattern(comparison_pattern, 1024)?,
        })
    }

    pub fn category(&self) -> &CategoryId {
        &self.category
    }

    pub fn category_revision(&self) -> ClassificationRevision {
        self.category_revision
    }
}

impl CategoryNaming {
    pub fn revision(&self) -> ClassificationRevision {
        match self {
            Self::Unconditioned => ClassificationRevision::derive(
                "ame.classification.category-naming.unconditioned.v1",
                std::iter::empty(),
            ),
            Self::WithinCategory(purpose) => ClassificationRevision::derive(
                "ame.classification.category-naming.purpose.v1",
                [
                    purpose.category.as_str().as_bytes(),
                    purpose.category_revision.bytes().as_slice(),
                    purpose.label_pattern.as_bytes(),
                    purpose.comparison_pattern.as_bytes(),
                ],
            ),
        }
    }

    pub(crate) fn validate(&self, taxonomy: &Taxonomy) -> Result<(), DiscoveryError> {
        if let Self::WithinCategory(purpose) = self {
            let current = taxonomy
                .categories()
                .iter()
                .find(|category| category.id() == &purpose.category);
            if current
                .is_none_or(|category| category.semantic_revision() != purpose.category_revision)
            {
                return Err(DiscoveryError::NamingPurposeChanged);
            }
        }
        Ok(())
    }

    pub(crate) fn includes(&self, category: &CategoryId) -> bool {
        match self {
            Self::Unconditioned => true,
            Self::WithinCategory(purpose) => &purpose.category == category,
        }
    }

    pub(crate) fn project(&self, concept: &VocabularyConcept) -> VocabularyConcept {
        let Self::WithinCategory(purpose) = self else {
            return concept.clone();
        };
        let mut projected = concept.clone();
        if concept.naming_scope() == ConceptNamingScope::TraversalOnly {
            return projected;
        }
        let Some(subject) = concept.subject() else {
            return projected.with_naming_scope(ConceptNamingScope::TraversalOnly);
        };
        let label = purpose.label_pattern.replace(SUBJECT, concept.label());
        let description = purpose.comparison_pattern.replace(SUBJECT, subject);
        if label.chars().count() > 80 || description.chars().count() > 1024 {
            return projected.with_naming_scope(ConceptNamingScope::TraversalOnly);
        }
        projected.label = label;
        projected.description = description;
        projected
    }

    pub(crate) fn admits_description(&self, concept: &VocabularyConcept) -> bool {
        match self {
            Self::Unconditioned => true,
            Self::WithinCategory(purpose) => concept.subject().is_some_and(|subject| {
                concept.description() == purpose.comparison_pattern.replace(SUBJECT, subject)
            }),
        }
    }
}

fn validated_pattern(pattern: String, limit: usize) -> Result<String, DiscoveryError> {
    let pattern = pattern.trim();
    let without_subject = pattern.replace(SUBJECT, "");
    if pattern.matches(SUBJECT).count() != 1
        || without_subject.trim().is_empty()
        || without_subject.contains(['{', '}'])
        || pattern.chars().any(char::is_control)
        || pattern.chars().count() > limit
    {
        return Err(DiscoveryError::InvalidNamingPurpose);
    }
    Ok(pattern.to_owned())
}

#[cfg(test)]
mod tests;
