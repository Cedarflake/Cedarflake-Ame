use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{ClassificationBacklogPage, ClassificationWorkPriority};
use crate::domain::organization::OrganizationSelection;

use super::super::super::{database_error, exact_duplicates, sqlite_integer};
use super::{control, queue};

const SOURCE_WINDOW: usize = 64;

struct SourcePage {
    sources: Vec<OrganizationSelection>,
    has_more: bool,
}

pub(super) fn seed(
    connection: &Connection,
    session: &ClassificationRecordId,
    now_unix_ms: u64,
) -> Result<ClassificationBacklogPage, ScanError> {
    let dispatch = control::session(connection, session)?;
    let run = control::run(connection, &dispatch)?;
    let new_budget = if dispatch.source_cursor < dispatch.backlog_ceiling {
        SOURCE_WINDOW / 2
    } else {
        SOURCE_WINDOW
    };
    let arrivals = read_sources(
        connection,
        dispatch.arrival_cursor,
        i64::MAX as u64,
        new_budget,
    )?;
    let backlog = read_sources(
        connection,
        dispatch.source_cursor,
        dispatch.backlog_ceiling,
        SOURCE_WINDOW - arrivals.sources.len(),
    )?;
    let mut queued = 0;
    for (page, priority) in [
        (&arrivals, ClassificationWorkPriority::NewContent),
        (&backlog, ClassificationWorkPriority::Backlog),
    ] {
        for source in &page.sources {
            queued += queue::enqueue_source(
                connection,
                &run,
                source.source_generation,
                priority,
                now_unix_ms,
            )?;
        }
    }
    let backlog_cursor = if backlog.has_more {
        backlog
            .sources
            .last()
            .map_or(dispatch.source_cursor, |source| source.source_generation)
    } else {
        dispatch.backlog_ceiling
    };
    connection
        .execute(
            "UPDATE classification_dispatch_state SET source_cursor = ?1 WHERE singleton = 1",
            [sqlite_integer(backlog_cursor, "source cursor")?],
        )
        .map_err(database_error)?;
    if let Some(last) = arrivals.sources.last() {
        connection.execute(
            "UPDATE classification_dispatch_admission SET arrival_cursor = ?1 WHERE singleton = 1",
            [sqlite_integer(last.source_generation, "arrival cursor")?],
        ).map_err(database_error)?;
    }
    Ok(ClassificationBacklogPage {
        visited_sources: (arrivals.sources.len() + backlog.sources.len()) as u32,
        queued_sources: queued as u32,
        has_more: arrivals.has_more || backlog.has_more,
    })
}

fn read_sources(
    connection: &Connection,
    after: u64,
    through: u64,
    limit: usize,
) -> Result<SourcePage, ScanError> {
    if after >= through || limit == 0 {
        return Ok(SourcePage {
            sources: Vec::new(),
            has_more: false,
        });
    }
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation FROM {}
         WHERE l.source_generation > ?1 AND l.source_generation <= ?2 AND l.location_id = (
             SELECT min(member.location_id) FROM asset_locations AS member
             JOIN library_roots AS root ON root.id = member.root_id AND root.active_scan_id = member.scan_id
             JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
             WHERE member.source_generation = l.source_generation)
         ORDER BY l.source_generation LIMIT ?3", exact_duplicates::queries::CURRENT_LOCATIONS,
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut sources = statement
        .query_map(
            params![
                sqlite_integer(after, "source cursor")?,
                sqlite_integer(through, "source bound")?,
                (limit + 1) as i64
            ],
            queue::selection,
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    let has_more = sources.len() > limit;
    sources.truncate(limit);
    Ok(SourcePage { sources, has_more })
}
