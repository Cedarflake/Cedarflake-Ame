use serde::{Deserialize, Serialize};

use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryScore, ClassificationRevision, ClassificationRun, PredictionEvidence, RankedPrediction,
    ReviewRecommendation,
};

use super::{ContentRecord, category_id, decode, encode, identifier, invalid_record};

pub(in super::super) fn save(evidence: &PredictionEvidence) -> Result<String, ScanError> {
    encode(
        &PredictionRecord {
            version: 1,
            id: evidence.id().as_str().into(),
            run_id: evidence.run().id().as_str().into(),
            content: ContentRecord::from(evidence.content()),
            initial: RankingRecord::from(evidence.initial()),
            refined: evidence.refined().map(RankingRecord::from),
            review: match evidence.review_recommendation() {
                ReviewRecommendation::Uncalibrated => ReviewRecord::Uncalibrated,
                ReviewRecommendation::LowSeparation => ReviewRecord::LowSeparation,
                ReviewRecommendation::RoutineSampling => ReviewRecord::RoutineSampling,
            },
        },
        131072,
    )
}

pub(in super::super) fn load(
    value: &str,
    run: ClassificationRun,
) -> Result<PredictionEvidence, ScanError> {
    let record: PredictionRecord = decode(value, 131072)?;
    if record.version != 1 || record.run_id != run.id().as_str() {
        return Err(invalid_record());
    }
    let initial = record.initial.admit(&run)?;
    let refined = record
        .refined
        .map(|ranking| ranking.admit(&run))
        .transpose()?;
    PredictionEvidence::new(
        identifier(record.id)?,
        run,
        record.content.admit()?,
        initial,
        refined,
        match record.review {
            ReviewRecord::Uncalibrated => ReviewRecommendation::Uncalibrated,
            ReviewRecord::LowSeparation => ReviewRecommendation::LowSeparation,
            ReviewRecord::RoutineSampling => ReviewRecommendation::RoutineSampling,
        },
    )
    .map_err(|_| invalid_record())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PredictionRecord {
    version: u32,
    id: String,
    run_id: String,
    content: ContentRecord,
    initial: RankingRecord,
    refined: Option<RankingRecord>,
    review: ReviewRecord,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RankingRecord {
    rules: [u8; 32],
    categories: Vec<ScoreRecord>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScoreRecord {
    category: String,
    similarity: f32,
}

impl From<&RankedPrediction> for RankingRecord {
    fn from(value: &RankedPrediction) -> Self {
        Self {
            rules: *value.rules_revision().bytes(),
            categories: value
                .categories()
                .iter()
                .map(|value| ScoreRecord {
                    category: value.category_id.as_str().into(),
                    similarity: value.similarity,
                })
                .collect(),
        }
    }
}

impl RankingRecord {
    fn admit(self, run: &ClassificationRun) -> Result<RankedPrediction, ScanError> {
        let categories = self
            .categories
            .into_iter()
            .map(|value| {
                Ok(CategoryScore {
                    category_id: category_id(value.category)?,
                    similarity: value.similarity,
                })
            })
            .collect::<Result<Vec<_>, ScanError>>()?;
        RankedPrediction::new(
            ClassificationRevision::from_bytes(self.rules),
            run.model(),
            categories,
        )
        .map_err(|_| invalid_record())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReviewRecord {
    Uncalibrated,
    LowSeparation,
    RoutineSampling,
}
