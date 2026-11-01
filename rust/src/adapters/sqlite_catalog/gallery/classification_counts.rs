use rusqlite::{Transaction, params_from_iter};

use crate::domain::classification_review::{ClassificationCounts, ClassificationReviewFilter};
use crate::domain::{GalleryCollection, GalleryQuery, ScanError};

use super::super::{database_error, sqlite_unsigned};
use super::organization_groups::build_group_query;

pub(in super::super) fn load(
    transaction: &Transaction<'_>,
    query: &GalleryQuery,
) -> Result<Option<ClassificationCounts>, ScanError> {
    let GalleryCollection::SmartAlbum { category_id, .. } = &query.collection else {
        return Ok(None);
    };
    let count = |review| {
        let scope = GalleryQuery {
            collection: GalleryCollection::SmartAlbum {
                category_id: category_id.clone(),
                review,
            },
            ..query.clone()
        };
        let grouped = build_group_query(&scope);
        let value = transaction
            .query_row(
                &format!("SELECT COUNT(*) FROM ({})", grouped.sql),
                params_from_iter(grouped.parameters),
                |row| row.get(0),
            )
            .map_err(database_error)?;
        sqlite_unsigned(value, "classification group count")
    };
    Ok(Some(ClassificationCounts {
        total: count(ClassificationReviewFilter::All)?,
        pending: count(ClassificationReviewFilter::Pending)?,
        confirmed: count(ClassificationReviewFilter::Confirmed)?,
    }))
}
