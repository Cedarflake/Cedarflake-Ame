use serde::{Deserialize, Serialize};

use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationFieldAuthority, ClassificationObservation, ClassificationProposal,
    ClassificationRevision, ClassificationSubject, ClassificationTarget, HumanCategoryChoice,
    HumanClassification, HumanDecisionOrigin, ProvisionalClassificationContent,
    ReviewRecommendation,
};

use super::{
    ContentRecord, category_id, decode, decode_choice, encode, encode_choice, invalid_record,
};

const LIMIT: usize = 16384;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ObservationRecord {
    subject: SubjectRecord,
    membership: [u8; 32],
    category_authority: [u8; 32],
    review_authority: [u8; 32],
    rules: [u8; 32],
    configuration: Option<[u8; 32]>,
    human: HumanRecord,
    deferred: bool,
    prediction: Option<ProposalRecord>,
}

#[derive(Deserialize, Serialize)]
enum SubjectRecord {
    Exact(ContentRecord),
    Provisional { asset_id: String, generation: u64 },
}

#[derive(Deserialize, Serialize)]
enum HumanRecord {
    Unreviewed,
    Chosen { category: String, restored: bool },
    Conflict,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProposalRecord {
    evidence: String,
    review: ReviewRecord,
}

#[derive(Deserialize, Serialize)]
enum ReviewRecord {
    Uncalibrated,
    LowSeparation,
    RoutineSampling,
}

pub(in super::super) fn save(value: &ClassificationObservation) -> Result<String, ScanError> {
    let subject = match &value.target.subject {
        ClassificationSubject::Exact(content) => SubjectRecord::Exact(ContentRecord::from(content)),
        ClassificationSubject::Provisional(content) => SubjectRecord::Provisional {
            asset_id: content.asset_id().into(),
            generation: content.source_generation(),
        },
    };
    let human = match &value.human {
        HumanClassification::Unreviewed => HumanRecord::Unreviewed,
        HumanClassification::Chosen { category, origin } => HumanRecord::Chosen {
            category: category.as_str().into(),
            restored: *origin == HumanDecisionOrigin::Restored,
        },
        HumanClassification::Conflict => HumanRecord::Conflict,
    };
    let prediction = value
        .prediction
        .as_ref()
        .map(|proposal| {
            Ok::<_, ScanError>(ProposalRecord {
                evidence: encode_choice(&HumanCategoryChoice::Confirmed(
                    proposal.evidence.clone(),
                ))?,
                review: match proposal.review {
                    ReviewRecommendation::Uncalibrated => ReviewRecord::Uncalibrated,
                    ReviewRecommendation::LowSeparation => ReviewRecord::LowSeparation,
                    ReviewRecommendation::RoutineSampling => ReviewRecord::RoutineSampling,
                },
            })
        })
        .transpose()?;
    encode(
        &ObservationRecord {
            subject,
            membership: *value.target.membership.bytes(),
            category_authority: *value.authority.category.bytes(),
            review_authority: *value.authority.review.bytes(),
            rules: *value.rules.bytes(),
            configuration: value.configuration.map(|revision| *revision.bytes()),
            human,
            deferred: value.deferred,
            prediction,
        },
        LIMIT,
    )
}

pub(in super::super) fn load(value: &str) -> Result<ClassificationObservation, ScanError> {
    let record: ObservationRecord = decode(value, LIMIT)?;
    let subject = match record.subject {
        SubjectRecord::Exact(content) => ClassificationSubject::Exact(content.admit()?),
        SubjectRecord::Provisional {
            asset_id,
            generation,
        } => ClassificationSubject::Provisional(
            ProvisionalClassificationContent::new(asset_id, generation)
                .map_err(|_| invalid_record())?,
        ),
    };
    let human = match record.human {
        HumanRecord::Unreviewed => HumanClassification::Unreviewed,
        HumanRecord::Chosen { category, restored } => HumanClassification::Chosen {
            category: category_id(category)?,
            origin: if restored {
                HumanDecisionOrigin::Restored
            } else {
                HumanDecisionOrigin::Local
            },
        },
        HumanRecord::Conflict => HumanClassification::Conflict,
    };
    let prediction = record
        .prediction
        .map(|proposal| {
            let HumanCategoryChoice::Confirmed(evidence) = decode_choice(&proposal.evidence)?
            else {
                return Err(invalid_record());
            };
            Ok(ClassificationProposal {
                evidence,
                review: match proposal.review {
                    ReviewRecord::Uncalibrated => ReviewRecommendation::Uncalibrated,
                    ReviewRecord::LowSeparation => ReviewRecommendation::LowSeparation,
                    ReviewRecord::RoutineSampling => ReviewRecommendation::RoutineSampling,
                },
            })
        })
        .transpose()?;
    Ok(ClassificationObservation {
        target: ClassificationTarget {
            subject,
            membership: ClassificationRevision::from_bytes(record.membership),
        },
        authority: ClassificationFieldAuthority {
            category: ClassificationRevision::from_bytes(record.category_authority),
            review: ClassificationRevision::from_bytes(record.review_authority),
        },
        rules: ClassificationRevision::from_bytes(record.rules),
        configuration: record.configuration.map(ClassificationRevision::from_bytes),
        human,
        deferred: record.deferred,
        prediction,
    })
}
