use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::ClassificationRecordId;
use crate::domain::classification::work::{
    ClassificationFailureRecovery, ClassificationWorkIssue, ClassificationWorkLease,
    ClassificationWorkTarget,
};

use super::super::super::{database_error, sqlite_integer};
use super::{invalid, leases, sources};

pub(super) fn record(
    connection: &Connection,
    lease: &ClassificationWorkLease,
    issue: &ClassificationWorkIssue,
) -> Result<bool, ScanError> {
    ClassificationRecordId::new(issue.code.clone()).map_err(|_| invalid())?;
    let Some(work) = leases::current(connection, lease)? else {
        return Ok(false);
    };
    let (phase, failures, ready) = match issue.recovery(work.failure_count) {
        ClassificationFailureRecovery::SourceAvailability => {
            let source = match &lease.target {
                ClassificationWorkTarget::VerifySource(source)
                | ClassificationWorkTarget::AnalyzeContent { source, .. } => source,
            };
            (
                sources::unavailable(connection, &work, source)?.pending_phase(),
                work.failure_count,
                0,
            )
        }
        ClassificationFailureRecovery::RetryAt(time) => ("queued", work.failure_count + 1, time),
        ClassificationFailureRecovery::ChangedInput => {
            ("failed", (work.failure_count + 1).min(3), 0)
        }
    };
    connection
        .execute(
            "UPDATE classification_work_items SET
             phase = ?1, failure_count = ?2, ready_unix_ms = ?3, issue_code = ?4,
             lease_id = NULL, lease_epoch = NULL, lease_asset_id = NULL,
             lease_location_id = NULL, lease_source_generation = NULL
         WHERE work_id = ?5",
            params![
                phase,
                sqlite_integer(failures, "failure count")?,
                sqlite_integer(ready, "retry time")?,
                issue.code,
                sqlite_integer(work.id, "work identity")?
            ],
        )
        .map_err(database_error)?;
    if phase == "failed" {
        connection
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
    }
    Ok(true)
}
