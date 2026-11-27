use super::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
    ClassificationConfigurationSnapshot,
};
use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryDefinition, ClassificationAnalysisSetup, ClassificationRun, RefinementIdentity,
    Taxonomy,
};

pub struct ClassificationTermChange {
    pub expected: Option<CategoryDefinition>,
    pub replacement: Option<CategoryDefinition>,
    pub baseline: ClassificationAnalysisSetup,
}

impl ClassificationTermChange {
    pub fn configuration(
        &self,
        current: &ClassificationConfigurationSnapshot,
        current_setup: Option<&ClassificationAnalysisSetup>,
    ) -> Result<ClassificationConfigurationChange, ScanError> {
        let selected = self
            .expected
            .as_ref()
            .or(self.replacement.as_ref())
            .ok_or_else(invalid)?;
        if selected.id().as_str() == "other" {
            return Err(invalid());
        }
        match (&self.expected, &self.replacement) {
            (Some(expected), Some(replacement)) if expected.id() != replacement.id() => {
                return Err(invalid());
            }
            (None, Some(_)) | (Some(_), None) if !selected.id().as_str().starts_with("custom-") => {
                return Err(invalid());
            }
            _ => {}
        }
        let mut categories = current
            .catalog
            .taxonomy
            .as_ref()
            .map(|taxonomy| taxonomy.categories().to_vec())
            .unwrap_or_else(|| self.baseline.primary().categories().to_vec());
        let position = categories
            .iter()
            .position(|category| category.id() == selected.id());
        if position.map(|index| &categories[index]) != self.expected.as_ref() {
            return Err(ScanError::new(
                "classification_term_changed",
                "The selected category changed; reload its current definition",
            ));
        }
        match (position, &self.replacement) {
            (Some(index), Some(replacement)) => categories[index] = replacement.clone(),
            (Some(index), None) => {
                categories.remove(index);
            }
            (None, Some(replacement)) => categories.push(replacement.clone()),
            (None, None) => return Err(invalid()),
        }
        let taxonomy = Taxonomy::new(categories).map_err(configuration_invalid)?;
        let model = current_setup.map_or(self.baseline.run().model(), |setup| setup.run().model());
        let basis = current_setup
            .filter(|setup| setup.primary().revision() == taxonomy.revision())
            .unwrap_or(&self.baseline);
        let refinement =
            if taxonomy.revision() == basis.primary().revision() && model == basis.run().model() {
                basis
                    .refinement()
                    .map(|(rules, profile)| (rules.clone(), profile.clone()))
            } else {
                None
            };
        let refinement_identity = refinement
            .as_ref()
            .map(|(rules, profile)| RefinementIdentity {
                rules: rules.revision(),
                profile: profile.revision(),
            });
        let run = ClassificationRun::new(
            self.baseline.run().id().clone(),
            model,
            taxonomy.revision(),
            refinement_identity,
        )
        .map_err(configuration_invalid)?;
        Ok(ClassificationConfigurationChange {
            operation_id: run.id().clone(),
            expected_catalog_revision: current.catalog.revision,
            expected_dispatch_revision: current.dispatch.revision,
            setup: ClassificationAnalysisSetup::new(run, taxonomy, refinement)
                .map_err(configuration_invalid)?,
            action: ClassificationConfigurationAction::Save,
        })
    }

    pub fn changes_matching(&self) -> bool {
        self.expected
            .as_ref()
            .map(CategoryDefinition::semantic_revision)
            != self
                .replacement
                .as_ref()
                .map(CategoryDefinition::semantic_revision)
    }
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_term_invalid",
        "The category edit is not permitted",
    )
}

fn configuration_invalid(error: impl std::fmt::Display) -> ScanError {
    ScanError::new("classification_configuration_invalid", error.to_string())
}
