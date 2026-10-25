use rusqlite::Connection;

use crate::domain::organization_review::{
    OrganizationContinuation, OrganizationContinuationStatus as Status,
};
use crate::domain::{GalleryQuery, ScanError};

use super::super::super::gallery::review::{ReviewRead, candidate};
use super::{position, scope};

pub(super) fn load(
    connection: &Connection,
    requested: Option<&GalleryQuery>,
) -> Result<OrganizationContinuation, ScanError> {
    let Some(bookmark) = position::bookmark(connection)? else {
        return Ok(OrganizationContinuation {
            status: Status::NoActivity,
            query: None,
            target: None,
        });
    };
    let saved = scope::load(connection, "bookmark")?
        .ok_or_else(crate::domain::organization_review::stale_session)?;
    let mut query = saved.clone();
    if let Some(requested) = requested {
        if requested.collection != saved.collection
            || requested.root_id != saved.root_id
            || requested.folder_relative_path != saved.folder_relative_path
            || requested.include_descendants != saved.include_descendants
        {
            return Ok(OrganizationContinuation {
                status: Status::NoActivity,
                query: None,
                target: None,
            });
        }
        query.sort_key = requested.sort_key.clone();
        query.sort_direction = requested.order_direction().clone();
    }
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
    let mut anchor_scope = query.clone();
    anchor_scope.organization_queue = None;
    let current = candidate(
        connection,
        &anchor_scope,
        ReviewRead::Current(&bookmark.record_id),
    )?;
    let associated = current.is_some()
        || candidate(
            connection,
            &GalleryQuery::default(),
            ReviewRead::Current(&bookmark.record_id),
        )?
        .is_some();
    let boundary = if let Some(current) = &current {
        &current.boundary
    } else if saved.sort_key == query.sort_key && saved.order_direction() == query.order_direction()
    {
        &bookmark.boundary
    } else {
        return Ok(OrganizationContinuation {
            status: Status::AnchorUnavailable,
            query: Some(query),
            target: None,
        });
    };
    let next = candidate(
        connection,
        &query,
        ReviewRead::After {
            boundary,
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
