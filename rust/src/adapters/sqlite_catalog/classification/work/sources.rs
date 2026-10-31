use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{ClassificationContent, ClassificationRun};
use crate::domain::organization::OrganizationSelection;

use super::super::super::{database_error, exact_duplicates, sqlite_integer};
use super::super::codec;
use super::{
    invalid,
    leases::{self, Work},
    queue,
};

pub(super) enum SourceChoice {
    Available(OrganizationSelection),
    Waiting,
    Removed,
}

impl SourceChoice {
    pub(super) fn pending_phase(&self) -> &'static str {
        match self {
            Self::Available(_) => "queued",
            Self::Waiting => "waiting",
            Self::Removed => "retired",
        }
    }
}

pub(super) fn choose(connection: &Connection, work: &Work) -> Result<SourceChoice, ScanError> {
    let eligible = match (&work.content, work.generation) {
        (Some(content), None) => content_source(connection, content, Some(work.id))?,
        (None, Some(generation)) => current_source(connection, generation, Some(work.id))?,
        _ => return Err(invalid()),
    };
    if let Some(source) = eligible {
        return Ok(SourceChoice::Available(source));
    }
    let remaining = match (&work.content, work.generation) {
        (Some(content), None) => content_source(connection, content, None)?,
        (None, Some(generation)) => current_source(connection, generation, None)?,
        _ => return Err(invalid()),
    };
    if remaining.is_some() {
        Ok(SourceChoice::Waiting)
    } else {
        Ok(SourceChoice::Removed)
    }
}

fn current_source(
    connection: &Connection,
    generation: u64,
    work_id: Option<u64>,
) -> Result<Option<OrganizationSelection>, ScanError> {
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
         WHERE l.source_generation = ?1 AND NOT EXISTS(
             SELECT 1 FROM classification_unavailable_sources AS unavailable
             WHERE unavailable.work_id = ?2 AND unavailable.source_generation = l.source_generation
               AND unavailable.location_id = l.location_id AND unavailable.root_id = l.root_id
               AND unavailable.root_generation = state.generation)
         ORDER BY l.location_id LIMIT 1",
        exact_duplicates::queries::CURRENT_LOCATIONS
    );
    connection
        .query_row(
            &sql,
            params![
                sqlite_integer(generation, "source generation")?,
                work_id
                    .map(|id| sqlite_integer(id, "work identity"))
                    .transpose()?
            ],
            queue::selection,
        )
        .optional()
        .map_err(database_error)
}

fn content_source(
    connection: &Connection,
    content: &ClassificationContent,
    work_id: Option<u64>,
) -> Result<Option<OrganizationSelection>, ScanError> {
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
         JOIN content_fingerprints AS f ON {} WHERE f.digest = ?2 AND f.byte_length = ?3
           AND EXISTS(SELECT 1 FROM gallery_content_fingerprints AS published
               WHERE published.source_generation = f.source_generation AND published.scheme = f.scheme)
           AND NOT EXISTS(SELECT 1 FROM classification_unavailable_sources AS unavailable
               WHERE unavailable.work_id = ?4 AND unavailable.source_generation = l.source_generation
                 AND unavailable.location_id = l.location_id AND unavailable.root_id = l.root_id
                 AND unavailable.root_generation = state.generation)
         ORDER BY l.location_id LIMIT 1",
        exact_duplicates::queries::CURRENT_LOCATIONS,
        exact_duplicates::queries::COMPATIBLE_FINGERPRINT
    );
    connection
        .query_row(
            &sql,
            params![
                content.scheme.as_str(),
                codec::digest(&content.digest),
                sqlite_integer(content.byte_length, "content length")?,
                work_id
                    .map(|id| sqlite_integer(id, "work identity"))
                    .transpose()?
            ],
            queue::selection,
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn unavailable(
    connection: &Connection,
    work: &Work,
    source: &OrganizationSelection,
) -> Result<SourceChoice, ScanError> {
    let sql = format!(
        "INSERT INTO classification_unavailable_sources(
             work_id, root_id, root_generation, asset_id, location_id, source_generation)
         SELECT ?1, l.root_id, state.generation, l.asset_id, l.location_id, l.source_generation FROM {}
         WHERE l.asset_id = ?2 AND l.location_id = ?3 AND l.source_generation = ?4
         ON CONFLICT(work_id, source_generation, location_id) DO UPDATE SET
             failure_id = excluded.failure_id,
             root_id = excluded.root_id, root_generation = excluded.root_generation,
             asset_id = excluded.asset_id",
        exact_duplicates::queries::CURRENT_LOCATIONS,
    );
    connection
        .execute(
            &sql,
            params![
                sqlite_integer(work.id, "work identity")?,
                source.asset_id,
                source.location_id,
                sqlite_integer(source.source_generation, "source generation")?,
            ],
        )
        .map_err(database_error)?;
    choose(connection, work)
}

pub(super) fn clear(connection: &Connection, work_id: u64) -> Result<(), ScanError> {
    connection
        .execute(
            "DELETE FROM classification_unavailable_sources WHERE work_id = ?1",
            [sqlite_integer(work_id, "work identity")?],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn wake_waiting(
    connection: &Connection,
    work_id: u64,
    now_unix_ms: u64,
) -> Result<bool, ScanError> {
    let Some(work) = leases::waiting(connection, work_id)? else {
        return Ok(false);
    };
    if !matches!(choose(connection, &work)?, SourceChoice::Available(_)) {
        return Ok(false);
    }
    connection.execute(
        "UPDATE classification_work_items SET phase = 'queued', issue_code = NULL, ready_unix_ms = ?1
         WHERE work_id = ?2 AND phase = 'waiting'",
        params![sqlite_integer(now_unix_ms, "work ready time")?, sqlite_integer(work_id, "work identity")?],
    ).map_err(database_error)?;
    Ok(true)
}

pub(super) fn wake_generation(
    connection: &Connection,
    run: &ClassificationRun,
    generation: u64,
    now_unix_ms: u64,
) -> Result<bool, ScanError> {
    let waiting = connection.query_row(
        "SELECT work_id FROM classification_work_items
         WHERE configuration_revision = ?1 AND kind = 'source' AND source_generation = ?2 AND phase = 'waiting'",
        params![codec::revision(run.configuration()), sqlite_integer(generation, "source generation")?],
        |row| exact_duplicates::read_unsigned(row, 0),
    ).optional().map_err(database_error)?;
    match waiting {
        Some(work_id) => wake_waiting(connection, work_id, now_unix_ms),
        None => Ok(false),
    }
}

pub(super) fn wake_content(
    connection: &Connection,
    run: &ClassificationRun,
    content: &ClassificationContent,
    now_unix_ms: u64,
) -> Result<(), ScanError> {
    let waiting = connection
        .query_row(
            "SELECT work_id FROM classification_work_items
         WHERE configuration_revision = ?1 AND kind = 'content' AND fingerprint_scheme = ?2
           AND digest = ?3 AND byte_length = ?4 AND phase = 'waiting'",
            params![
                codec::revision(run.configuration()),
                content.scheme.as_str(),
                codec::digest(&content.digest),
                sqlite_integer(content.byte_length, "content length")?
            ],
            |row| exact_duplicates::read_unsigned(row, 0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(work_id) = waiting {
        wake_waiting(connection, work_id, now_unix_ms)?;
    }
    Ok(())
}
