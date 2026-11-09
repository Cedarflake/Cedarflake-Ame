use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::file_operation_journal::{
    AuthorizedFileOperation, FileOperationCheckpoint, FileOperationEvent as Event,
    FileOperationPhase as Phase, invalid,
};

use super::super::{database_error, sqlite_unsigned};
use super::codec;

struct ValidatedItem {
    id: i64,
    authorization: AuthorizedFileOperation,
    head_version: u64,
    head_phase: String,
    previous: Option<FileOperationCheckpoint>,
}

impl ValidatedItem {
    fn finish(&self) -> Result<(), ScanError> {
        let last = self
            .previous
            .as_ref()
            .ok_or_else(|| invalid("An operation has no durable intent event"))?;
        if last.version != self.head_version || codec::phase_name(last.phase) != self.head_phase {
            return Err(invalid("The operation head is not its last durable event"));
        }
        Ok(())
    }
}

pub(in super::super) fn validate_history(connection: &Connection) -> Result<(), ScanError> {
    let orphaned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('file_operation_events'))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if orphaned {
        return Err(invalid("Operation history contains an orphaned event"));
    }
    let mut statement = connection.prepare(
        "SELECT i.id, i.authorization, i.version, i.phase, i.batch_id, i.ordinal, e.version, e.snapshot
         FROM file_operation_items AS i LEFT JOIN file_operation_events AS e ON e.item_id = i.id
         ORDER BY i.id, e.version",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    let mut current: Option<ValidatedItem> = None;
    while let Some(row) = rows.next().map_err(database_error)? {
        let id: i64 = row.get(0).map_err(database_error)?;
        if current.as_ref().is_none_or(|item| item.id != id) {
            if let Some(item) = &current {
                item.finish()?;
            }
            let authorization =
                codec::decode_authorization(&row.get::<_, String>(1).map_err(database_error)?)?;
            let batch: String = row.get(4).map_err(database_error)?;
            let ordinal =
                sqlite_unsigned(row.get(5).map_err(database_error)?, "operation ordinal")?;
            if authorization.batch_id != batch || authorization.ordinal != ordinal {
                return Err(invalid("Operation identity differs from its authorization"));
            }
            current = Some(ValidatedItem {
                id,
                authorization,
                head_version: sqlite_unsigned(
                    row.get(2).map_err(database_error)?,
                    "operation version",
                )?,
                head_phase: row.get(3).map_err(database_error)?,
                previous: None,
            });
        }
        let item = current
            .as_mut()
            .ok_or_else(|| invalid("Missing operation history owner"))?;
        let version = row
            .get::<_, Option<i64>>(6)
            .map_err(database_error)?
            .ok_or_else(|| invalid("Missing operation intent event"))?;
        let checkpoint =
            codec::decode_checkpoint(&row.get::<_, String>(7).map_err(database_error)?)?;
        if checkpoint.version != sqlite_unsigned(version, "event version")? {
            return Err(invalid("The event version differs from its checkpoint"));
        }
        let expected = match &item.previous {
            None => FileOperationCheckpoint::default(),
            Some(previous) => previous.advance(&item.authorization, event_from(&checkpoint)?)?,
        };
        if codec::encode_checkpoint(&expected)? != codec::encode_checkpoint(&checkpoint)? {
            return Err(invalid(
                "An operation event violates its authorization or transition",
            ));
        }
        item.previous = Some(checkpoint);
    }
    if let Some(item) = current {
        item.finish()?;
    }
    Ok(())
}

fn event_from(checkpoint: &FileOperationCheckpoint) -> Result<Event, ScanError> {
    Ok(match checkpoint.phase {
        Phase::Executing => Event::Started(checkpoint.destination.clone()),
        Phase::NameOccupied => Event::NameOccupied,
        Phase::Copied => Event::Copied(
            checkpoint
                .copied
                .clone()
                .ok_or_else(|| invalid("Missing copied-file evidence"))?,
        ),
        Phase::RemovingSource => Event::RemovingSource,
        Phase::NativeCompleted => Event::NativeCompleted(
            checkpoint
                .receipt
                .clone()
                .ok_or_else(|| invalid("Missing native receipt"))?,
        ),
        Phase::Published => Event::Published(
            checkpoint
                .catalog_revision
                .ok_or_else(|| invalid("Missing catalog revision"))?,
        ),
        Phase::Failed => Event::Failed(
            checkpoint
                .issue
                .clone()
                .ok_or_else(|| invalid("Missing operation failure"))?,
        ),
        Phase::Cancelled => Event::Cancelled,
        Phase::Uncertain => Event::Uncertain(
            checkpoint
                .issue
                .clone()
                .ok_or_else(|| invalid("Missing uncertain-outcome evidence"))?,
        ),
        Phase::Prepared => return Err(invalid("An operation cannot return to unexecuted intent")),
    })
}
