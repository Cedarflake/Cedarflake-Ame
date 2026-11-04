use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};

use crate::domain::ScanError;
use crate::domain::similar_images::review::{
    SimilarExclusionResult, SimilarGroupDecision, SimilarMemberExclusion, SimilarMemberSelection,
    SimilarReviewContext, SimilarUndoResult,
};
use crate::domain::similar_images::{SIMILARITY_SCHEME, SimilarContentKey};

use super::super::super::exact_duplicates::read_unsigned;
use super::super::super::{database_error, sqlite_integer};
use super::{projection, queries};

pub(super) fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<SimilarGroupDecision> {
    let scheme: Option<String> = row.get(start)?;
    let preferred = scheme
        .map(|scheme| -> rusqlite::Result<_> {
            Ok(SimilarContentKey {
                scheme,
                digest: row.get(start + 1)?,
                byte_length: read_unsigned(row, start + 2)?,
            })
        })
        .transpose()?;
    Ok(SimilarGroupDecision {
        preferred,
        ignored: row.get(start + 3)?,
        revision: read_unsigned(row, start + 4)?,
    })
}

pub(super) fn load(
    connection: &Connection,
    group_id: &str,
) -> Result<SimilarGroupDecision, ScanError> {
    connection
        .query_row(
            "SELECT choice.preferred_scheme, choice.preferred_digest, choice.preferred_byte_length,
                COALESCE(choice.ignored, 0), COALESCE(choice.revision, 0)
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
    let preferred = decision.preferred.as_ref();
    transaction.execute(
        "INSERT INTO similarity_decisions VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(group_id) DO UPDATE SET preferred_scheme = excluded.preferred_scheme,
           preferred_digest = excluded.preferred_digest, preferred_byte_length = excluded.preferred_byte_length,
           ignored = excluded.ignored, revision = excluded.revision",
        params![
            group_id,
            preferred.map(|key| key.scheme.as_str()), preferred.map(|key| key.digest.as_str()),
            preferred.map(|key| sqlite_integer(key.byte_length, "content length")).transpose()?,
            decision.ignored, sqlite_integer(decision.revision, "decision revision")?,
        ],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn prefer(
    transaction: &Transaction<'_>,
    context: &SimilarReviewContext,
    selection: Option<&SimilarMemberSelection>,
) -> Result<SimilarGroupDecision, ScanError> {
    let mut decision = current(transaction, context)?;
    if let Some(selection) = selection {
        projection::require_selected(transaction, context, selection)?;
    }
    let preferred = selection.map(|selection| selection.content.clone());
    if preferred != decision.preferred {
        decision.preferred = preferred;
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
