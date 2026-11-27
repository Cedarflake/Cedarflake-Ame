use std::collections::BTreeMap;

use crate::domain::ScanError;
use crate::domain::classification::{CategoryDefinition, CategoryId, ClassificationAnalysisSetup};

use super::terms::configuration_for_categories;
use super::{ClassificationConfigurationChange, ClassificationConfigurationSnapshot};

pub struct ClassificationTermsReset {
    baseline: ClassificationAnalysisSetup,
    definitions: BTreeMap<CategoryId, (CategoryDefinition, CategoryDefinition)>,
}

impl ClassificationTermsReset {
    pub fn new(
        baseline: ClassificationAnalysisSetup,
        expected: Vec<CategoryDefinition>,
        defaults: &BTreeMap<CategoryId, CategoryDefinition>,
    ) -> Result<Self, ScanError> {
        if expected.len() > defaults.len() {
            return Err(invalid());
        }
        let mut definitions = BTreeMap::new();
        for expected in expected {
            let id = expected.id().clone();
            if id.as_str() == "other" || id.as_str().starts_with("custom-") {
                return Err(invalid());
            }
            let replacement = defaults.get(&id).ok_or_else(invalid)?.clone();
            if replacement.id() != &id || definitions.insert(id, (expected, replacement)).is_some()
            {
                return Err(invalid());
            }
        }
        Ok(Self {
            baseline,
            definitions,
        })
    }

    pub fn baseline(&self) -> &ClassificationAnalysisSetup {
        &self.baseline
    }

    pub fn definitions(&self) -> impl Iterator<Item = &(CategoryDefinition, CategoryDefinition)> {
        self.definitions.values()
    }

    pub fn changes_matching(&self) -> bool {
        self.definitions().any(|(expected, replacement)| {
            expected.semantic_revision() != replacement.semantic_revision()
        })
    }

    pub fn configuration(
        &self,
        current: &ClassificationConfigurationSnapshot,
        current_setup: Option<&ClassificationAnalysisSetup>,
    ) -> Result<Option<ClassificationConfigurationChange>, ScanError> {
        if self.definitions.is_empty() {
            return Ok(None);
        }
        let taxonomy = current.catalog.taxonomy.as_ref().ok_or_else(changed)?;
        for (expected, _) in self.definitions() {
            let actual = taxonomy
                .categories()
                .iter()
                .find(|category| category.id() == expected.id());
            if actual != Some(expected) {
                return Err(changed());
            }
        }
        let categories = taxonomy
            .categories()
            .iter()
            .map(|category| {
                self.definitions
                    .get(category.id())
                    .map_or_else(|| category.clone(), |(_, replacement)| replacement.clone())
            })
            .collect();
        configuration_for_categories(&self.baseline, current, current_setup, categories).map(Some)
    }
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_term_invalid",
        "Only distinct bundled categories can be restored",
    )
}

fn changed() -> ScanError {
    ScanError::new(
        "classification_term_changed",
        "A selected category changed; reload its current definition",
    )
}
