use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationWorkFailure, ClassificationWorkIssue, ClassificationWorkLease,
    ClassificationWorkTarget,
};
use crate::domain::classification::{ClassificationRecordId, PredictionEvidence};

use super::super::super::{database_error, sqlite_integer};
use super::super::{codec, predictions};
use super::{control, invalid, leases, queue, sources};

pub(super) fn verified(
    connection: &Connection,
    lease: &ClassificationWorkLease,
) -> Result<bool, ScanError> {
    let Some(work) = leases::current(connection, lease)? else {
        return Ok(false);
    };
    let ClassificationWorkTarget::VerifySource(source) = &lease.target else {
        return Err(invalid());
    };
    let Some(content) = queue::verified_content(connection, source)? else {
        let phase = sources::choose(connection, &work)?.pending_phase();
        leases::checkpoint(connection, work.id, phase)?;
        return Ok(false);
    };
    let run = predictions::load_run(connection, lease.run_id.as_str())?;
    let phase = if predictions::latest(connection, &content, run.configuration())?.is_some() {
        "done"
    } else {
        "queued"
    };
    connection.execute(
        "INSERT INTO classification_work_items(created_run_id, configuration_revision, kind,
             fingerprint_scheme, digest, byte_length, priority, phase, failure_count, ready_unix_ms)
         VALUES (?1, ?2, 'content', ?3, ?4, ?5, ?6, ?7, 0, ?8)
         ON CONFLICT(configuration_revision, fingerprint_scheme, digest, byte_length) WHERE kind = 'content'
         DO UPDATE SET priority = min(excluded.priority, classification_work_items.priority),
             phase = CASE WHEN classification_work_items.phase = 'retired' THEN excluded.phase ELSE classification_work_items.phase END,
             failure_count = CASE WHEN classification_work_items.phase = 'retired' THEN 0 ELSE classification_work_items.failure_count END
         WHERE classification_work_items.phase = 'retired'
            OR (excluded.priority < classification_work_items.priority
                AND classification_work_items.phase IN ('queued', 'running', 'waiting'))",
        params![run.id().as_str(), codec::revision(run.configuration()), content.scheme.as_str(),
            codec::digest(&content.digest), sqlite_integer(content.byte_length, "content length")?,
            queue::stored_priority(work.priority), phase, sqlite_integer(work.ready_unix_ms, "work ready time")?],
    ).map_err(database_error)?;
    sources::wake_content(connection, &run, &content, work.ready_unix_ms)?;
    leases::checkpoint(connection, work.id, "done")?;
    sources::clear(connection, work.id)?;
    Ok(true)
}

pub(super) fn publish(
    connection: &Connection,
    lease: &ClassificationWorkLease,
    evidence: &PredictionEvidence,
) -> Result<bool, ScanError> {
    let Some(work) = leases::current(connection, lease)? else {
        return Ok(false);
    };
    let ClassificationWorkTarget::AnalyzeContent { content, source } = &lease.target else {
        return Err(invalid());
    };
    if evidence.content() != content || evidence.run().id() != &lease.run_id {
        return Err(invalid());
    }
    if queue::verified_content(connection, source)?.as_ref() != Some(content) {
        let phase = sources::choose(connection, &work)?.pending_phase();
        leases::checkpoint(connection, work.id, phase)?;
        return Ok(false);
    }
    predictions::publish(connection, source, evidence)?;
    leases::checkpoint(connection, work.id, "done")?;
    sources::clear(connection, work.id)?;
    Ok(true)
}

pub(super) fn yield_work(
    connection: &Connection,
    lease: &ClassificationWorkLease,
) -> Result<bool, ScanError> {
    let Some(work) = leases::current(connection, lease)? else {
        return Ok(false);
    };
    leases::checkpoint(connection, work.id, "queued")?;
    Ok(true)
}

pub(super) fn fail(
    connection: &Connection,
    lease: &ClassificationWorkLease,
    issue: &ClassificationWorkIssue,
) -> Result<bool, ScanError> {
    ClassificationRecordId::new(issue.code.clone()).map_err(|_| invalid())?;
    let Some(work) = leases::current(connection, lease)? else {
        return Ok(false);
    };
    let (phase, count_failure) = match issue.kind {
        ClassificationWorkFailure::Unavailable => {
            let source = match &lease.target {
                ClassificationWorkTarget::VerifySource(source)
                | ClassificationWorkTarget::AnalyzeContent { source, .. } => source,
            };
            (
                sources::unavailable(connection, &work, source)?.pending_phase(),
                false,
            )
        }
        ClassificationWorkFailure::Transient => ("queued", true),
        ClassificationWorkFailure::Permanent => ("failed", true),
    };
    connection
        .execute(
            "UPDATE classification_work_items SET
             phase = CASE WHEN ?1 = 'queued' AND ?2 AND failure_count >= 2 THEN 'failed' ELSE ?1 END,
             failure_count = min(failure_count + ?2, 3), ready_unix_ms = ?3, issue_code = ?4,
             lease_id = NULL, lease_epoch = NULL, lease_asset_id = NULL,
             lease_location_id = NULL, lease_source_generation = NULL
         WHERE work_id = ?5",
            params![
                phase,
                count_failure,
                if count_failure { sqlite_integer(issue.retry_unix_ms, "retry time")? } else { 0 },
                issue.code,
                sqlite_integer(work.id, "work identity")?
            ],
        )
        .map_err(database_error)?;
    Ok(true)
}

pub(super) fn retry(
    connection: &Connection,
    session: &ClassificationRecordId,
    now_unix_ms: u64,
) -> Result<u64, ScanError> {
    let dispatch = control::session(connection, session)?;
    let run = control::run(connection, &dispatch)?;
    connection
        .execute(
            "DELETE FROM classification_unavailable_sources WHERE work_id IN (
             SELECT work_id FROM classification_work_items
             WHERE configuration_revision = ?1 AND phase IN ('waiting', 'failed'))",
            [codec::revision(run.configuration())],
        )
        .map_err(database_error)?;
    let changed = connection.execute(
        "UPDATE classification_work_items SET phase = 'queued', failure_count = 0, issue_code = NULL, ready_unix_ms = ?1
         WHERE configuration_revision = ?2 AND phase IN ('waiting', 'failed')",
        params![sqlite_integer(now_unix_ms, "retry time")?, codec::revision(run.configuration())],
    ).map_err(database_error)?;
    Ok(changed as u64)
}
