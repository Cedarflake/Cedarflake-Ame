use crate::domain::ScanError;
use crate::domain::classification::plan_classification_change;
use crate::domain::classification::{
    ClassificationAction, ClassificationObservation, ClassificationReview, ReviewRecommendation,
    Taxonomy,
};
use crate::domain::classification_selection::{
    ClassificationItemReview, ClassificationItemState, ClassificationReviewReason,
};
use crate::domain::organization::{OrganizationSelection, validate_selections};
use rusqlite::Connection;

use super::super::{gallery::classification_annotations, organization};
use super::{configuration, observations};

pub(super) fn can_confirm(observation: &ClassificationObservation, taxonomy: &Taxonomy) -> bool {
    observation.effective().review != ClassificationReview::Confirmed
        && plan_classification_change(
            observation,
            observation,
            &ClassificationAction::Confirm,
            taxonomy,
        )
        .is_ok()
}

pub(super) fn load(
    connection: &Connection,
    selections: &[OrganizationSelection],
) -> Result<Vec<ClassificationItemState>, ScanError> {
    validate_selections(selections)?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(super::super::database_error)?;
    let state = configuration::load(&transaction)?;
    let Some(taxonomy) = state.taxonomy.as_ref() else {
        return Ok(Vec::new());
    };
    let mut result = Vec::with_capacity(selections.len());
    for selection in selections {
        let prepared =
            organization::selection::prepare(&transaction, std::slice::from_ref(selection))?;
        let target = prepared.groups.first().ok_or_else(super::changed)?;
        let observation = observations::for_target(&transaction, target, &state)?;
        result.push(item_state(&transaction, selection, &observation, taxonomy)?);
    }
    transaction.commit().map_err(super::super::database_error)?;
    Ok(result)
}

pub(super) fn item_state(
    connection: &Connection,
    selection: &OrganizationSelection,
    observation: &ClassificationObservation,
    taxonomy: &Taxonomy,
) -> Result<ClassificationItemState, ScanError> {
    let effective = observation.effective();
    Ok(ClassificationItemState {
        asset_id: selection.asset_id.clone(),
        category_id: effective.category.map(|category| category.as_str().into()),
        review: match effective.review {
            ClassificationReview::Unclassified => ClassificationItemReview::Unclassified,
            ClassificationReview::Suggested => ClassificationItemReview::Suggested,
            ClassificationReview::NeedsReview => ClassificationItemReview::NeedsReview,
            ClassificationReview::Deferred => ClassificationItemReview::Deferred,
            ClassificationReview::Confirmed => ClassificationItemReview::Confirmed,
            ClassificationReview::Conflict => ClassificationItemReview::Conflict,
        },
        can_confirm: can_confirm(observation, taxonomy),
        review_reason: match effective.review {
            ClassificationReview::Confirmed => None,
            ClassificationReview::Unclassified => {
                classification_annotations::has_failed_work(connection, selection)?
                    .then_some(ClassificationReviewReason::AnalysisFailed)
            }
            ClassificationReview::Conflict => Some(ClassificationReviewReason::Conflict),
            ClassificationReview::Deferred => Some(ClassificationReviewReason::Deferred),
            ClassificationReview::Suggested | ClassificationReview::NeedsReview => observation
                .compatible_prediction()
                .map(|prediction| match prediction.review {
                    ReviewRecommendation::RoutineSampling => ClassificationReviewReason::Sampling,
                    ReviewRecommendation::Uncalibrated => ClassificationReviewReason::Uncalibrated,
                    ReviewRecommendation::LowSeparation => ClassificationReviewReason::Uncertain,
                }),
        },
    })
}
