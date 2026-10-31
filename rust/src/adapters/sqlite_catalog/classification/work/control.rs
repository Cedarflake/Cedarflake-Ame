use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchMode, ClassificationDispatchState,
};
use crate::domain::classification::{ClassificationRecordId, ClassificationRun};

use super::super::super::{database_error, exact_duplicates::read_unsigned, sqlite_integer};
use super::super::{configuration, predictions};
use super::{changed, invalid};

pub(in crate::adapters::sqlite_catalog::classification) struct Dispatch {
    pub(in crate::adapters::sqlite_catalog::classification) state: ClassificationDispatchState,
    pub source_cursor: u64,
    pub backlog_ceiling: u64,
    pub arrival_cursor: u64,
    pub lease_clock: u64,
    pub fairness_cursor: usize,
}

struct StoredDispatch {
    revision: u64,
    epoch: u64,
    mode: String,
    run: Option<String>,
    session: Option<String>,
    source_cursor: u64,
    backlog_ceiling: u64,
    arrival_cursor: u64,
    lease_clock: u64,
    fairness_cursor: u64,
}

pub(in crate::adapters::sqlite_catalog::classification) fn load(
    connection: &Connection,
) -> Result<Dispatch, ScanError> {
    let stored = connection
        .query_row(
            "SELECT revision, epoch, mode, run_id, session_id, source_cursor, lease_clock, fairness_cursor,
                    backlog_ceiling, arrival_cursor
             FROM classification_dispatch_state JOIN classification_dispatch_admission USING(singleton)
             WHERE singleton = 1",
            [],
            |row| {
                Ok(StoredDispatch {
                    revision: read_unsigned(row, 0)?,
                    epoch: read_unsigned(row, 1)?,
                    mode: row.get(2)?,
                    run: row.get(3)?,
                    session: row.get(4)?,
                    source_cursor: read_unsigned(row, 5)?,
                    lease_clock: read_unsigned(row, 6)?,
                    fairness_cursor: read_unsigned(row, 7)?,
                    backlog_ceiling: read_unsigned(row, 8)?,
                    arrival_cursor: read_unsigned(row, 9)?,
                })
            },
        )
        .map_err(database_error)?;
    let mode = match stored.mode.as_str() {
        "disabled" => ClassificationDispatchMode::Disabled,
        "automatic" => ClassificationDispatchMode::Automatic,
        "paused" => ClassificationDispatchMode::Paused,
        "stopped" => ClassificationDispatchMode::Stopped,
        _ => return Err(invalid()),
    };
    if stored.fairness_cursor > 6 {
        return Err(invalid());
    }
    Ok(Dispatch {
        state: ClassificationDispatchState {
            revision: stored.revision,
            epoch: stored.epoch,
            mode,
            run_id: stored
                .run
                .map(ClassificationRecordId::new)
                .transpose()
                .map_err(|_| invalid())?,
            session_id: stored
                .session
                .map(ClassificationRecordId::new)
                .transpose()
                .map_err(|_| invalid())?,
        },
        source_cursor: stored.source_cursor,
        backlog_ceiling: stored.backlog_ceiling,
        arrival_cursor: stored.arrival_cursor,
        lease_clock: stored.lease_clock,
        fairness_cursor: stored.fairness_cursor as usize,
    })
}

pub(super) fn session(
    connection: &Connection,
    id: &ClassificationRecordId,
) -> Result<Dispatch, ScanError> {
    let dispatch = load(connection)?;
    if dispatch.state.session_id.as_ref() != Some(id) {
        return Err(changed());
    }
    Ok(dispatch)
}

pub(super) fn run(
    connection: &Connection,
    dispatch: &Dispatch,
) -> Result<ClassificationRun, ScanError> {
    let id = dispatch.state.run_id.as_ref().ok_or_else(changed)?;
    let current = configuration::load(connection)?;
    let run = predictions::load_run(connection, id.as_str())?;
    if current.active_run_id.as_deref() != Some(id.as_str())
        || current.configuration != Some(run.configuration())
        || current.taxonomy.as_ref().map(|rules| rules.revision()) != Some(run.primary_rules())
    {
        return Err(changed());
    }
    Ok(run)
}

pub(in crate::adapters::sqlite_catalog::classification) fn change(
    connection: &Connection,
    expected_revision: u64,
    command: &ClassificationDispatchCommand,
) -> Result<ClassificationDispatchState, ScanError> {
    let mut dispatch = load(connection)?;
    if dispatch.state.revision != expected_revision {
        return Err(changed());
    }
    let previous = dispatch.state.clone();
    match command {
        ClassificationDispatchCommand::Enable { run_id }
        | ClassificationDispatchCommand::ReplaceRun { run_id } => {
            let stopped: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM classification_stopped_runs WHERE run_id = ?1)",
                    [run_id.as_str()],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            if stopped
                || (matches!(command, ClassificationDispatchCommand::ReplaceRun { .. })
                    && dispatch.state.mode == ClassificationDispatchMode::Stopped)
            {
                return Err(ScanError::new(
                    "classification_run_stopped",
                    "Start a new analysis to replace the stopped run",
                ));
            }
            dispatch.state.run_id = Some(run_id.clone());
            run(connection, &dispatch)?;
            if matches!(command, ClassificationDispatchCommand::Enable { .. }) {
                dispatch.state.mode = ClassificationDispatchMode::Automatic;
            }
        }
        ClassificationDispatchCommand::Pause => {
            if dispatch.state.mode != ClassificationDispatchMode::Automatic
                && dispatch.state.mode != ClassificationDispatchMode::Paused
            {
                return Err(changed());
            }
            dispatch.state.mode = ClassificationDispatchMode::Paused;
        }
        ClassificationDispatchCommand::Continue => {
            if dispatch.state.mode != ClassificationDispatchMode::Paused
                && dispatch.state.mode != ClassificationDispatchMode::Automatic
            {
                return Err(changed());
            }
            run(connection, &dispatch)?;
            dispatch.state.mode = ClassificationDispatchMode::Automatic;
        }
        ClassificationDispatchCommand::Disable => {
            dispatch.state.mode = ClassificationDispatchMode::Disabled
        }
        ClassificationDispatchCommand::Stop => {
            let Some(run) = &dispatch.state.run_id else {
                return Ok(dispatch.state);
            };
            connection
                .execute(
                    "INSERT OR IGNORE INTO classification_stopped_runs(run_id) VALUES (?1)",
                    [run.as_str()],
                )
                .map_err(database_error)?;
            dispatch.state.mode = ClassificationDispatchMode::Stopped;
        }
    }
    if previous == dispatch.state {
        return Ok(dispatch.state);
    }
    let reset_cursor = previous.run_id != dispatch.state.run_id;
    retire_leases(connection)?;
    save(connection, &mut dispatch.state, reset_cursor)?;
    Ok(dispatch.state)
}

pub(super) fn open_session(
    connection: &Connection,
    expected_revision: u64,
    id: &ClassificationRecordId,
) -> Result<ClassificationDispatchState, ScanError> {
    let mut state = load(connection)?.state;
    if state.revision != expected_revision {
        return Err(changed());
    }
    if state.session_id.as_ref() == Some(id) {
        return Ok(state);
    }
    // A process restart consumes one retry only for work that never reached a checkpoint.
    connection
        .execute_batch(
            "UPDATE classification_work_items SET
             phase = CASE WHEN failure_count >= 2 THEN 'failed' ELSE 'queued' END,
             failure_count = min(failure_count + 1, 3), issue_code = 'classification_interrupted',
             lease_id = NULL, lease_epoch = NULL, lease_asset_id = NULL,
             lease_location_id = NULL, lease_source_generation = NULL
         WHERE phase = 'running';",
        )
        .map_err(database_error)?;
    state.session_id = Some(id.clone());
    save(connection, &mut state, false)?;
    Ok(state)
}

fn save(
    connection: &Connection,
    state: &mut ClassificationDispatchState,
    reset_cursor: bool,
) -> Result<(), ScanError> {
    state.revision = state.revision.checked_add(1).ok_or_else(invalid)?;
    state.epoch = state.epoch.checked_add(1).ok_or_else(invalid)?;
    let mode = match state.mode {
        ClassificationDispatchMode::Disabled => "disabled",
        ClassificationDispatchMode::Automatic => "automatic",
        ClassificationDispatchMode::Paused => "paused",
        ClassificationDispatchMode::Stopped => "stopped",
    };
    connection.execute(
        "UPDATE classification_dispatch_state SET revision = ?1, epoch = ?2, mode = ?3,
             run_id = ?4, session_id = ?5, source_cursor = CASE WHEN ?6 THEN 0 ELSE source_cursor END
         WHERE singleton = 1",
        params![sqlite_integer(state.revision, "dispatch revision")?, sqlite_integer(state.epoch, "dispatch epoch")?,
                mode, state.run_id.as_ref().map(ClassificationRecordId::as_str),
                state.session_id.as_ref().map(ClassificationRecordId::as_str), reset_cursor],
    ).map_err(database_error)?;
    if reset_cursor {
        connection
            .execute_batch(
                "UPDATE classification_dispatch_admission SET
                backlog_ceiling = (SELECT next_source_generation - 1 FROM catalog_state),
                arrival_cursor = (SELECT next_source_generation - 1 FROM catalog_state)
             WHERE singleton = 1;
             DELETE FROM classification_source_publications;",
            )
            .map_err(database_error)?;
    }
    Ok(())
}

fn retire_leases(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "UPDATE classification_work_items SET phase = 'queued',
             lease_id = NULL, lease_epoch = NULL, lease_asset_id = NULL,
             lease_location_id = NULL, lease_source_generation = NULL
         WHERE phase = 'running';",
        )
        .map_err(database_error)
}
