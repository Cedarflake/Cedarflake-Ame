use std::collections::BTreeSet;

use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{
    ClassificationDispatchMode, ClassificationRecoveryCursor, ClassificationRecoveryPage,
    ClassificationRecoveryRoot, ClassificationWorkPriority,
};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_availability::ClassificationAvailabilityRepository;

use super::super::super::{
    SqliteCatalog, database_error, exact_duplicates::read_unsigned, sqlite_integer,
};
use super::super::codec;
use super::{changed, control, queue, sources};

const PAGE_SIZE: usize = 64;

pub(in crate::adapters::sqlite_catalog) fn source_removed(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    connection.execute(
        "UPDATE classification_work_items SET phase = 'queued', ready_unix_ms = 0, issue_code = NULL
         WHERE phase = 'waiting' AND work_id IN (
             SELECT work_id FROM classification_unavailable_sources WHERE root_id = ?1)",
        [root_id],
    ).map_err(database_error)?;
    Ok(())
}

impl ClassificationAvailabilityRepository for SqliteCatalog {
    fn refresh_classification_sources(
        &mut self,
        session: &ClassificationRecordId,
        selections: &[OrganizationSelection],
        now_unix_ms: u64,
    ) -> Result<u64, ScanError> {
        let transaction = self.begin_write()?;
        let dispatch = control::session(&transaction, session)?;
        let selected = queue::requested_sources(&transaction, selections)?;
        if !matches!(
            dispatch.state.mode,
            ClassificationDispatchMode::Automatic | ClassificationDispatchMode::Paused
        ) {
            return Ok(0);
        }
        let run = control::run(&transaction, &dispatch)?;
        let mut waiting = BTreeSet::new();
        for source in &selected {
            let content = queue::verified_content(&transaction, source)?;
            let mut statement = transaction
                .prepare_cached(
                    "SELECT work_id FROM classification_work_items
                 WHERE configuration_revision = ?1 AND kind = 'source' AND source_generation = ?2
                 UNION ALL SELECT work_id FROM classification_work_items
                 WHERE configuration_revision = ?1 AND kind = 'content' AND fingerprint_scheme = ?3
                   AND digest = ?4 AND byte_length = ?5 LIMIT 2",
                )
                .map_err(database_error)?;
            let work_ids = statement
                .query_map(
                    params![
                        codec::revision(run.configuration()),
                        sqlite_integer(source.source_generation, "source generation")?,
                        content.as_ref().map(|value| value.scheme.as_str()),
                        content.as_ref().map(|value| codec::digest(&value.digest)),
                        content
                            .as_ref()
                            .map(|value| sqlite_integer(value.byte_length, "content length"))
                            .transpose()?,
                    ],
                    |row| read_unsigned(row, 0),
                )
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?;
            drop(statement);
            for work_id in work_ids {
                transaction.execute(
                    "DELETE FROM classification_unavailable_sources
                     WHERE work_id = ?1 AND asset_id = ?2 AND location_id = ?3 AND source_generation = ?4",
                    params![sqlite_integer(work_id, "work identity")?, source.asset_id, source.location_id,
                        sqlite_integer(source.source_generation, "source generation")?],
                ).map_err(database_error)?;
                waiting.insert(work_id);
            }
        }
        let mut resumed = 0;
        for work_id in waiting {
            resumed += u64::from(sources::wake_waiting(&transaction, work_id, now_unix_ms)?);
        }
        for source in selected {
            resumed += queue::enqueue_source(
                &transaction,
                &run,
                source.source_generation,
                ClassificationWorkPriority::NewContent,
                now_unix_ms,
            )?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(resumed)
    }

    fn recover_classification_root(
        &mut self,
        session: &ClassificationRecordId,
        root: &ClassificationRecoveryRoot,
        cursor: Option<&ClassificationRecoveryCursor>,
        now_unix_ms: u64,
    ) -> Result<ClassificationRecoveryPage, ScanError> {
        let transaction = self.begin_write()?;
        let result = recover_root(&transaction, session, root, cursor, now_unix_ms)?;
        transaction.commit().map_err(database_error)?;
        Ok(result)
    }
}

fn recover_root(
    connection: &Connection,
    session: &ClassificationRecordId,
    root: &ClassificationRecoveryRoot,
    cursor: Option<&ClassificationRecoveryCursor>,
    now_unix_ms: u64,
) -> Result<ClassificationRecoveryPage, ScanError> {
    let dispatch = control::session(connection, session)?;
    let current: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM library_change_root_state WHERE root_id = ?1 AND generation = ?2 AND is_active = 1)",
        params![root.id, sqlite_integer(root.generation.value(), "root generation")?], |row| row.get(0),
    ).map_err(database_error)?;
    if !current {
        return Err(changed());
    }
    if !matches!(
        dispatch.state.mode,
        ClassificationDispatchMode::Automatic | ClassificationDispatchMode::Paused
    ) {
        return Ok(ClassificationRecoveryPage {
            released_sources: 0,
            resumed_tasks: 0,
            next: None,
        });
    }
    let run = control::run(connection, &dispatch)?;
    let mut cursor = match cursor {
        Some(cursor)
            if cursor.root == *root
                && cursor.run_id == *run.id()
                && cursor.after_failure_id <= cursor.through_failure_id =>
        {
            cursor.clone()
        }
        Some(_) => return Err(changed()),
        None => {
            let through = connection.query_row(
                "SELECT coalesce(max(unavailable.failure_id), 0) FROM classification_unavailable_sources AS unavailable
                 JOIN classification_work_items AS work USING(work_id)
                 WHERE unavailable.root_id = ?1 AND unavailable.root_generation = ?2 AND work.configuration_revision = ?3",
                params![root.id, sqlite_integer(root.generation.value(), "root generation")?, codec::revision(run.configuration())],
                |row| read_unsigned(row, 0),
            ).map_err(database_error)?;
            ClassificationRecoveryCursor {
                root: root.clone(),
                run_id: run.id().clone(),
                after_failure_id: 0,
                through_failure_id: through,
            }
        }
    };
    let mut statement = connection.prepare(
        "SELECT unavailable.failure_id, unavailable.work_id FROM classification_unavailable_sources AS unavailable
         JOIN classification_work_items AS work USING(work_id)
         WHERE unavailable.root_id = ?1 AND unavailable.root_generation = ?2 AND work.configuration_revision = ?3
           AND unavailable.failure_id > ?4 AND unavailable.failure_id <= ?5
         ORDER BY unavailable.failure_id LIMIT ?6",
    ).map_err(database_error)?;
    let failures = statement
        .query_map(
            params![
                root.id,
                sqlite_integer(root.generation.value(), "root generation")?,
                codec::revision(run.configuration()),
                sqlite_integer(cursor.after_failure_id, "recovery cursor")?,
                sqlite_integer(cursor.through_failure_id, "recovery boundary")?,
                PAGE_SIZE as i64 + 1,
            ],
            |row| Ok((read_unsigned(row, 0)?, read_unsigned(row, 1)?)),
        )
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    drop(statement);
    let more = failures.len() > PAGE_SIZE;
    let mut tasks = BTreeSet::new();
    let mut released = 0;
    for (failure_id, work_id) in failures.into_iter().take(PAGE_SIZE) {
        connection
            .execute(
                "DELETE FROM classification_unavailable_sources WHERE failure_id = ?1",
                [sqlite_integer(failure_id, "source failure")?],
            )
            .map_err(database_error)?;
        cursor.after_failure_id = failure_id;
        tasks.insert(work_id);
        released += 1;
    }
    let mut resumed = 0;
    for work_id in tasks {
        resumed += u32::from(sources::wake_waiting(connection, work_id, now_unix_ms)?);
    }
    Ok(ClassificationRecoveryPage {
        released_sources: released,
        resumed_tasks: resumed,
        next: more.then_some(cursor),
    })
}
