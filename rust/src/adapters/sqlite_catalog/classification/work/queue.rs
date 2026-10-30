use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{ClassificationBacklogPage, ClassificationWorkPriority};
use crate::domain::classification::{
    ClassificationContent, ClassificationRecordId, ClassificationRun,
};
use crate::domain::organization::{OrganizationSelection, validate_selections};

use super::super::super::{database_error, exact_duplicates, sqlite_integer};
use super::super::codec;
use super::{changed, control, invalid};

const SOURCE_WINDOW: usize = 64;

pub(super) fn seed(
    connection: &Connection,
    session: &ClassificationRecordId,
    now_unix_ms: u64,
) -> Result<ClassificationBacklogPage, ScanError> {
    let dispatch = control::session(connection, session)?;
    let run = control::run(connection, &dispatch)?;
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
         WHERE l.source_generation > ?1 AND l.location_id = (
             SELECT min(member.location_id) FROM asset_locations AS member
             JOIN library_roots AS root ON root.id = member.root_id AND root.active_scan_id = member.scan_id
             JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
             WHERE member.source_generation = l.source_generation)
         ORDER BY l.source_generation LIMIT ?2", exact_duplicates::queries::CURRENT_LOCATIONS,
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut sources = statement
        .query_map(
            params![
                sqlite_integer(dispatch.source_cursor, "source cursor")?,
                (SOURCE_WINDOW + 1) as i64
            ],
            selection,
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let has_more = sources.len() > SOURCE_WINDOW;
    sources.truncate(SOURCE_WINDOW);
    let mut queued = 0;
    for source in &sources {
        queued += enqueue_source(
            connection,
            &run,
            source.source_generation,
            ClassificationWorkPriority::Backlog,
            now_unix_ms,
        )?;
    }
    if let Some(source) = sources.last() {
        connection
            .execute(
                "UPDATE classification_dispatch_state SET source_cursor = ?1 WHERE singleton = 1",
                [sqlite_integer(source.source_generation, "source cursor")?],
            )
            .map_err(database_error)?;
    }
    Ok(ClassificationBacklogPage {
        visited_sources: sources.len() as u32,
        queued_sources: queued as u32,
        has_more,
    })
}

pub(super) fn request(
    connection: &Connection,
    session: &ClassificationRecordId,
    selections: &[OrganizationSelection],
    priority: ClassificationWorkPriority,
    now_unix_ms: u64,
) -> Result<u64, ScanError> {
    validate_selections(selections)?;
    let dispatch = control::session(connection, session)?;
    let run = control::run(connection, &dispatch)?;
    let unique: BTreeSet<_> = selections.iter().cloned().collect();
    let tuples: Vec<_> = unique
        .iter()
        .map(|item| (&item.asset_id, &item.location_id, item.source_generation))
        .collect();
    let requested = serde_json::to_string(&tuples).map_err(|_| invalid())?;
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {} JOIN json_each(?1) AS requested
         ON l.asset_id = json_extract(requested.value, '$[0]')
           AND l.location_id = json_extract(requested.value, '$[1]')
           AND l.source_generation = json_extract(requested.value, '$[2]')", exact_duplicates::queries::CURRENT_LOCATIONS,
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let sources = statement
        .query_map([requested], selection)
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    if sources.len() != unique.len() {
        return Err(changed());
    }
    let mut queued = 0;
    for source in sources {
        queued += enqueue_source(
            connection,
            &run,
            source.source_generation,
            priority,
            now_unix_ms,
        )?;
    }
    Ok(queued)
}

fn enqueue_source(
    connection: &Connection,
    run: &ClassificationRun,
    generation: u64,
    priority: ClassificationWorkPriority,
    now_unix_ms: u64,
) -> Result<u64, ScanError> {
    let changed = connection.execute(
        "INSERT INTO classification_work_items(created_run_id, configuration_revision, kind, source_generation,
             priority, phase, failure_count, ready_unix_ms)
         VALUES (?1, ?2, 'source', ?3, ?4, 'queued', 0, ?5)
         ON CONFLICT(configuration_revision, source_generation) WHERE kind = 'source'
         DO UPDATE SET priority = min(excluded.priority, classification_work_items.priority),
             phase = CASE WHEN classification_work_items.phase = 'retired' THEN 'queued' ELSE classification_work_items.phase END,
             failure_count = CASE WHEN classification_work_items.phase = 'retired' THEN 0 ELSE classification_work_items.failure_count END,
             ready_unix_ms = CASE WHEN classification_work_items.phase = 'retired' THEN excluded.ready_unix_ms ELSE classification_work_items.ready_unix_ms END
         WHERE classification_work_items.phase = 'retired'
            OR (excluded.priority < classification_work_items.priority
                AND classification_work_items.phase IN ('queued', 'running', 'waiting'))",
        params![run.id().as_str(), codec::revision(run.configuration()), sqlite_integer(generation, "source generation")?,
                stored_priority(priority), sqlite_integer(now_unix_ms, "work ready time")?],
    ).map_err(database_error)?;
    Ok(changed as u64)
}

pub(super) fn current_source(
    connection: &Connection,
    generation: u64,
) -> Result<Option<OrganizationSelection>, ScanError> {
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
        WHERE l.source_generation = ?1 ORDER BY l.location_id LIMIT 1",
        exact_duplicates::queries::CURRENT_LOCATIONS
    );
    connection
        .query_row(
            &sql,
            [sqlite_integer(generation, "source generation")?],
            selection,
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn content_source(
    connection: &Connection,
    content: &ClassificationContent,
) -> Result<Option<OrganizationSelection>, ScanError> {
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
        JOIN content_fingerprints AS f ON {} WHERE f.digest = ?2 AND f.byte_length = ?3
        AND EXISTS(SELECT 1 FROM gallery_content_fingerprints AS published
            WHERE published.source_generation = f.source_generation AND published.scheme = f.scheme)
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
                sqlite_integer(content.byte_length, "content length")?
            ],
            selection,
        )
        .optional()
        .map_err(database_error)
}

pub(super) fn verified_content(
    connection: &Connection,
    source: &OrganizationSelection,
) -> Result<Option<ClassificationContent>, ScanError> {
    let sql = format!(
        "SELECT f.digest, f.byte_length FROM {}
        JOIN content_fingerprints AS f ON {}
        JOIN gallery_content_fingerprints AS published
          ON published.source_generation = f.source_generation AND published.scheme = f.scheme
        WHERE l.asset_id = ?2 AND l.location_id = ?3 AND l.source_generation = ?4",
        exact_duplicates::queries::CURRENT_LOCATIONS,
        exact_duplicates::queries::COMPATIBLE_FINGERPRINT
    );
    let row: Option<(String, u64)> = connection
        .query_row(
            &sql,
            params![
                crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME,
                source.asset_id,
                source.location_id,
                sqlite_integer(source.source_generation, "source generation")?
            ],
            |row| Ok((row.get(0)?, exact_duplicates::read_unsigned(row, 1)?)),
        )
        .optional()
        .map_err(database_error)?;
    row.map(|(digest, byte_length)| {
        Ok(ClassificationContent {
            scheme: ClassificationRecordId::new(
                crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME.into(),
            )
            .map_err(|_| invalid())?,
            digest: codec::read_digest(&digest)?,
            byte_length,
        })
    })
    .transpose()
}

pub(super) fn stored_priority(priority: ClassificationWorkPriority) -> i64 {
    match priority {
        ClassificationWorkPriority::Requested => 0,
        ClassificationWorkPriority::NewContent => 1,
        ClassificationWorkPriority::Backlog => 2,
    }
}

pub(super) fn selection(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrganizationSelection> {
    Ok(OrganizationSelection {
        asset_id: row.get(0)?,
        location_id: row.get(1)?,
        source_generation: exact_duplicates::read_unsigned(row, 2)?,
    })
}
