use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use crate::domain::classification_review::{ClassificationReviewFilter, ClassificationReviewScope};
use crate::domain::gallery_metadata_filter::GalleryMetadataFilter;
use crate::domain::organization::validate_identifier;
use crate::domain::{
    GalleryCollection, GalleryDuplicateMode, GalleryQuery, GallerySortDirection, GallerySortKey,
    ScanError,
};

use super::super::super::{database_error, gallery};
use super::super::{codec, invalid_record};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeRecord {
    version: u32,
    root_id: Option<String>,
    folder_relative_path: Option<String>,
    include_descendants: bool,
    search_text: String,
    metadata_filter: GalleryMetadataFilter,
    sort: Sort,
    ascending: bool,
    category: Option<String>,
    review: Review,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Sort {
    CaptureTime,
    CreatedTime,
    ModifiedTime,
    FileName,
    LibraryAdmission,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Review {
    All,
    Pending,
    Confirmed,
}

pub(in crate::adapters::sqlite_catalog) fn encode(
    scope: &ClassificationReviewScope,
) -> Result<String, ScanError> {
    validate(scope)?;
    codec::encode(
        &ScopeRecord {
            version: 1,
            root_id: scope.gallery.root_id.clone(),
            folder_relative_path: scope.gallery.folder_relative_path.clone(),
            include_descendants: scope.gallery.include_descendants,
            search_text: scope.gallery.search_text.clone(),
            metadata_filter: scope.gallery.metadata_filter.clone(),
            sort: match scope.gallery.sort_key {
                GallerySortKey::CaptureTime => Sort::CaptureTime,
                GallerySortKey::CreatedTime => Sort::CreatedTime,
                GallerySortKey::ModifiedTime => Sort::ModifiedTime,
                GallerySortKey::FileName => Sort::FileName,
                GallerySortKey::LibraryAdmission => Sort::LibraryAdmission,
            },
            ascending: scope.gallery.sort_direction == GallerySortDirection::Ascending,
            category: scope
                .category
                .as_ref()
                .map(|value| value.as_str().to_owned()),
            review: match scope.review {
                ClassificationReviewFilter::All => Review::All,
                ClassificationReviewFilter::Pending => Review::Pending,
                ClassificationReviewFilter::Confirmed => Review::Confirmed,
            },
        },
        8192,
    )
}

pub(in crate::adapters::sqlite_catalog) fn decode(
    payload: &str,
) -> Result<ClassificationReviewScope, ScanError> {
    let record: ScopeRecord = codec::decode(payload, 8192)?;
    if record.version != 1 {
        return Err(invalid_record());
    }
    let scope = ClassificationReviewScope {
        gallery: GalleryQuery {
            root_id: record.root_id,
            folder_relative_path: record.folder_relative_path,
            include_descendants: record.include_descendants,
            search_text: record.search_text,
            metadata_filter: record.metadata_filter,
            sort_key: match record.sort {
                Sort::CaptureTime => GallerySortKey::CaptureTime,
                Sort::CreatedTime => GallerySortKey::CreatedTime,
                Sort::ModifiedTime => GallerySortKey::ModifiedTime,
                Sort::FileName => GallerySortKey::FileName,
                Sort::LibraryAdmission => GallerySortKey::LibraryAdmission,
            },
            sort_direction: if record.ascending {
                GallerySortDirection::Ascending
            } else {
                GallerySortDirection::Descending
            },
            duplicate_mode: GalleryDuplicateMode::Merged,
            ..Default::default()
        },
        category: record.category.map(codec::category_id).transpose()?,
        review: match record.review {
            Review::All => ClassificationReviewFilter::All,
            Review::Pending => ClassificationReviewFilter::Pending,
            Review::Confirmed => ClassificationReviewFilter::Confirmed,
        },
    };
    validate(&scope)?;
    Ok(scope)
}

fn validate(scope: &ClassificationReviewScope) -> Result<(), ScanError> {
    let query = &scope.gallery;
    gallery::validate_gallery_query(query)?;
    if query.collection != GalleryCollection::Library
        || query.organization_queue.is_some()
        || query.new_since_admission.is_some()
        || query.duplicate_mode != GalleryDuplicateMode::Merged
    {
        return Err(invalid_record());
    }
    if let Some(root) = &query.root_id {
        validate_identifier(root)?;
    }
    if query
        .folder_relative_path
        .as_ref()
        .is_some_and(|folder| folder.len() > 32768 || folder.chars().any(char::is_control))
    {
        return Err(invalid_record());
    }
    Ok(())
}

pub(super) fn admit(
    connection: &Connection,
    scope: &ClassificationReviewScope,
) -> Result<(), ScanError> {
    validate(scope)?;
    let available: bool = connection
        .query_row(
            "SELECT (?1 IS NULL OR EXISTS(SELECT 1 FROM library_roots WHERE id = ?1))
         AND (?2 IS NULL OR EXISTS(SELECT 1 FROM classification_categories WHERE category_id = ?2))
         AND EXISTS(SELECT 1 FROM classification_state WHERE active_rules IS NOT NULL)",
            params![
                scope.gallery.root_id,
                scope.category.as_ref().map(|value| value.as_str())
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !available {
        return Err(super::stale());
    }
    Ok(())
}
