use serde::{Deserialize, Serialize};

use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryScreeningThresholds, ClassificationRevision, ClassificationRun, ScreeningIdentity,
    ScreeningPolicy, ScreeningProfile, ScreeningThresholds,
};

use super::super::{codec, invalid_record};

const PAYLOAD_LIMIT: usize = 65_536;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRecord {
    version: u32,
    calibration: [u8; 32],
    policy: PolicyRecord,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum PolicyRecord {
    Uniform { thresholds: ThresholdRecord },
    Categories { entries: Vec<CategoryRecord> },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CategoryRecord {
    category: String,
    thresholds: ThresholdRecord,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdRecord {
    rescreen_bits: u32,
    review_bits: Option<u32>,
}

impl ThresholdRecord {
    fn from_thresholds(thresholds: ScreeningThresholds) -> Self {
        Self {
            rescreen_bits: thresholds.rescreen_below().to_bits(),
            review_bits: thresholds.review_below().map(f32::to_bits),
        }
    }

    fn admit(self) -> Result<ScreeningThresholds, ScanError> {
        let rescreen = f32::from_bits(self.rescreen_bits);
        let thresholds = match self.review_bits {
            Some(review) => ScreeningThresholds::new(rescreen, f32::from_bits(review)),
            None => ScreeningThresholds::without_review_calibration(rescreen),
        }
        .map_err(|_| invalid_record())?;
        // The revision uses the normalized float bits, not a JSON decimal approximation.
        if thresholds.rescreen_below().to_bits() != self.rescreen_bits
            || thresholds.review_below().map(f32::to_bits) != self.review_bits
        {
            return Err(invalid_record());
        }
        Ok(thresholds)
    }
}

pub(super) fn save(profile: &ScreeningProfile) -> Result<String, ScanError> {
    let policy = match profile.policy() {
        ScreeningPolicy::Uniform(thresholds) => PolicyRecord::Uniform {
            thresholds: ThresholdRecord::from_thresholds(*thresholds),
        },
        ScreeningPolicy::Categories(entries) => PolicyRecord::Categories {
            entries: entries
                .iter()
                .map(|entry| CategoryRecord {
                    category: entry.category_id.as_str().into(),
                    thresholds: ThresholdRecord::from_thresholds(entry.thresholds),
                })
                .collect(),
        },
    };
    codec::encode(
        &ProfileRecord {
            version: 1,
            calibration: *profile.calibration().bytes(),
            policy,
        },
        PAYLOAD_LIMIT,
    )
}

pub(super) fn load(payload: &str, run: &ClassificationRun) -> Result<ScreeningProfile, ScanError> {
    let record: ProfileRecord = codec::decode(payload, PAYLOAD_LIMIT)?;
    if record.version != 1 {
        return Err(invalid_record());
    }
    let refinement = run.refinement().ok_or_else(invalid_record)?;
    let identity = ScreeningIdentity {
        model: run.model(),
        primary_rules: run.primary_rules(),
        refinement_rules: refinement.rules,
    };
    let calibration = ClassificationRevision::from_bytes(record.calibration);
    let profile = match record.policy {
        PolicyRecord::Uniform { thresholds } => {
            ScreeningProfile::new(identity, calibration, thresholds.admit()?)
        }
        PolicyRecord::Categories { entries } => {
            let entries = entries
                .into_iter()
                .map(|entry| {
                    Ok(CategoryScreeningThresholds {
                        category_id: codec::category_id(entry.category)?,
                        thresholds: entry.thresholds.admit()?,
                    })
                })
                .collect::<Result<Vec<_>, ScanError>>()?;
            ScreeningProfile::for_categories(identity, calibration, entries)
                .map_err(|_| invalid_record())?
        }
    };
    if profile.revision() != refinement.profile {
        return Err(invalid_record());
    }
    Ok(profile)
}
