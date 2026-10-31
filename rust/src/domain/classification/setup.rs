use super::{ClassificationError, ClassificationRun, ScreeningProfile, Taxonomy};

#[derive(Clone, Debug, PartialEq)]
pub struct ClassificationAnalysisSetup {
    run: ClassificationRun,
    primary: Taxonomy,
    refinement: Option<(Taxonomy, ScreeningProfile)>,
}

impl ClassificationAnalysisSetup {
    pub fn new(
        run: ClassificationRun,
        primary: Taxonomy,
        refinement: Option<(Taxonomy, ScreeningProfile)>,
    ) -> Result<Self, ClassificationError> {
        if primary.revision() != run.primary_rules() {
            return Err(ClassificationError::IncompatibleAnalysisSetup);
        }
        match (&refinement, run.refinement()) {
            (None, None) => {}
            (Some((rules, profile)), Some(identity)) => {
                if rules.revision() != identity.rules
                    || profile.revision() != identity.profile
                    || profile.identity().model != run.model()
                    || profile.identity().primary_rules != primary.revision()
                    || profile.identity().refinement_rules != rules.revision()
                    || primary.revision() == rules.revision()
                    || !profile.matches_categories(primary.categories())
                    || !primary
                        .categories()
                        .iter()
                        .map(|category| category.id())
                        .eq(rules.categories().iter().map(|category| category.id()))
                {
                    return Err(ClassificationError::IncompatibleAnalysisSetup);
                }
            }
            _ => return Err(ClassificationError::IncompatibleAnalysisSetup),
        }
        Ok(Self {
            run,
            primary,
            refinement,
        })
    }

    pub fn run(&self) -> &ClassificationRun {
        &self.run
    }

    pub fn primary(&self) -> &Taxonomy {
        &self.primary
    }

    pub fn refinement(&self) -> Option<(&Taxonomy, &ScreeningProfile)> {
        self.refinement
            .as_ref()
            .map(|(rules, profile)| (rules, profile))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationSetupActivation {
    Enable,
    ReplaceRun,
}
