use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::organization::OrganizationSelection;
use crate::domain::organization_review::{OrganizationReviewSession, stale_session};

use super::super::super::{
    database_error,
    gallery::review::{ReviewBoundary, ReviewRead, candidate},
    unix_time_ms,
};
use super::super::{authority, records, selection};
use super::scope;

pub(super) struct Bookmark {
    pub record_id: String,
    pub boundary: ReviewBoundary,
    pub after_current: bool,
}

pub(super) struct Session {
    pub id: String,
    pub sequence: i64,
    pub digest: Option<String>,
}

pub(super) fn session(connection: &Connection) -> Result<Session, ScanError> {
    connection
        .query_row(
            "SELECT session_id, position_sequence, checkpoint_digest
             FROM organization_review_session WHERE singleton = 1",
            [],
            |row| {
                Ok(Session {
                    id: row.get(0)?,
                    sequence: row.get(1)?,
                    digest: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(stale_session)
}

pub(super) fn bookmark(connection: &Connection) -> Result<Option<Bookmark>, ScanError> {
    connection
        .query_row(
            "SELECT record_id, admission_id, root_id, location_id, after_current
             FROM organization_review_bookmark WHERE singleton = 1",
            [],
            |row| {
                Ok(Bookmark {
                    record_id: row.get(0)?,
                    boundary: ReviewBoundary {
                        admission_id: row.get(1)?,
                        root_id: row.get(2)?,
                        location_id: row.get(3)?,
                    },
                    after_current: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn save(
    connection: &Connection,
    requested: &OrganizationReviewSession,
    selected: &OrganizationSelection,
) -> Result<(), ScanError> {
    let current = session(connection)?;
    let sequence = requested.position_sequence as i64;
    if current.id != requested.session_id || sequence == 0 || sequence < current.sequence {
        return Err(stale_session());
    }
    let mut hasher = blake3::Hasher::new();
    authority::field(&mut hasher, b"ame-review-position-v1");
    authority::field(&mut hasher, selected.asset_id.as_bytes());
    authority::field(&mut hasher, selected.location_id.as_bytes());
    authority::field(&mut hasher, &selected.source_generation.to_le_bytes());
    let digest = hasher.finalize().to_hex().to_string();
    if sequence == current.sequence {
        return if current.digest.as_deref() == Some(&digest) {
            Ok(())
        } else {
            Err(stale_session())
        };
    }
    let query = scope::load(connection, "active")?.ok_or_else(stale_session)?;
    scope::validate(connection, &query)?;
    let prepared = selection::prepare(connection, std::slice::from_ref(selected))?;
    let group = prepared.groups.first().ok_or_else(stale_session)?;
    let reconciled = records::materialize(connection, &group.key)?;
    let record_id: String = connection
        .query_row(
            "SELECT record_id FROM organization_bindings
             WHERE asset_id = ?1 AND source_generation = ?2",
            params![selected.asset_id, selected.source_generation as i64],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let candidate = candidate(connection, &query, ReviewRead::Current(&record_id))?
        .ok_or_else(stale_session)?;
    scope::copy_to_bookmark(connection)?;
    connection
        .execute(
            "INSERT INTO organization_review_bookmark(
               singleton, record_id, admission_id, root_id, location_id,
               after_current, updated_unix_ms
             ) VALUES (1, ?1, ?2, ?3, ?4, 0, ?5)
             ON CONFLICT(singleton) DO UPDATE SET
               record_id = excluded.record_id, admission_id = excluded.admission_id,
               root_id = excluded.root_id, location_id = excluded.location_id,
               after_current = 0, updated_unix_ms = excluded.updated_unix_ms",
            params![
                record_id,
                candidate.boundary.admission_id,
                candidate.boundary.root_id,
                candidate.boundary.location_id,
                unix_time_ms()
            ],
        )
        .map_err(database_error)?;
    connection
        .execute(
            "UPDATE organization_review_session
             SET position_sequence = ?1, checkpoint_digest = ?2 WHERE singleton = 1",
            params![sequence, digest],
        )
        .map_err(database_error)?;
    if reconciled {
        connection
            .execute_batch(
                "UPDATE organization_state SET revision = revision + 1;
                 UPDATE catalog_state SET revision = revision + 1;",
            )
            .map_err(database_error)?;
    }
    Ok(())
}
