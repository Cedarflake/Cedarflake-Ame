use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification_review::{
    ClassificationReviewBookmark, ClassificationReviewPosition, ClassificationReviewScope,
    ClassificationReviewSession,
};
use crate::domain::organization::OrganizationSelection;

use super::super::super::{
    database_error,
    exact_duplicates::read_unsigned,
    gallery::review::{ReviewRead, candidate},
    organization::records,
    sqlite_integer,
};
use super::super::{codec, observations, operations};
use super::{scope, stale};

pub(super) struct Session {
    pub id: String,
    pub sequence: i64,
    pub scope: ClassificationReviewScope,
    pub digest: Option<String>,
}

pub(super) fn session(connection: &Connection) -> Result<Session, ScanError> {
    let stored = connection.query_row(
        "SELECT session_id, position_sequence, scope, checkpoint_digest FROM classification_review_session WHERE singleton = 1",
        [], |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get(3)?)),
    ).optional().map_err(database_error)?.ok_or_else(stale)?;
    codec::identifier(stored.0.clone())?;
    Ok(Session {
        id: stored.0,
        sequence: stored.1,
        scope: scope::decode(&stored.2)?,
        digest: stored.3,
    })
}

pub(in crate::adapters::sqlite_catalog) fn load(
    connection: &Connection,
) -> Result<Option<ClassificationReviewBookmark>, ScanError> {
    let stored = connection.query_row(
        "SELECT record_id, scope, admission_id, root_id, location_id, primary_missing, primary_text, primary_number, after_current
         FROM classification_review_bookmark WHERE singleton = 1",
        [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, ClassificationReviewPosition {
            admission_id: read_unsigned(row, 2)?, root_id: row.get(3)?, location_id: row.get(4)?,
            primary_missing: row.get(5)?, primary_text: row.get(6)?, primary_number: row.get(7)?,
        }, row.get(8)?)),
    ).optional().map_err(database_error)?;
    stored
        .map(|(record_id, encoded, position, after_current)| {
            Ok(ClassificationReviewBookmark {
                record_id,
                scope: scope::decode(&encoded)?,
                position,
                after_current,
            })
        })
        .transpose()
}

pub(super) fn save(
    connection: &Connection,
    requested: &ClassificationReviewSession,
    selected: &OrganizationSelection,
) -> Result<(), ScanError> {
    let current = session(connection)?;
    let sequence = sqlite_integer(
        requested.position_sequence,
        "classification review position",
    )?;
    if current.id != requested.id.as_str() || sequence == 0 || sequence < current.sequence {
        return Err(stale());
    }
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.position.v1");
    for value in [
        selected.asset_id.as_bytes(),
        selected.location_id.as_bytes(),
        &selected.source_generation.to_le_bytes(),
    ] {
        operations::field(&mut hash, value);
    }
    let digest = hash.finalize().to_hex().to_string();
    if sequence == current.sequence {
        return if current.digest.as_deref() == Some(&digest) {
            Ok(())
        } else {
            Err(stale())
        };
    }
    scope::admit(connection, &current.scope)?;
    let prepared = observations::prepare(connection, std::slice::from_ref(selected))?;
    let group = prepared.groups.first().ok_or_else(stale)?;
    if !current.scope.includes(&group.effective())
        || records::materialize(connection, &observations::key(&group.target.subject))?
    {
        return Err(stale());
    }
    let record: String = connection.query_row(
        "SELECT record_id FROM organization_bindings WHERE asset_id = ?1 AND source_generation = ?2",
        params![selected.asset_id, selected.source_generation as i64], |row| row.get(0),
    ).map_err(database_error)?;
    let anchor = candidate(
        connection,
        &current.scope.gallery,
        ReviewRead::Current(&record),
    )?
    .ok_or_else(stale)?;
    let revision = operations::next_revision(connection)?;
    connection.execute(
        "INSERT INTO classification_review_bookmark(singleton, record_id, scope, admission_id, root_id, location_id,
         primary_missing, primary_text, primary_number, after_current, revision)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)
         ON CONFLICT(singleton) DO UPDATE SET record_id = excluded.record_id, scope = excluded.scope,
           admission_id = excluded.admission_id, root_id = excluded.root_id, location_id = excluded.location_id,
           primary_missing = excluded.primary_missing, primary_text = excluded.primary_text,
           primary_number = excluded.primary_number, after_current = 0, revision = excluded.revision",
        params![record, scope::encode(&current.scope)?, anchor.boundary.admission_id, anchor.boundary.root_id,
            anchor.boundary.location_id, anchor.boundary.primary_missing, anchor.boundary.primary_text,
            anchor.boundary.primary_number, revision],
    ).map_err(database_error)?;
    connection
        .execute(
            "UPDATE classification_state SET revision = ?1 WHERE singleton = 1",
            [revision],
        )
        .map_err(database_error)?;
    connection.execute(
        "UPDATE classification_review_session SET position_sequence = ?1, checkpoint_digest = ?2 WHERE singleton = 1",
        params![sequence, digest],
    ).map_err(database_error)?;
    Ok(())
}
