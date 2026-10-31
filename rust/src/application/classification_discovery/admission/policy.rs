use crate::domain::ScanError;
use crate::domain::classification::ClassificationRevision;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CategoryAdmissionCriteria {
    pub minimum_members: usize,
    pub coverage_per_thousand: u16,
    pub minimum_improvement: f32,
    pub minimum_separation: f32,
    pub minimum_context_contrast: f32,
    pub max_proposals: usize,
}

pub struct CategoryAdmissionPolicy {
    criteria: CategoryAdmissionCriteria,
    revision: ClassificationRevision,
}

impl CategoryAdmissionPolicy {
    pub fn new(
        calibration: ClassificationRevision,
        mut criteria: CategoryAdmissionCriteria,
    ) -> Result<Self, ScanError> {
        if !(2..=511).contains(&criteria.minimum_members)
            || !(1..=1000).contains(&criteria.coverage_per_thousand)
            || !(1..=64).contains(&criteria.max_proposals)
        {
            return Err(invalid_policy());
        }
        for value in [
            &mut criteria.minimum_improvement,
            &mut criteria.minimum_separation,
            &mut criteria.minimum_context_contrast,
        ] {
            if !value.is_finite() || !(0.0..=2.0).contains(value) {
                return Err(invalid_policy());
            }
            *value = value.max(0.0);
        }
        let mut values = Vec::new();
        values.extend_from_slice(&(criteria.minimum_members as u64).to_le_bytes());
        values.extend_from_slice(&criteria.coverage_per_thousand.to_le_bytes());
        values.extend_from_slice(&criteria.minimum_improvement.to_le_bytes());
        values.extend_from_slice(&criteria.minimum_separation.to_le_bytes());
        values.extend_from_slice(&criteria.minimum_context_contrast.to_le_bytes());
        values.extend_from_slice(&(criteria.max_proposals as u64).to_le_bytes());
        Ok(Self {
            criteria,
            revision: ClassificationRevision::derive(
                "ame.classification.category-admission.v1",
                [calibration.bytes().as_slice(), values.as_slice()],
            ),
        })
    }

    pub fn criteria(&self) -> CategoryAdmissionCriteria {
        self.criteria
    }

    pub fn revision(&self) -> ClassificationRevision {
        self.revision
    }
}

fn invalid_policy() -> ScanError {
    ScanError::new(
        "classification_discovery_policy_invalid",
        "Category admission requires bounded support, coverage and cosine separations",
    )
}
