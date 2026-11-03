use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryId, ClassificationContent, ClassificationRecordId, ClassificationRevision,
    ClassificationRun, HumanCategoryChoice, ModelCompatibility, PredictionReference,
    RefinementIdentity,
};

use super::invalid_record;

pub(super) mod prediction;

pub(super) fn revision(value: ClassificationRevision) -> String {
    digest(value.bytes())
}

pub(super) fn digest(value: &[u8; 32]) -> String {
    blake3::Hash::from_bytes(*value).to_hex().to_string()
}

pub(super) fn read_digest(value: &str) -> Result<[u8; 32], ScanError> {
    let hash = blake3::Hash::from_hex(value).map_err(|_| invalid_record())?;
    if hash.to_hex().as_str() != value {
        return Err(invalid_record());
    }
    Ok(*hash.as_bytes())
}

pub(super) fn read_revision(value: &str) -> Result<ClassificationRevision, ScanError> {
    Ok(ClassificationRevision::from_bytes(read_digest(value)?))
}

pub(super) fn encode_run(run: &ClassificationRun) -> Result<String, ScanError> {
    let model = run.model();
    encode(
        &RunRecord {
            version: 1,
            id: run.id().as_str().into(),
            embedding_space: *model.image_identity().embedding_space.bytes(),
            image_pipeline: *model.image_identity().pipeline.bytes(),
            text_pipeline: *model.text_identity().pipeline.bytes(),
            dimensions: model.dimensions(),
            primary_rules: *run.primary_rules().bytes(),
            refinement: run.refinement().map(|identity| RefinementRecord {
                rules: *identity.rules.bytes(),
                profile: *identity.profile.bytes(),
            }),
        },
        4096,
    )
}

pub(super) fn decode_run(value: &str) -> Result<ClassificationRun, ScanError> {
    let record: RunRecord = decode(value, 4096)?;
    if record.version != 1 {
        return Err(invalid_record());
    }
    ClassificationRun::new(
        identifier(record.id)?,
        ModelCompatibility::new(
            ClassificationRevision::from_bytes(record.embedding_space),
            ClassificationRevision::from_bytes(record.image_pipeline),
            ClassificationRevision::from_bytes(record.text_pipeline),
            record.dimensions,
        )
        .map_err(|_| invalid_record())?,
        ClassificationRevision::from_bytes(record.primary_rules),
        record.refinement.map(|value| RefinementIdentity {
            rules: ClassificationRevision::from_bytes(value.rules),
            profile: ClassificationRevision::from_bytes(value.profile),
        }),
    )
    .map_err(|_| invalid_record())
}

pub(super) fn encode_choice(choice: &HumanCategoryChoice) -> Result<String, ScanError> {
    let choice = match choice {
        HumanCategoryChoice::Confirmed(reference) => ChoiceRecord::Confirmed {
            prediction_id: reference.prediction_id.as_str().into(),
            run_id: reference.run_id.as_str().into(),
            configuration: *reference.configuration.bytes(),
            primary_rules: *reference.primary_rules.bytes(),
            content: ContentRecord::from(&reference.content),
            category: reference.category.as_str().into(),
        },
        HumanCategoryChoice::Corrected { category, rules } => ChoiceRecord::Corrected {
            category: category.as_str().into(),
            rules: *rules.bytes(),
        },
    };
    encode(&VersionedChoice { version: 1, choice }, 4096)
}

pub(super) fn decode_choice(value: &str) -> Result<HumanCategoryChoice, ScanError> {
    let record: VersionedChoice = decode(value, 4096)?;
    if record.version != 1 {
        return Err(invalid_record());
    }
    match record.choice {
        ChoiceRecord::Confirmed {
            prediction_id,
            run_id,
            configuration,
            primary_rules,
            content,
            category,
        } => Ok(HumanCategoryChoice::Confirmed(PredictionReference {
            prediction_id: identifier(prediction_id)?,
            run_id: identifier(run_id)?,
            configuration: ClassificationRevision::from_bytes(configuration),
            primary_rules: ClassificationRevision::from_bytes(primary_rules),
            content: content.admit()?,
            category: category_id(category)?,
        })),
        ChoiceRecord::Corrected { category, rules } => Ok(HumanCategoryChoice::Corrected {
            category: category_id(category)?,
            rules: ClassificationRevision::from_bytes(rules),
        }),
    }
}

pub(super) fn encode<T: Serialize>(value: &T, limit: usize) -> Result<String, ScanError> {
    let payload = serde_json::to_string(value).map_err(|_| invalid_record())?;
    if payload.len() > limit {
        return Err(invalid_record());
    }
    Ok(payload)
}

pub(super) fn decode<T: DeserializeOwned>(value: &str, limit: usize) -> Result<T, ScanError> {
    if value.len() > limit {
        return Err(invalid_record());
    }
    serde_json::from_str(value).map_err(|_| invalid_record())
}

pub(super) fn identifier(value: String) -> Result<ClassificationRecordId, ScanError> {
    ClassificationRecordId::new(value).map_err(|_| invalid_record())
}

pub(super) fn category_id(value: String) -> Result<CategoryId, ScanError> {
    CategoryId::new(value).map_err(|_| invalid_record())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunRecord {
    version: u32,
    id: String,
    embedding_space: [u8; 32],
    image_pipeline: [u8; 32],
    text_pipeline: [u8; 32],
    dimensions: usize,
    primary_rules: [u8; 32],
    refinement: Option<RefinementRecord>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RefinementRecord {
    rules: [u8; 32],
    profile: [u8; 32],
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContentRecord {
    scheme: String,
    digest: [u8; 32],
    byte_length: u64,
}

impl From<&ClassificationContent> for ContentRecord {
    fn from(value: &ClassificationContent) -> Self {
        Self {
            scheme: value.scheme.as_str().into(),
            digest: value.digest,
            byte_length: value.byte_length,
        }
    }
}

impl ContentRecord {
    pub(super) fn admit(self) -> Result<ClassificationContent, ScanError> {
        if self.byte_length > i64::MAX as u64 {
            return Err(invalid_record());
        }
        Ok(ClassificationContent {
            scheme: identifier(self.scheme)?,
            digest: self.digest,
            byte_length: self.byte_length,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionedChoice {
    version: u32,
    choice: ChoiceRecord,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ChoiceRecord {
    Confirmed {
        prediction_id: String,
        run_id: String,
        configuration: [u8; 32],
        primary_rules: [u8; 32],
        content: ContentRecord,
        category: String,
    },
    Corrected {
        category: String,
        rules: [u8; 32],
    },
}
