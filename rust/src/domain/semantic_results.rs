use std::collections::BTreeSet;

use super::ScanError;
use super::organization::OrganizationSelection;

pub(crate) const SEMANTIC_RESULT_LIMIT: usize = 500;
pub(crate) const SEMANTIC_CANDIDATE_WINDOW: usize = 256;

#[derive(Clone, Debug, Default)]
pub struct SemanticIndexStatus {
    pub indexing: bool,
    pub blocked: bool,
    pub catalog_revision: u64,
    pub feature_generation: u64,
    pub budget_reached: bool,
    pub issue_code: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SemanticSearchInput {
    Text { description: String },
    Image { source: OrganizationSelection },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticResultManifest {
    pub id: String,
    pub catalog_revision: u64,
    pub feature_generation: u64,
    pub eligible_items: u64,
    pub indexed_items: u64,
    pub storage_full: bool,
    pub items: Vec<OrganizationSelection>,
}

impl SemanticResultManifest {
    pub(crate) fn validate(&self) -> Result<(), ScanError> {
        let mut locations = BTreeSet::new();
        if self.id.len() != 64
            || !self.id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.items.len() > SEMANTIC_RESULT_LIMIT
            || self.indexed_items > self.eligible_items
            || self.items.len() as u64 > self.indexed_items
            || self.items.iter().any(|item| {
                item.asset_id.is_empty()
                    || item.asset_id.len() > 256
                    || item.location_id.is_empty()
                    || item.location_id.len() > 256
                    || item.source_generation == 0
                    || item.source_generation > i64::MAX as u64
                    || !locations.insert(&item.location_id)
            })
        {
            return Err(ScanError::new(
                "semantic_results_invalid",
                "The search result list is invalid",
            ));
        }
        Ok(())
    }
}
