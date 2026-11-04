use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;
use crate::domain::similar_images::{
    SIMILARITY_SCHEME, SimilarAnalysisRun, SimilarContentKey, SimilarEvidence, group_id,
};

use super::super::{database_error, sqlite_integer};

pub(super) fn contains(
    connection: &Connection,
    content: &SimilarContentKey,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM similarity_evidence
         WHERE scheme = ?1 AND fingerprint_scheme = ?2 AND digest = ?3 AND byte_length = ?4)",
            params![
                SIMILARITY_SCHEME,
                content.scheme,
                content.digest,
                sqlite_integer(content.byte_length, "content length")?
            ],
            |row| row.get(0),
        )
        .map_err(database_error)
}

pub(super) fn publish(
    transaction: &Transaction<'_>,
    run: &SimilarAnalysisRun,
    content: &SimilarContentKey,
    evidence: &SimilarEvidence,
) -> Result<bool, ScanError> {
    let (disposition, issue) = match evidence {
        SimilarEvidence::Ready { .. } => ("ready", None),
        SimilarEvidence::InsufficientStructure => ("insufficient_structure", None),
        SimilarEvidence::DecodeFailed { issue_code } => {
            ("decode_failed", Some(issue_code.as_str()))
        }
    };
    let inserted = transaction.execute(
        "INSERT INTO similarity_evidence(run_id, scheme, fingerprint_scheme, digest, byte_length, disposition, issue_code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(scheme, fingerprint_scheme, digest, byte_length) DO NOTHING",
        params![run.run_id, SIMILARITY_SCHEME, content.scheme, content.digest, sqlite_integer(content.byte_length,"content length")?, disposition, issue],
    ).map_err(database_error)?;
    if inserted == 0 {
        return Ok(false);
    }
    let id = transaction.last_insert_rowid();
    if let SimilarEvidence::Ready { variants } = evidence {
        for (variant, words) in variants.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO similarity_variants VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, variant as i64, words[0], words[1], words[2], words[3]],
                )
                .map_err(database_error)?;
        }
        let group = group_id(SIMILARITY_SCHEME, content)?;
        let restored: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM similarity_groups WHERE group_id = ?1)",
                [&group],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if restored {
            admit_reference(transaction, id, &group)?;
        }
        transaction
            .execute(
                "INSERT INTO similarity_match_work
             SELECT ?1, ?2, reference_revision, 0, 0, 0, 0, '', -1, 0 FROM similarity_dispatch",
                params![id, if restored { "members" } else { "references" }],
            )
            .map_err(database_error)?;
    }
    Ok(true)
}

pub(super) fn admit_reference(
    transaction: &Transaction<'_>,
    evidence_id: i64,
    group: &str,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "INSERT INTO similarity_references VALUES (?1, ?2)",
            params![group, evidence_id],
        )
        .map_err(database_error)?;
    transaction.execute(
        "INSERT INTO similarity_reference_variants
         SELECT ?1, variant, word0, word1, word2, word3 FROM similarity_variants WHERE evidence_id = ?2",
        params![group, evidence_id],
    ).map_err(database_error)?;
    transaction.execute("UPDATE similarity_dispatch SET reference_revision = reference_revision + 1 WHERE singleton = 1", []).map_err(database_error)?;
    Ok(())
}
