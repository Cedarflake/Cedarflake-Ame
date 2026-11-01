use crate::domain::classification::{CategoryId, ClassificationRecordId};
use crate::domain::classification_review::{
    ClassificationContinuation, ClassificationReviewScope, ClassificationReviewSession,
};
use crate::domain::organization::OrganizationSelection;
use crate::domain::{GalleryCollection, GalleryDuplicateMode, GalleryQuery, ScanError};
use crate::ports::classification_review::ClassificationReviewRepository;

pub(crate) fn continuation() -> Result<ClassificationContinuation, ScanError> {
    super::organization::open_catalog()?.load_classification_continuation()
}

pub(crate) fn begin(query: GalleryQuery) -> Result<ClassificationReviewSession, ScanError> {
    let scope = review_scope(query)?;
    super::organization::open_catalog()?.begin_classification_review(&scope)
}

fn review_scope(mut query: GalleryQuery) -> Result<ClassificationReviewScope, ScanError> {
    let GalleryCollection::SmartAlbum {
        category_id,
        review,
    } = query.collection
    else {
        return Err(ScanError::new(
            "classification_review_changed",
            "Classification progress requires a smart album scope",
        ));
    };
    query.collection = GalleryCollection::Library;
    query.duplicate_mode = GalleryDuplicateMode::Merged;
    let category = category_id
        .map(CategoryId::new)
        .transpose()
        .map_err(|error| ScanError::new("classification_category_invalid", error.to_string()))?;
    Ok(ClassificationReviewScope {
        gallery: query,
        category,
        review,
    })
}

pub(crate) fn checkpoint(
    session: &ClassificationReviewSession,
    target: &OrganizationSelection,
) -> Result<(), ScanError> {
    super::organization::open_catalog()?.checkpoint_classification_review(session, target)
}

pub(crate) fn end(id: &ClassificationRecordId) -> Result<(), ScanError> {
    super::organization::open_catalog()?.end_classification_review(id)
}

#[cfg(test)]
mod tests;
