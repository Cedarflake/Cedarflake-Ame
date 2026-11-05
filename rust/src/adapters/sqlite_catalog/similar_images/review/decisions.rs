use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::domain::ScanError;
use crate::domain::similar_images::SIMILARITY_SCHEME;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarUndoResult,
};

use super::super::super::exact_duplicates::read_unsigned;
use super::super::super::{database_error, sqlite_integer};
use super::{projection, queries};

pub(super) fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<SimilarGroupDecision> {
    Ok(SimilarGroupDecision {
        ignored: row.get(start)?,
        revision: read_unsigned(row, start + 1)?,
    })
}

pub(super) fn load(
    connection: &Connection,
    group_id: &str,
) -> Result<SimilarGroupDecision, ScanError> {
    connection
        .query_row(
            "SELECT COALESCE(choice.ignored, 0), COALESCE(choice.revision, 0)
         FROM similarity_groups AS g
         LEFT JOIN similarity_decisions AS choice ON choice.group_id = g.group_id
         WHERE g.group_id = ?1 AND g.scheme = ?2",
            params![group_id, SIMILARITY_SCHEME],
            |row| read(row, 0),
        )
        .optional()
        .map_err(database_error)?
        .ok_or_else(projection::stale)
}

fn current(
    connection: &Connection,
    context: &SimilarReviewContext,
) -> Result<SimilarGroupDecision, ScanError> {
    let decision = load(connection, &context.group_id)?;
    if decision.revision != context.revision {
        return Err(projection::stale());
    }
    Ok(decision)
}

fn save(
    transaction: &Transaction<'_>,
    group_id: &str,
    decision: &mut SimilarGroupDecision,
) -> Result<(), ScanError> {
    decision.revision = decision
        .revision
        .checked_add(1)
        .ok_or_else(projection::stale)?;
    transaction.execute(
        "INSERT INTO similarity_decisions (group_id, ignored, revision) VALUES (?1, ?2, ?3)
         ON CONFLICT(group_id) DO UPDATE SET ignored = excluded.ignored, revision = excluded.revision",
        params![
            group_id,
            decision.ignored, sqlite_integer(decision.revision, "decision revision")?,
        ],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn prefer(
    transaction: &Transaction<'_>,
    context: &SimilarReviewContext,
    selection: &SimilarMemberSelection,
    preferred: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    let mut decision = current(transaction, context)?;
    projection::require_selected(transaction, context, selection)?;
    let sql = if preferred {
        "INSERT INTO similarity_preferences (group_id, fingerprint_scheme, digest, byte_length)
         VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING"
    } else {
        "DELETE FROM similarity_preferences
         WHERE group_id = ?1 AND fingerprint_scheme = ?2 AND digest = ?3 AND byte_length = ?4"
    };
    let changed = transaction
        .execute(
            sql,
            params![
                context.group_id,
                selection.content.scheme,
                selection.content.digest,
                sqlite_integer(selection.content.byte_length, "content length")?,
            ],
        )
        .map_err(database_error)?;
    if changed > 0 {
        save(transaction, &context.group_id, &mut decision)?;
    }
    Ok(decision)
}

pub(super) fn ignore(
    transaction: &Transaction<'_>,
    context: &SimilarReviewContext,
    ignored: bool,
) -> Result<SimilarGroupDecision, ScanError> {
    let mut decision = current(transaction, context)?;
    if ignored && queries::count_members(transaction, &context.group_id, context.degree)? < 2 {
        return Err(projection::stale());
    }
    if decision.ignored != ignored {
        decision.ignored = ignored;
        save(transaction, &context.group_id, &mut decision)?;
    }
    Ok(decision)
}

pub(super) fn exclude(
    transaction: &Transaction<'_>,
    context: &SimilarReviewContext,
    selection: &SimilarMemberSelection,
    exclusion_id: &str,
) -> Result<SimilarExclusionResult, ScanError> {
    let mut decision = current(transaction, context)?;
    projection::require_selected(transaction, context, selection)?;
    let content = &selection.content;
    transaction
        .execute(
            "INSERT INTO similarity_exclusions VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                context.group_id,
                content.scheme,
                content.digest,
                sqlite_integer(content.byte_length, "content length")?,
                exclusion_id,
            ],
        )
        .map_err(database_error)?;
    save(transaction, &context.group_id, &mut decision)?;
    Ok(SimilarExclusionResult {
        decision,
        undo: SimilarMemberExclusion {
            group_id: context.group_id.clone(),
            content: content.clone(),
            exclusion_id: exclusion_id.into(),
        },
    })
}

pub(super) fn undo(
    transaction: &Transaction<'_>,
    exclusion: &SimilarMemberExclusion,
) -> Result<SimilarUndoResult, ScanError> {
    let mut decision = load(transaction, &exclusion.group_id)?;
    let content = &exclusion.content;
    let changed = transaction
        .execute(
            "DELETE FROM similarity_exclusions
         WHERE group_id = ?1 AND fingerprint_scheme = ?2 AND digest = ?3
           AND byte_length = ?4 AND exclusion_id = ?5",
            params![
                exclusion.group_id,
                content.scheme,
                content.digest,
                sqlite_integer(content.byte_length, "content length")?,
                exclusion.exclusion_id,
            ],
        )
        .map_err(database_error)?;
    if changed > 0 {
        save(transaction, &exclusion.group_id, &mut decision)?;
    }
    Ok(SimilarUndoResult {
        decision,
        restored: changed > 0,
    })
}
