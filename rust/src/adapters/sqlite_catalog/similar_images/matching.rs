use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::ScanError;
use crate::domain::similar_images::matching::{MatchPhase, band_neighbors, distance};
use crate::domain::similar_images::{
    SIMILARITY_MAX_DISTANCE, SIMILARITY_SCHEME, SimilarAnalysisRun, SimilarContentKey, group_id,
};

use super::super::{database_error, exact_duplicates::read_unsigned};

mod cursor;
mod postings;
use cursor::MatchWork;

pub(super) fn advance(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
    cancelled: &AtomicBool,
) -> Result<bool, ScanError> {
    let Some(mut work) = cursor::load(transaction)? else {
        return Ok(false);
    };
    let revision: i64 = transaction
        .query_row(
            "SELECT reference_revision FROM similarity_dispatch WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if work.phase != MatchPhase::Members && work.reference_revision != revision {
        work.restart(MatchPhase::References);
        work.found_reference = false;
        work.reference_revision = revision;
    }
    let variants = postings::variants(transaction, work.evidence_id)?;
    let query = variants.get(work.query_variant).ok_or_else(invalid)?;
    let neighbors = band_neighbors(query[work.band]);
    let rows = postings::load(transaction, &work, &neighbors)?;
    let reference = if work.phase == MatchPhase::Members {
        Some(
            transaction
                .query_row(
                    "SELECT group_id FROM similarity_references WHERE evidence_id = ?1",
                    [work.evidence_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(database_error)?,
        )
    } else {
        None
    };
    for row in &rows {
        if cancelled.load(Ordering::Acquire) {
            return Ok(false);
        }
        let separation = distance(query, &row.words);
        if separation <= SIMILARITY_MAX_DISTANCE {
            match work.phase {
                MatchPhase::References => {
                    publish_member(
                        transaction,
                        run,
                        &row.group_id,
                        work.evidence_id,
                        separation,
                    )?;
                    work.found_reference = true;
                }
                MatchPhase::Peer if row.evidence_id != work.evidence_id => {
                    let group = establish_reference(transaction, &work)?;
                    publish_member(transaction, run, &group, work.evidence_id, 0)?;
                    work.restart(MatchPhase::Members);
                    cursor::save(transaction, &work)?;
                    return Ok(true);
                }
                MatchPhase::Members => {
                    publish_member(
                        transaction,
                        run,
                        reference.as_deref().ok_or_else(invalid)?,
                        row.evidence_id,
                        separation,
                    )?;
                }
                _ => {}
            }
        }
        work.neighbor = neighbors
            .binary_search(&row.words[work.band])
            .map_err(|_| invalid())?;
        work.after_evidence_id = row.evidence_id;
        work.after_group_id.clone_from(&row.group_id);
        work.after_variant = row.variant;
    }
    if rows.len() < postings::WINDOW_SIZE && !work.next_band(variants.len()) {
        work.restart(work.phase.exhausted(work.found_reference));
    }
    cursor::save(transaction, &work)?;
    Ok(true)
}

fn establish_reference(
    transaction: &Transaction<'_>,
    work: &MatchWork,
) -> Result<String, ScanError> {
    let content = transaction.query_row(
        "SELECT fingerprint_scheme, digest, byte_length FROM similarity_evidence WHERE evidence_id = ?1",
        [work.evidence_id], |row| Ok(SimilarContentKey { scheme: row.get(0)?, digest: row.get(1)?, byte_length: read_unsigned(row, 2)? }),
    ).map_err(database_error)?;
    let group = group_id(SIMILARITY_SCHEME, &content)?;
    transaction
        .execute(
            "INSERT INTO similarity_groups VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                group,
                SIMILARITY_SCHEME,
                content.scheme,
                content.digest,
                content.byte_length as i64
            ],
        )
        .map_err(database_error)?;
    super::evidence::admit_reference(transaction, work.evidence_id, &group)?;
    Ok(group)
}

fn publish_member(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
    group: &str,
    evidence_id: i64,
    distance: u8,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO similarity_members VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(group_id, evidence_id) DO UPDATE SET
           distance = MIN(distance, excluded.distance), run_id = excluded.run_id",
            params![group, evidence_id, distance, run.run_id],
        )
        .map_err(database_error)?;
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "similarity_cursor_invalid",
        "Similarity matching requires a valid bounded continuation",
    )
}
