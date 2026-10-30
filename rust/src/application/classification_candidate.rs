use std::sync::OnceLock;

use serde::Deserialize;

use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScreeningThresholds, ClassificationAnalysisSetup,
    ClassificationError, ClassificationRecordId, ClassificationRevision, ClassificationRun,
    ModelCompatibility, RefinementIdentity, ScreeningIdentity, ScreeningProfile,
    ScreeningThresholds, Taxonomy,
};

#[derive(Deserialize)]
struct Candidate {
    model: CandidateModel,
    calibration: [u8; 32],
    categories: Vec<CandidateCategory>,
}

#[derive(Deserialize)]
struct CandidateModel {
    embedding_space: [u8; 32],
    image_pipeline: [u8; 32],
    text_pipeline: [u8; 32],
    dimensions: usize,
}

#[derive(Deserialize)]
struct CandidateCategory {
    id: String,
    label: String,
    primary: Vec<String>,
    refinement: Vec<String>,
    rescreen_below: f32,
    review_below: Option<f32>,
}

fn candidate() -> &'static Candidate {
    static CANDIDATE: OnceLock<Candidate> = OnceLock::new();
    CANDIDATE.get_or_init(|| {
        serde_json::from_str(include_str!(
            "classification_candidate/initial_screening.json"
        ))
        .expect("the bundled screening candidate is valid")
    })
}

pub(crate) fn initial_categories() -> Result<Taxonomy, ClassificationError> {
    Taxonomy::new(
        candidate()
            .categories
            .iter()
            .map(|category| {
                CategoryDefinition::new(
                    CategoryId::new(category.id.clone())?,
                    category.label.clone(),
                    category.primary.clone(),
                )
            })
            .collect::<Result<_, _>>()?,
    )
}

pub(crate) fn prepare(
    id: ClassificationRecordId,
    primary: Taxonomy,
    model: ModelCompatibility,
) -> Result<ClassificationAnalysisSetup, ClassificationError> {
    let candidate = candidate();
    let admitted_model = ModelCompatibility::new(
        ClassificationRevision::from_bytes(candidate.model.embedding_space),
        ClassificationRevision::from_bytes(candidate.model.image_pipeline),
        ClassificationRevision::from_bytes(candidate.model.text_pipeline),
        candidate.model.dimensions,
    )?;
    let refinement =
        if model == admitted_model && primary.revision() == initial_categories()?.revision() {
            let rules = Taxonomy::new(
                candidate
                    .categories
                    .iter()
                    .map(|category| {
                        CategoryDefinition::new(
                            CategoryId::new(category.id.clone())?,
                            category.label.clone(),
                            category.refinement.clone(),
                        )
                    })
                    .collect::<Result<_, _>>()?,
            )?;
            let thresholds = candidate
                .categories
                .iter()
                .map(|category| {
                    let thresholds = match category.review_below {
                        Some(review) => ScreeningThresholds::new(category.rescreen_below, review)?,
                        None => ScreeningThresholds::without_review_calibration(
                            category.rescreen_below,
                        )?,
                    };
                    Ok(CategoryScreeningThresholds {
                        category_id: CategoryId::new(category.id.clone())?,
                        thresholds,
                    })
                })
                .collect::<Result<Vec<_>, ClassificationError>>()?;
            let profile = ScreeningProfile::for_categories(
                ScreeningIdentity {
                    model,
                    primary_rules: primary.revision(),
                    refinement_rules: rules.revision(),
                },
                ClassificationRevision::from_bytes(candidate.calibration),
                thresholds,
            )?;
            Some((rules, profile))
        } else {
            None
        };
    let identity = refinement
        .as_ref()
        .map(|(rules, profile)| RefinementIdentity {
            rules: rules.revision(),
            profile: profile.revision(),
        });
    let run = ClassificationRun::new(id, model, primary.revision(), identity)?;
    ClassificationAnalysisSetup::new(run, primary, refinement)
}

#[cfg(test)]
mod tests;
