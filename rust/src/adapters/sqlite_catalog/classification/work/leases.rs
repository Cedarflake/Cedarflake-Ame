use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationDispatchMode, ClassificationWorkLease, ClassificationWorkPriority,
    ClassificationWorkTarget,
};
use crate::domain::classification::{ClassificationContent, ClassificationRecordId};
use crate::domain::organization::OrganizationSelection;

use super::super::super::{database_error, exact_duplicates::read_unsigned, sqlite_integer};
use super::super::{codec, configuration};
use super::{control, invalid, queue};

const PRIORITIES: [ClassificationWorkPriority; 7] = [
    ClassificationWorkPriority::Requested,
    ClassificationWorkPriority::Requested,
    ClassificationWorkPriority::NewContent,
    ClassificationWorkPriority::Requested,
    ClassificationWorkPriority::NewContent,
    ClassificationWorkPriority::Requested,
    ClassificationWorkPriority::Backlog,
];
const COLUMNS: &str = "work_id, kind, source_generation, fingerprint_scheme, digest, byte_length, priority, ready_unix_ms";

pub(super) struct Work {
    pub id: u64,
    pub generation: Option<u64>,
    pub content: Option<ClassificationContent>,
    pub priority: ClassificationWorkPriority,
    pub ready_unix_ms: u64,
}

impl Work {
    fn target(&self, source: OrganizationSelection) -> Result<ClassificationWorkTarget, ScanError> {
        match (&self.content, self.generation) {
            (Some(content), None) => Ok(ClassificationWorkTarget::AnalyzeContent {
                content: content.clone(),
                source,
            }),
            (None, Some(generation)) if generation == source.source_generation => {
                Ok(ClassificationWorkTarget::VerifySource(source))
            }
            _ => Err(invalid()),
        }
    }
}

pub(super) fn claim(
    connection: &Connection,
    session: &ClassificationRecordId,
    now_unix_ms: u64,
) -> Result<Option<ClassificationWorkLease>, ScanError> {
    let dispatch = control::session(connection, session)?;
    if dispatch.state.mode != ClassificationDispatchMode::Automatic {
        return Ok(None);
    }
    let run = control::run(connection, &dispatch)?;
    let running: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM classification_work_items WHERE phase = 'running')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if running {
        return Ok(None);
    }
    let query = format!("SELECT {COLUMNS} FROM classification_work_items
        WHERE configuration_revision = ?1 AND phase = 'queued' AND priority = ?2 AND ready_unix_ms <= ?3
        ORDER BY kind, ready_unix_ms, work_id LIMIT 1");
    for _ in 0..16 {
        let mut selected = None;
        for offset in 0..PRIORITIES.len() {
            let slot = (dispatch.fairness_cursor + offset) % PRIORITIES.len();
            let work = connection
                .query_row(
                    &query,
                    params![
                        codec::revision(run.configuration()),
                        queue::stored_priority(PRIORITIES[slot]),
                        sqlite_integer(now_unix_ms, "claim time")?
                    ],
                    read,
                )
                .optional()
                .map_err(database_error)?
                .transpose()?;
            if let Some(work) = work {
                selected = Some((slot, work));
                break;
            }
        }
        let Some((slot, work)) = selected else {
            return Ok(None);
        };
        let source = match (&work.content, work.generation) {
            (Some(content), None) => queue::content_source(connection, content)?,
            (None, Some(generation)) => queue::current_source(connection, generation)?,
            _ => return Err(invalid()),
        };
        let Some(source) = source else {
            connection
                .execute(
                    "UPDATE classification_work_items SET phase = 'retired' WHERE work_id = ?1",
                    [sqlite_integer(work.id, "work identity")?],
                )
                .map_err(database_error)?;
            continue;
        };
        let lease_id = dispatch.lease_clock.checked_add(1).ok_or_else(invalid)?;
        connection.execute(
            "UPDATE classification_work_items SET phase = 'running', lease_id = ?1, lease_epoch = ?2,
                 lease_asset_id = ?3, lease_location_id = ?4, lease_source_generation = ?5, issue_code = NULL
             WHERE work_id = ?6",
            params![sqlite_integer(lease_id, "work lease")?, sqlite_integer(dispatch.state.epoch, "dispatch epoch")?,
                source.asset_id, source.location_id, sqlite_integer(source.source_generation, "source generation")?,
                sqlite_integer(work.id, "work identity")?],
        ).map_err(database_error)?;
        connection.execute(
            "UPDATE classification_dispatch_state SET lease_clock = ?1, fairness_cursor = ?2 WHERE singleton = 1",
            params![sqlite_integer(lease_id, "work lease")?, ((slot + 1) % PRIORITIES.len()) as i64],
        ).map_err(database_error)?;
        return Ok(Some(ClassificationWorkLease {
            work_id: work.id,
            lease_id,
            epoch: dispatch.state.epoch,
            session_id: session.clone(),
            run_id: run.id().clone(),
            priority: work.priority,
            target: work.target(source)?,
        }));
    }
    Ok(None)
}

pub(super) fn current(
    connection: &Connection,
    lease: &ClassificationWorkLease,
) -> Result<Option<Work>, ScanError> {
    let dispatch = control::load(connection)?;
    if dispatch.state.mode != ClassificationDispatchMode::Automatic
        || dispatch.state.epoch != lease.epoch
        || dispatch.state.session_id.as_ref() != Some(&lease.session_id)
        || dispatch.state.run_id.as_ref() != Some(&lease.run_id)
    {
        return Ok(None);
    }
    let classification = configuration::load(connection)?;
    if classification.active_run_id.as_deref() != Some(lease.run_id.as_str()) {
        return Ok(None);
    }
    let Some(configuration) = classification.configuration else {
        return Ok(None);
    };
    let source = match &lease.target {
        ClassificationWorkTarget::VerifySource(source)
        | ClassificationWorkTarget::AnalyzeContent { source, .. } => source,
    };
    let sql = format!(
        "SELECT {COLUMNS} FROM classification_work_items
        WHERE work_id = ?1 AND phase = 'running' AND lease_id = ?2 AND lease_epoch = ?3
          AND configuration_revision = ?4 AND lease_asset_id = ?5 AND lease_location_id = ?6
          AND lease_source_generation = ?7"
    );
    let work = connection
        .query_row(
            &sql,
            params![
                sqlite_integer(lease.work_id, "work identity")?,
                sqlite_integer(lease.lease_id, "work lease")?,
                sqlite_integer(lease.epoch, "dispatch epoch")?,
                codec::revision(configuration),
                source.asset_id,
                source.location_id,
                sqlite_integer(source.source_generation, "source generation")?
            ],
            read,
        )
        .optional()
        .map_err(database_error)?
        .transpose()?;
    if let Some(work) = &work
        && work.target(source.clone())? != lease.target
    {
        return Ok(None);
    }
    Ok(work)
}

pub(super) fn checkpoint(
    connection: &Connection,
    work_id: u64,
    phase: &str,
) -> Result<(), ScanError> {
    connection
        .execute(
            "UPDATE classification_work_items SET phase = ?1, lease_id = NULL, lease_epoch = NULL,
             lease_asset_id = NULL, lease_location_id = NULL, lease_source_generation = NULL
         WHERE work_id = ?2",
            params![phase, sqlite_integer(work_id, "work identity")?],
        )
        .map_err(database_error)?;
    Ok(())
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<Work, ScanError>> {
    let id = read_unsigned(row, 0)?;
    let kind: String = row.get(1)?;
    let generation: Option<i64> = row.get(2)?;
    let scheme: Option<String> = row.get(3)?;
    let digest: Option<String> = row.get(4)?;
    let length: Option<i64> = row.get(5)?;
    let priority: i64 = row.get(6)?;
    let ready_unix_ms = read_unsigned(row, 7)?;
    Ok((|| {
        let priority = match priority {
            0 => ClassificationWorkPriority::Requested,
            1 => ClassificationWorkPriority::NewContent,
            2 => ClassificationWorkPriority::Backlog,
            _ => return Err(invalid()),
        };
        let (generation, content) = match (kind.as_str(), generation, scheme, digest, length) {
            ("source", Some(generation), None, None, None) if generation > 0 => {
                (Some(generation as u64), None)
            }
            ("content", None, Some(scheme), Some(digest), Some(length)) if length > 0 => (
                None,
                Some(ClassificationContent {
                    scheme: ClassificationRecordId::new(scheme).map_err(|_| invalid())?,
                    digest: codec::read_digest(&digest)?,
                    byte_length: length as u64,
                }),
            ),
            _ => return Err(invalid()),
        };
        Ok(Work {
            id,
            generation,
            content,
            priority,
            ready_unix_ms,
        })
    })())
}
