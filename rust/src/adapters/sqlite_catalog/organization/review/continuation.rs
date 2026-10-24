use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationContinuationStatus as Status,
};

use super::super::super::gallery::review::{ReviewRead, candidate};
use super::{position, scope};

pub(super) fn load(connection: &Connection) -> Result<OrganizationContinuation, ScanError> {
    let Some(bookmark) = position::bookmark(connection)? else {
        return Ok(OrganizationContinuation {
            status: Status::NoActivity,
            query: None,
            target: None,
        });
    };
    let query = scope::load(connection, "bookmark")?
        .ok_or_else(crate::domain::organization_review::stale_session)?;
    if let Err(error) = scope::validate(connection, &query) {
        if error.code != "organization_review_scope_unavailable"
            && error.code != "organization_album_unavailable"
        {
            return Err(error);
        }
        return Ok(OrganizationContinuation {
            status: Status::ScopeUnavailable,
            query: Some(query),
            target: None,
        });
    }
    if !bookmark.after_current
        && let Some(current) =
            candidate(connection, &query, ReviewRead::Current(&bookmark.record_id))?
    {
        return Ok(OrganizationContinuation {
            status: Status::Ready,
            query: Some(query),
            target: Some(current.selection),
        });
    }
    let unfiltered = crate::domain::GalleryQuery::default();
    let associated = candidate(
        connection,
        &unfiltered,
        ReviewRead::Current(&bookmark.record_id),
    )?
    .is_some();
    let next = candidate(
        connection,
        &query,
        ReviewRead::After {
            boundary: &bookmark.boundary,
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
        Status::EndOfQueue
    };
    Ok(OrganizationContinuation {
        status,
        query: Some(query),
        target: next.map(|next| next.selection),
    })
}
