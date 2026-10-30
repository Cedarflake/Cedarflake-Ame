use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRun;
use crate::domain::classification::work::ClassificationWorkPriority;

use super::super::super::{database_error, exact_duplicates::read_unsigned, sqlite_integer};
use super::super::codec;
use super::{control::Dispatch, queue};

struct Publication {
    id: i64,
    root: String,
    root_generation: i64,
    scan: String,
    after_source_generation: u64,
}

pub(super) struct Admission {
    pub visited: usize,
    pub queued: u64,
    pub has_more: bool,
}

pub(in crate::adapters::sqlite_catalog) fn root_published(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection
        .execute(
            "INSERT INTO classification_source_publications(root_id, root_generation, scan_id)
         SELECT root.id, state.generation, root.active_scan_id FROM library_roots AS root
         JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
         WHERE root.id = ?1 AND root.active_scan_id IS NOT NULL
         ON CONFLICT(root_id) DO UPDATE SET publication_id = excluded.publication_id,
             root_generation = excluded.root_generation, scan_id = excluded.scan_id,
             after_source_generation = 0",
            [root_id],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn admit(
    connection: &Connection,
    run: &ClassificationRun,
    dispatch: &Dispatch,
    limit: usize,
    now_unix_ms: u64,
) -> Result<Admission, ScanError> {
    let publication = connection
        .query_row(
            "SELECT publication_id, root_id, root_generation, scan_id, after_source_generation
         FROM classification_source_publications ORDER BY publication_id LIMIT 1",
            [],
            |row| {
                Ok(Publication {
                    id: row.get(0)?,
                    root: row.get(1)?,
                    root_generation: row.get(2)?,
                    scan: row.get(3)?,
                    after_source_generation: read_unsigned(row, 4)?,
                })
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(publication) = publication else {
        return Ok(Admission {
            visited: 0,
            queued: 0,
            has_more: false,
        });
    };
    let current: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots AS root
         JOIN library_change_root_state AS state ON state.root_id = root.id AND state.is_active = 1
         WHERE root.id = ?1 AND root.active_scan_id = ?2 AND state.generation = ?3)",
            params![
                publication.root,
                publication.scan,
                publication.root_generation
            ],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let generations = if current {
        missed_generations(connection, &publication, dispatch, limit)?
    } else {
        Vec::new()
    };
    let has_more = generations.len() > limit;
    let mut admitted = Admission {
        visited: 0,
        queued: 0,
        has_more: false,
    };
    let mut cursor = publication.after_source_generation;
    for generation in generations.into_iter().take(limit) {
        // A refreshed snapshot must not promote existing backlog or retry a known failed copy.
        let needed: bool = connection
            .query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM classification_work_items
             WHERE configuration_revision = ?1 AND kind = 'source' AND source_generation = ?2
               AND phase <> 'retired')",
                params![
                    codec::revision(run.configuration()),
                    sqlite_integer(generation, "source generation")?
                ],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if needed {
            admitted.queued += queue::enqueue_source(
                connection,
                run,
                generation,
                ClassificationWorkPriority::NewContent,
                now_unix_ms,
            )?;
        }
        admitted.visited += 1;
        cursor = generation;
    }
    if has_more {
        connection
            .execute(
                "UPDATE classification_source_publications SET after_source_generation = ?1
             WHERE publication_id = ?2",
                params![
                    sqlite_integer(cursor, "publication cursor")?,
                    publication.id
                ],
            )
            .map_err(database_error)?;
    } else {
        connection
            .execute(
                "DELETE FROM classification_source_publications WHERE publication_id = ?1",
                [publication.id],
            )
            .map_err(database_error)?;
    }
    admitted.has_more = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM classification_source_publications)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    Ok(admitted)
}

fn missed_generations(
    connection: &Connection,
    publication: &Publication,
    dispatch: &Dispatch,
    limit: usize,
) -> Result<Vec<u64>, ScanError> {
    // Sources beyond either cursor are already covered by the ordinary admission lanes.
    let mut generations = read_generations(
        connection,
        publication,
        publication.after_source_generation,
        dispatch.source_cursor,
        limit + 1,
    )?;
    if generations.len() <= limit {
        generations.extend(read_generations(
            connection,
            publication,
            publication
                .after_source_generation
                .max(dispatch.backlog_ceiling),
            dispatch.arrival_cursor,
            limit + 1 - generations.len(),
        )?);
    }
    Ok(generations)
}

fn read_generations(
    connection: &Connection,
    publication: &Publication,
    after: u64,
    through: u64,
    limit: usize,
) -> Result<Vec<u64>, ScanError> {
    if after >= through {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT source_generation FROM asset_locations
         WHERE root_id = ?1 AND scan_id = ?2 AND source_generation > ?3 AND source_generation <= ?4
         ORDER BY source_generation LIMIT ?5",
        )
        .map_err(database_error)?;
    statement
        .query_map(
            params![
                publication.root,
                publication.scan,
                sqlite_integer(after, "publication cursor")?,
                sqlite_integer(through, "publication boundary")?,
                limit as i64
            ],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)
}
