use crate::domain::ScanError;
use crate::domain::classification::maintenance::{
    CategoryCapacityEvidence, CategoryCoverageSummary, CategoryExpansionCapacity,
    CategoryExpansionSnapshot,
};

pub enum CategoryExpansionBudget<'a> {
    Explicit(usize),
    VerifiedContents(&'a CategoryCoverageSummary),
}

impl CategoryExpansionBudget<'_> {
    pub(super) fn validate(&self) -> Result<(), ScanError> {
        if matches!(self, Self::Explicit(limit) if !(1..=256).contains(limit)) {
            return Err(ScanError::new(
                "classification_category_limit",
                "The cumulative generated-category budget must be between 1 and 256",
            ));
        }
        Ok(())
    }

    pub(super) fn resolve(
        self,
        snapshot: &CategoryExpansionSnapshot,
        minimum_members: usize,
    ) -> Result<CategoryExpansionCapacity, ScanError> {
        self.validate()?;
        let active_generated = snapshot.active_generated_categories();
        let (maximum_generated, catalog) = match self {
            Self::Explicit(limit) => (limit, None),
            Self::VerifiedContents(coverage) => {
                if coverage.snapshot.configuration != snapshot.configuration
                    || Some(coverage.snapshot.model) != snapshot.model
                    || !(2..=511).contains(&minimum_members)
                {
                    return Err(super::stale());
                }
                let verified_contents = coverage
                    .classified_contents
                    .checked_add(coverage.unclassified_contents)
                    .ok_or_else(super::stale)?;
                let supported = (verified_contents / minimum_members as u64).min(256) as usize;
                // A smaller library restricts additions; retirement has separate authority.
                let limit = supported.max(active_generated.len());
                let evidence = CategoryCapacityEvidence {
                    snapshot: coverage.snapshot.clone(),
                    coverage: coverage.evidence,
                    verified_contents,
                    minimum_members,
                };
                (limit, Some(evidence))
            }
        };
        Ok(CategoryExpansionCapacity {
            maximum_generated,
            active_generated,
            catalog,
        })
    }
}
