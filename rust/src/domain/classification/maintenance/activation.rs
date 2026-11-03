use super::{CategoryExpansionSnapshot, ManagedCategoryOrigin};
use crate::domain::classification::CategoryDefinition;

pub(crate) enum CategoryActivation {
    New,
    Reactivate(ManagedCategoryOrigin),
}

impl CategoryExpansionSnapshot {
    pub(crate) fn reactivation_origin(
        &self,
        category: &CategoryDefinition,
    ) -> Option<&ManagedCategoryOrigin> {
        if self.known_categories.get(category.id()).map(String::as_str) != Some(category.label())
            || self
                .configuration
                .catalog
                .taxonomy
                .as_ref()
                .is_some_and(|rules| {
                    rules
                        .categories()
                        .iter()
                        .any(|active| active.id() == category.id())
                })
        {
            return None;
        }
        self.managed_categories.get(category.id()).filter(|origin| {
            !origin.protected
                && origin.admitted_label == category.label()
                && origin.admitted_semantic_revision == category.semantic_revision()
        })
    }
}
