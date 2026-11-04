use rusqlite::{Connection, params};

use crate::adapters::sqlite_catalog::exact_duplicates::read_unsigned;
use crate::domain::ScanError;
use crate::domain::similar_images::{
    SIMILARITY_MAX_DISTANCE, SIMILARITY_PARAMETERS, SIMILARITY_SCHEME, SimilarContentKey, group_id,
};

use super::super::database_error;
use super::super::schema_shape::schema_object_sql_matches;

pub(in crate::adapters::sqlite_catalog::migrations) fn validate(
    connection: &Connection,
) -> Result<(), ScanError> {
    for (kind, name, ddl) in super::schema::OBJECTS {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(in crate::adapters::sqlite_catalog::migrations) fn validate_rows(
    connection: &Connection,
) -> Result<(), ScanError> {
    let incompatible: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM similarity_runs
             WHERE scheme <> ?1 OR engine_id <> 'image_hasher' OR engine_version <> '3.1.1'
               OR parameters <> ?2)
         OR EXISTS(SELECT 1 FROM similarity_evidence AS e
             JOIN similarity_runs AS r USING(run_id) WHERE e.scheme <> r.scheme)
         OR EXISTS(SELECT 1 FROM similarity_groups WHERE scheme <> ?1)
         OR (SELECT COUNT(*) FROM similarity_dispatch) <> 1
         OR EXISTS(SELECT 1 FROM similarity_dispatch AS active
             JOIN similarity_outcomes AS outcome ON outcome.run_id = active.active_run_id)
         OR EXISTS(SELECT 1 FROM pragma_foreign_key_check WHERE [table] LIKE 'similarity_%')",
            params![SIMILARITY_SCHEME, SIMILARITY_PARAMETERS],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if incompatible {
        return Err(invalid());
    }
    validate_content(connection)?;
    validate_groups(connection)?;
    let inconsistent: bool = connection
        .query_row(
            "SELECT EXISTS(
            SELECT 1 FROM similarity_references AS r
            JOIN similarity_groups AS g USING(group_id)
            JOIN similarity_evidence AS e USING(evidence_id)
            WHERE e.disposition <> 'ready' OR e.scheme <> g.scheme
              OR e.fingerprint_scheme <> g.reference_scheme
              OR e.digest <> g.reference_digest OR e.byte_length <> g.reference_byte_length)
         OR EXISTS(
            SELECT 1 FROM similarity_reference_variants AS v
            JOIN similarity_references AS r USING(group_id)
            LEFT JOIN similarity_variants AS original
              ON original.evidence_id = r.evidence_id AND original.variant = v.variant
            WHERE original.word0 IS NOT v.word0 OR original.word1 IS NOT v.word1
              OR original.word2 IS NOT v.word2 OR original.word3 IS NOT v.word3)
         OR EXISTS(
            SELECT 1 FROM similarity_references AS r
            JOIN similarity_variants AS original USING(evidence_id)
            LEFT JOIN similarity_reference_variants AS v
              ON v.group_id = r.group_id AND v.variant = original.variant
            WHERE v.group_id IS NULL)
         OR EXISTS(
            SELECT 1 FROM similarity_members AS m
            JOIN similarity_groups AS g USING(group_id)
            JOIN similarity_evidence AS e USING(evidence_id)
            JOIN similarity_runs AS run ON run.run_id = m.run_id
            WHERE e.disposition <> 'ready' OR e.scheme <> g.scheme OR run.scheme <> g.scheme
              OR m.distance NOT BETWEEN 0 AND ?1)
         OR EXISTS(
            SELECT 1 FROM similarity_match_work AS work
            JOIN similarity_evidence AS e USING(evidence_id)
            WHERE e.disposition <> 'ready'
              OR (work.phase = 'members' AND NOT EXISTS(
                  SELECT 1 FROM similarity_references AS r WHERE r.evidence_id = e.evidence_id)))",
            [i64::from(SIMILARITY_MAX_DISTANCE)],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if inconsistent { Err(invalid()) } else { Ok(()) }
}

fn validate_content(connection: &Connection) -> Result<(), ScanError> {
    let invalid_variants: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM similarity_evidence AS e WHERE
            (e.disposition = 'ready' AND (
                (SELECT COUNT(*) FROM similarity_variants AS v WHERE v.evidence_id = e.evidence_id)
                    NOT BETWEEN 1 AND 8
                OR NOT EXISTS(SELECT 1 FROM similarity_variants AS v WHERE v.evidence_id = e.evidence_id
                    AND (word0 <> 0 OR word1 <> 0 OR word2 <> 0 OR word3 <> 0)
                    AND (word0 <> 65535 OR word1 <> 65535 OR word2 <> 65535 OR word3 <> 65535))))
            OR (e.disposition <> 'ready' AND EXISTS(
                SELECT 1 FROM similarity_variants AS v WHERE v.evidence_id = e.evidence_id)))",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if invalid_variants {
        return Err(invalid());
    }
    let mut statement = connection
        .prepare(
            "SELECT fingerprint_scheme, digest, byte_length FROM similarity_evidence
         UNION ALL SELECT fingerprint_scheme, digest, byte_length FROM similarity_exclusions
         UNION ALL SELECT preferred_scheme, preferred_digest, preferred_byte_length
           FROM similarity_decisions WHERE preferred_scheme IS NOT NULL",
        )
        .map_err(database_error)?;
    let contents = statement
        .query_map([], |row| {
            Ok(SimilarContentKey {
                scheme: row.get(0)?,
                digest: row.get(1)?,
                byte_length: read_unsigned(row, 2)?,
            })
        })
        .map_err(database_error)?;
    for content in contents {
        content.map_err(database_error)?.validate()?;
    }
    Ok(())
}

fn validate_groups(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT group_id, scheme, reference_scheme, reference_digest, reference_byte_length
         FROM similarity_groups",
        )
        .map_err(database_error)?;
    let groups = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                SimilarContentKey {
                    scheme: row.get(2)?,
                    digest: row.get(3)?,
                    byte_length: read_unsigned(row, 4)?,
                },
            ))
        })
        .map_err(database_error)?;
    for group in groups {
        let (stored, scheme, reference) = group.map_err(database_error)?;
        if stored != group_id(&scheme, &reference)? {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_similarity_contract_invalid",
        "Similarity evidence and durable choices require consistent versioned identities",
    )
}
