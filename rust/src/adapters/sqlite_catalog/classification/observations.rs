use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationContent, ClassificationFieldAuthority, ClassificationObservation,
    ClassificationRevision, ClassificationSubject, ClassificationTarget, HumanClassification,
    HumanClassificationConsensus, HumanDecisionOrigin, ProvisionalClassificationContent,
};
use crate::domain::classification_catalog::PreparedClassificationSelection;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::{OrganizationGroupKey, OrganizationSelection};

use super::super::{database_error, organization};
use super::{codec, configuration, invalid_record, operations::field, predictions};

pub(super) fn prepare(
    connection: &Connection,
    selections: &[OrganizationSelection],
) -> Result<PreparedClassificationSelection, ScanError> {
    let selection = organization::selection::prepare(connection, selections)?;
    let state = configuration::load(connection)?;
    let rules = state
        .taxonomy
        .ok_or_else(|| {
            ScanError::new(
                "classification_not_configured",
                "Choose category definitions before classifying images",
            )
        })?
        .revision();
    let mut groups = Vec::with_capacity(selection.groups.len());
    for group in selection.groups {
        let (authority, human, deferred) = decisions(connection, &group.key)?;
        let subject = subject(&group.key)?;
        let prediction = match (&subject, state.configuration) {
            (ClassificationSubject::Exact(content), Some(configuration)) => {
                predictions::latest(connection, content, configuration)?
                    .map(|value| value.proposal())
            }
            _ => None,
        };
        groups.push(ClassificationObservation {
            target: ClassificationTarget {
                subject,
                membership: codec::read_revision(&group.membership_authority)?,
            },
            authority,
            rules,
            configuration: state.configuration,
            human,
            deferred,
            prediction,
        });
    }
    Ok(PreparedClassificationSelection {
        selections: selection.selections,
        groups,
    })
}

pub(super) fn subject(key: &OrganizationGroupKey) -> Result<ClassificationSubject, ScanError> {
    Ok(match key {
        OrganizationGroupKey::Confirmed {
            digest,
            byte_length,
        } => ClassificationSubject::Exact(ClassificationContent {
            scheme: codec::identifier(EXACT_FINGERPRINT_SCHEME.into())?,
            digest: codec::read_digest(digest)?,
            byte_length: *byte_length,
        }),
        OrganizationGroupKey::Provisional {
            asset_id,
            source_generation,
        } => ClassificationSubject::Provisional(
            ProvisionalClassificationContent::new(asset_id.clone(), *source_generation)
                .map_err(|_| invalid_record())?,
        ),
    })
}

pub(super) fn key(subject: &ClassificationSubject) -> OrganizationGroupKey {
    match subject {
        ClassificationSubject::Exact(content) => OrganizationGroupKey::Confirmed {
            digest: codec::digest(&content.digest),
            byte_length: content.byte_length,
        },
        ClassificationSubject::Provisional(content) => OrganizationGroupKey::Provisional {
            asset_id: content.asset_id().into(),
            source_generation: content.source_generation(),
        },
    }
}

fn decisions(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<(ClassificationFieldAuthority, HumanClassification, bool), ScanError> {
    let mut category_hash =
        blake3::Hasher::new_derive_key("ame.classification.category.authority.v1");
    let mut review_hash = blake3::Hasher::new_derive_key("ame.classification.review.authority.v1");
    let mut consensus = HumanClassificationConsensus::default();
    let mut local_review = None;
    let mut restored_deferred = false;
    if let Some(record) = organization::inspection::record(connection, key)? {
        let mut statement = connection
            .prepare_cached(
                "SELECT decision.record_id, choice, choice_revision, choice_restored, deferred,
                    review_revision, review_restored
             FROM organization_record_groups AS member
             JOIN classification_decisions AS decision USING(record_id)
             WHERE member.group_id = ?1 ORDER BY decision.record_id",
            )
            .map_err(database_error)?;
        let mut rows = statement.query([record]).map_err(database_error)?;
        while let Some(row) = rows.next().map_err(database_error)? {
            let record_id: String = row.get(0).map_err(database_error)?;
            let choice: Option<String> = row.get(1).map_err(database_error)?;
            let choice_revision: i64 = row.get(2).map_err(database_error)?;
            let choice_restored: bool = row.get(3).map_err(database_error)?;
            let deferred: bool = row.get(4).map_err(database_error)?;
            let review_revision: i64 = row.get(5).map_err(database_error)?;
            let review_restored: bool = row.get(6).map_err(database_error)?;
            if choice_revision > 0 {
                field(&mut category_hash, record_id.as_bytes());
                field(&mut category_hash, &choice_revision.to_le_bytes());
                field(&mut category_hash, &[u8::from(choice_restored)]);
                field(
                    &mut category_hash,
                    choice.as_deref().unwrap_or("").as_bytes(),
                );
            }
            if let Some(choice) = choice {
                consensus.include(
                    &codec::decode_choice(&choice)?,
                    if choice_restored {
                        HumanDecisionOrigin::Restored
                    } else {
                        HumanDecisionOrigin::Local
                    },
                );
            }
            if review_revision > 0 {
                field(&mut review_hash, record_id.as_bytes());
                field(&mut review_hash, &review_revision.to_le_bytes());
                field(
                    &mut review_hash,
                    &[u8::from(review_restored), u8::from(deferred)],
                );
                if review_restored {
                    restored_deferred |= deferred;
                } else {
                    local_review = Some(local_review.unwrap_or(false) || deferred);
                }
            }
        }
    }
    Ok((
        ClassificationFieldAuthority {
            category: ClassificationRevision::from_bytes(*category_hash.finalize().as_bytes()),
            review: ClassificationRevision::from_bytes(*review_hash.finalize().as_bytes()),
        },
        consensus.resolve(),
        local_review.unwrap_or(restored_deferred),
    ))
}
