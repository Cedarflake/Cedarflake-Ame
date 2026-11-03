use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationAction, ClassificationObservation, ClassificationSubject, HumanCategoryChoice,
    HumanClassification, HumanDecisionOrigin, ReviewRecommendation,
};
use crate::domain::classification_catalog::PreparedClassificationSelection;

use super::{codec, operations::field};

pub(super) fn digest(
    prepared: &PreparedClassificationSelection,
    action: &ClassificationAction,
) -> Result<String, ScanError> {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.command.v1");
    match action {
        ClassificationAction::Confirm => field(&mut hash, b"confirm"),
        ClassificationAction::Correct(category) => {
            field(&mut hash, b"correct");
            field(&mut hash, category.as_str().as_bytes());
        }
        ClassificationAction::Defer => field(&mut hash, b"defer"),
    }
    field(&mut hash, &(prepared.selections.len() as u64).to_le_bytes());
    for selection in &prepared.selections {
        field(&mut hash, selection.asset_id.as_bytes());
        field(&mut hash, selection.location_id.as_bytes());
        field(&mut hash, &selection.source_generation.to_le_bytes());
    }
    field(&mut hash, &(prepared.groups.len() as u64).to_le_bytes());
    for group in &prepared.groups {
        observation(&mut hash, group)?;
    }
    Ok(hash.finalize().to_hex().to_string())
}

fn observation(
    hash: &mut blake3::Hasher,
    group: &ClassificationObservation,
) -> Result<(), ScanError> {
    match &group.target.subject {
        ClassificationSubject::Exact(content) => {
            field(hash, b"exact");
            field(hash, content.scheme.as_str().as_bytes());
            field(hash, &content.digest);
            field(hash, &content.byte_length.to_le_bytes());
        }
        ClassificationSubject::Provisional(content) => {
            field(hash, b"provisional");
            field(hash, content.asset_id().as_bytes());
            field(hash, &content.source_generation().to_le_bytes());
        }
    }
    for revision in [
        group.target.membership,
        group.authority.category,
        group.authority.review,
        group.rules,
    ] {
        field(hash, revision.bytes());
    }
    field(
        hash,
        &[
            u8::from(group.configuration.is_some()),
            u8::from(group.deferred),
        ],
    );
    if let Some(configuration) = group.configuration {
        field(hash, configuration.bytes());
    }
    match &group.human {
        HumanClassification::Unreviewed => field(hash, b"unreviewed"),
        HumanClassification::Conflict => field(hash, b"conflict"),
        HumanClassification::Chosen { category, origin } => {
            field(hash, b"chosen");
            field(hash, category.as_str().as_bytes());
            field(
                hash,
                match origin {
                    HumanDecisionOrigin::Local => b"local",
                    HumanDecisionOrigin::Restored => b"restored",
                },
            );
        }
    }
    field(hash, &[u8::from(group.prediction.is_some())]);
    if let Some(proposal) = &group.prediction {
        field(
            hash,
            codec::encode_choice(&HumanCategoryChoice::Confirmed(proposal.evidence.clone()))?
                .as_bytes(),
        );
        field(
            hash,
            match proposal.review {
                ReviewRecommendation::Uncalibrated => b"uncalibrated",
                ReviewRecommendation::LowSeparation => b"low_separation",
                ReviewRecommendation::RoutineSampling => b"routine_sampling",
            },
        );
    }
    Ok(())
}
