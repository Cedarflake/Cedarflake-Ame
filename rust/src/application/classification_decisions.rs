use crate::domain::classification::{
    ClassificationAction, ClassificationChange, ClassificationError, ClassificationObservation,
    HumanCategoryChoice, HumanClassification, HumanDecisionOrigin, Taxonomy,
};

pub fn plan_classification_change(
    observed: &ClassificationObservation,
    current: &ClassificationObservation,
    action: &ClassificationAction,
    taxonomy: &Taxonomy,
) -> Result<ClassificationChange, ClassificationError> {
    if observed.target != current.target || observed.authority != current.authority {
        return Err(ClassificationError::ClassificationTargetChanged);
    }
    match action {
        ClassificationAction::Confirm => {
            require_current_rules(observed, current, taxonomy)?;
            confirm(observed, current, taxonomy)
        }
        ClassificationAction::Correct(category) => {
            require_current_rules(observed, current, taxonomy)?;
            require_category(taxonomy, category)?;
            if current.human
                == (HumanClassification::Chosen {
                    category: category.clone(),
                    origin: HumanDecisionOrigin::Local,
                })
                && !current.deferred
            {
                return Ok(ClassificationChange::Unchanged);
            }
            Ok(ClassificationChange::Choose(
                HumanCategoryChoice::Corrected {
                    category: category.clone(),
                    rules: taxonomy.revision(),
                },
            ))
        }
        ClassificationAction::Defer => {
            if current.deferred || matches!(current.human, HumanClassification::Chosen { .. }) {
                return Ok(ClassificationChange::Unchanged);
            }
            Ok(ClassificationChange::Defer)
        }
    }
}

fn require_current_rules(
    observed: &ClassificationObservation,
    current: &ClassificationObservation,
    taxonomy: &Taxonomy,
) -> Result<(), ClassificationError> {
    if observed.rules != current.rules || current.rules != taxonomy.revision() {
        Err(ClassificationError::ClassificationRulesChanged)
    } else {
        Ok(())
    }
}

fn confirm(
    observed: &ClassificationObservation,
    current: &ClassificationObservation,
    taxonomy: &Taxonomy,
) -> Result<ClassificationChange, ClassificationError> {
    if current.human == HumanClassification::Conflict {
        return Err(ClassificationError::ClassificationConflictUnresolved);
    }
    let displayed = observed
        .compatible_prediction()
        .ok_or(ClassificationError::NoCompatiblePrediction)?;
    let available = current
        .compatible_prediction()
        .ok_or(ClassificationError::NoCompatiblePrediction)?;
    if displayed != available {
        return Err(ClassificationError::ClassificationPredictionChanged);
    }
    require_category(taxonomy, &available.evidence.category)?;
    if let HumanClassification::Chosen { category, origin } = &current.human {
        if category != &available.evidence.category {
            return Err(ClassificationError::HumanChoiceRequiresCorrection);
        }
        if *origin == HumanDecisionOrigin::Local {
            return Ok(ClassificationChange::Unchanged);
        }
    }
    Ok(ClassificationChange::Choose(
        HumanCategoryChoice::Confirmed(available.evidence.clone()),
    ))
}

fn require_category(
    taxonomy: &Taxonomy,
    category: &crate::domain::classification::CategoryId,
) -> Result<(), ClassificationError> {
    if taxonomy
        .categories()
        .iter()
        .any(|definition| definition.id() == category)
    {
        Ok(())
    } else {
        Err(ClassificationError::CategoryRetired)
    }
}

#[cfg(test)]
mod tests;
