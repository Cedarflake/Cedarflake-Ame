use crate::domain::ScanError;
use crate::domain::similar_images::SimilarContentKey;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarReviewMember, SimilarReviewMembers, SimilarReviewPage,
    SimilarReviewRange, SimilarUndoResult, SimilarityDegree,
};
use crate::ports::similar_images::review::SimilarReviewRepository;

use super::production;
use crate::application::exact_duplicates::{
    inspect_candidate_availability, validate_digest, validate_identifier,
};

pub(crate) fn groups(
    degree: SimilarityDegree,
    ignored: bool,
    after: Option<String>,
) -> Result<SimilarReviewPage, ScanError> {
    let degree = if ignored {
        SimilarityDegree::Broad
    } else {
        degree
    };
    if let Some(after) = &after {
        validate_digest(after)?;
    }
    production::repository()?.load_similar_groups(degree, ignored, after.as_deref().unwrap_or(""))
}

pub(crate) fn seek(
    degree: SimilarityDegree,
    ignored: bool,
    index: u64,
) -> Result<SimilarReviewRange, ScanError> {
    let degree = if ignored {
        SimilarityDegree::Broad
    } else {
        degree
    };
    production::repository()?.seek_similar_groups(degree, ignored, index)
}

pub(crate) fn members(
    group_id: String,
    degree: SimilarityDegree,
    after: Option<SimilarContentKey>,
) -> Result<SimilarReviewMembers, ScanError> {
    validate_digest(&group_id)?;
    if let Some(after) = &after {
        after.validate()?;
    }
    let repository = production::repository()?;
    let page = repository.load_similar_members(&group_id, degree, after.as_ref())?;
    drop(repository);
    let members = page
        .members
        .into_iter()
        .map(|member| {
            let availability = inspect_candidate_availability(&member.candidate);
            SimilarReviewMember {
                preferred: member.preferred,
                content: member.content,
                representative: member.candidate.location,
                availability,
                location_count: member.location_count,
                distance: member.distance,
            }
        })
        .collect();
    Ok(SimilarReviewMembers {
        members,
        total_members: page.total_members,
        next: page.next,
        decision: page.decision,
    })
}

pub(crate) fn prefer(
    context: SimilarReviewContext,
    selection: SimilarMemberSelection,
    preferred: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    validate_digest(&context.group_id)?;
    validate_selection(&selection)?;
    production::repository()?.prefer_similar_member(&context, &selection, preferred)
}

pub(crate) fn ignore(
    context: SimilarReviewContext,
    ignored: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    validate_digest(&context.group_id)?;
    production::repository()?.ignore_similar_group(&context, ignored)
}

pub(crate) fn exclude(
    context: SimilarReviewContext,
    selection: SimilarMemberSelection,
    exclusion_id: String,
) -> Result<SimilarExclusionResult, ScanError> {
    validate_digest(&context.group_id)?;
    validate_selection(&selection)?;
    validate_exclusion_id(&exclusion_id)?;
    production::repository()?.exclude_similar_member(&context, &selection, &exclusion_id)
}

pub(crate) fn undo(exclusion: SimilarMemberExclusion) -> Result<SimilarUndoResult, ScanError> {
    validate_digest(&exclusion.group_id)?;
    exclusion.content.validate()?;
    validate_exclusion_id(&exclusion.exclusion_id)?;
    production::repository()?.undo_similar_exclusion(&exclusion)
}

fn validate_selection(selection: &SimilarMemberSelection) -> Result<(), ScanError> {
    selection.content.validate()?;
    validate_identifier(&selection.source.asset_id)?;
    validate_identifier(&selection.source.location_id)
}

fn validate_exclusion_id(value: &str) -> Result<(), ScanError> {
    if value.is_empty() || value.len() > 128 || value.contains('\0') {
        return Err(ScanError::new(
            "similarity_exclusion_invalid",
            "Member exclusion requires a bounded operation identifier",
        ));
    }
    Ok(())
}
