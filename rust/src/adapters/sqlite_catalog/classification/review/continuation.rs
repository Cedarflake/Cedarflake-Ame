use rusqlite::Connection;

use crate::domain::classification_review::{
    ClassificationContinuation, ClassificationContinuationStatus as Status,
};
use crate::domain::{GalleryQuery, ScanError};

use super::super::super::gallery::review::{ReviewBoundary, ReviewRead, candidate};
use super::super::super::sqlite_integer;
use super::{position, scope};

pub(super) fn load(connection: &Connection) -> Result<ClassificationContinuation, ScanError> {
    let Some(bookmark) = position::load(connection)? else {
        return Ok(ClassificationContinuation {
            status: Status::NoActivity,
            query: None,
            target: None,
        });
    };
    let query = bookmark.scope.gallery_query();
    if let Err(error) = scope::admit(connection, &bookmark.scope) {
        if error.code != "classification_review_changed" {
            return Err(error);
        }
        return Ok(ClassificationContinuation {
            status: Status::ScopeUnavailable,
            query: Some(query),
            target: None,
        });
    }
    if !bookmark.after_current
        && let Some(current) =
            candidate(connection, &query, ReviewRead::Current(&bookmark.record_id))?
    {
        return Ok(ClassificationContinuation {
            status: Status::Ready,
            query: Some(query),
            target: Some(current.selection),
        });
    }
    let current = candidate(
        connection,
        &bookmark.scope.gallery,
        ReviewRead::Current(&bookmark.record_id),
    )?;
    let associated = current.is_some()
        || candidate(
            connection,
            &GalleryQuery::default(),
            ReviewRead::Current(&bookmark.record_id),
        )?
        .is_some();
    let saved = bookmark.position;
    let boundary = current
        .map(|current| current.boundary)
        .unwrap_or(ReviewBoundary {
            admission_id: sqlite_integer(saved.admission_id, "classification review admission")?,
            root_id: saved.root_id,
            location_id: saved.location_id,
            primary_missing: saved.primary_missing,
            primary_text: saved.primary_text,
            primary_number: saved.primary_number,
        });
    let next = candidate(
        connection,
        &query,
        ReviewRead::After {
            boundary: &boundary,
            exclude_record: &bookmark.record_id,
        },
    )?;
    let status = if !associated {
        Status::AnchorUnavailable
    } else if next.is_some() {
        Status::Ready
    } else if candidate(connection, &query, ReviewRead::First)?.is_none() {
        Status::EmptyScope
    } else {
        Status::EndOfScope
    };
    Ok(ClassificationContinuation {
        status,
        query: Some(query),
        target: next.map(|next| next.selection),
    })
}
