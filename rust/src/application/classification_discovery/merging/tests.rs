use std::sync::atomic::AtomicBool;

use crate::domain::classification::discovery::{CategoryNamingPurpose, ConceptId};
use crate::domain::classification::maintenance::merging_rules::test_support::{fixture, id};
use crate::domain::classification::maintenance::merging_rules::{
    select_purpose_category_merge, validate_category_merge,
};
use crate::domain::classification::maintenance::{
    CategoryCoverageSummary, CategoryCoverageSupport, CategoryMergeSnapshot, ManagedCategoryOrigin,
    managed_category_identity,
};
use crate::domain::classification::{CategoryDefinition, CategoryId, ClassificationRun, Taxonomy};

mod contexts;
