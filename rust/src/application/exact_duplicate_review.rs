use crate::domain::exact_duplicate_review::{
    ExactDuplicateDecision, ExactDuplicatePreference, ExactDuplicateReviewKey,
    ExactDuplicateReviewLocation, ExactDuplicateReviewLocations, ExactDuplicateReviewPage,
    ExactDuplicateReviewRange,
};
use crate::domain::exact_duplicates::EXACT_DUPLICATE_PAGE_SIZE;
use crate::domain::{ExactDuplicateGroupCursor, LibraryChangeLane, ScanError};
use crate::ports::exact_duplicate_review::ExactDuplicateReviewRepository;

use super::exact_duplicates::{
    inspect_candidate_availability, validate_digest, validate_identifier,
};

fn open_catalog() -> Result<crate::adapters::SqliteCatalog, ScanError> {
    let storage = super::storage_paths()?;
    super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)
}

fn validate_key(key: &ExactDuplicateReviewKey) -> Result<(), ScanError> {
    validate_digest(&key.digest)?;
    if key.byte_length > i64::MAX as u64 {
        return Err(ScanError::new(
            "exact_duplicate_length_invalid",
            "Content length exceeds the catalog range",
        ));
    }
    Ok(())
}

pub(crate) fn load_groups(
    after: Option<ExactDuplicateGroupCursor>,
    ignored: bool,
) -> Result<ExactDuplicateReviewPage, ScanError> {
    if let Some(cursor) = &after {
        validate_digest(&cursor.digest)?;
    }
    open_catalog()?.load_review_groups(after.as_ref(), ignored)
}

pub(crate) fn seek_groups(
    index: u64,
    ignored: bool,
) -> Result<ExactDuplicateReviewRange, ScanError> {
    open_catalog()?.seek_review_groups(index, ignored)
}

pub(crate) fn load_locations(
    key: ExactDuplicateReviewKey,
    after: Option<String>,
) -> Result<ExactDuplicateReviewLocations, ScanError> {
    validate_key(&key)?;
    if let Some(id) = &after {
        validate_identifier(id)?;
    }
    let catalog = open_catalog()?;
    let members = catalog.load_review_members(&key, after.as_deref().unwrap_or(""))?;
    drop(catalog);
    let mut candidates = members.candidates;
    let next_location_id = if candidates.len() > EXACT_DUPLICATE_PAGE_SIZE {
        candidates.truncate(EXACT_DUPLICATE_PAGE_SIZE);
        candidates
            .last()
            .map(|candidate| candidate.location.location_id.clone())
    } else {
        None
    };
    let locations = candidates
        .into_iter()
        .map(|candidate| {
            let availability = inspect_candidate_availability(&candidate);
            ExactDuplicateReviewLocation {
                asset: candidate.location,
                availability,
            }
        })
        .collect();
    Ok(ExactDuplicateReviewLocations {
        locations,
        next_location_id,
        decision: members.decision,
    })
}

pub(crate) fn prefer(
    key: ExactDuplicateReviewKey,
    preferred: Option<ExactDuplicatePreference>,
) -> Result<ExactDuplicateDecision, ScanError> {
    validate_key(&key)?;
    if let Some(target) = &preferred {
        validate_identifier(&target.location_id)?;
        validate_identifier(&target.asset_id)?;
    }
    open_catalog()?.set_duplicate_preference(&key, preferred.as_ref())
}

pub(crate) fn ignore(
    key: ExactDuplicateReviewKey,
    ignored: bool,
) -> Result<ExactDuplicateDecision, ScanError> {
    validate_key(&key)?;
    open_catalog()?.set_duplicate_ignored(&key, ignored)
}
