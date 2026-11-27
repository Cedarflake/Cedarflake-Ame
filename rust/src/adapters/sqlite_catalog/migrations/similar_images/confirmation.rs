use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::similar_images::SIMILARITY_SCHEME;

use super::super::database_error;

pub(super) const LEGACY_SCHEME: &str = "ame-perceptual-gradient64-v1";
pub(super) const LEGACY_PARAMETERS: &str = "normalization=ame-visual-sample-v1;gradient=8x8;resample=lanczos3;transforms=d4;decoder=image-0.25.10";

const VARIANTS: &str = "CREATE TABLE \"similarity_variants\" (
    evidence_id INTEGER NOT NULL REFERENCES similarity_evidence(evidence_id),
    variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
    word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
    word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
    word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
    word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
    confirmation BLOB CHECK(confirmation IS NULL OR length(confirmation) = 32),
    PRIMARY KEY(evidence_id, variant),
    UNIQUE(evidence_id, word0, word1, word2, word3, confirmation)
) STRICT, WITHOUT ROWID";

const REFERENCES: &str = "CREATE TABLE \"similarity_reference_variants\" (
    group_id TEXT NOT NULL REFERENCES similarity_references(group_id),
    variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
    word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
    word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
    word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
    word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
    confirmation BLOB CHECK(confirmation IS NULL OR length(confirmation) = 32),
    PRIMARY KEY(group_id, variant)
) STRICT, WITHOUT ROWID";

pub(super) fn current_ddl(name: &str, legacy: &str) -> String {
    match name {
        "similarity_variants" => VARIANTS.to_owned(),
        "similarity_reference_variants" => REFERENCES.to_owned(),
        name if name.starts_with("similarity_variants_band")
            || name.starts_with("similarity_references_band") =>
        {
            format!("{legacy} WHERE confirmation IS NOT NULL")
        }
        _ => legacy.to_owned(),
    }
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let incompatible: bool = connection
        .query_row(
            "SELECT EXISTS(
            SELECT 1 FROM similarity_variants AS v
            JOIN similarity_evidence AS e USING(evidence_id)
            WHERE (e.scheme = ?1) <> (v.confirmation IS NOT NULL))
         OR EXISTS(
            SELECT 1 FROM similarity_reference_variants AS v
            JOIN similarity_references AS r USING(group_id)
            JOIN similarity_variants AS original ON original.evidence_id = r.evidence_id
                AND original.variant = v.variant
            WHERE original.confirmation IS NOT v.confirmation)
         OR EXISTS(
            SELECT 1 FROM similarity_variants WHERE confirmation IS NULL
            GROUP BY evidence_id, word0, word1, word2, word3 HAVING COUNT(*) > 1)",
            [SIMILARITY_SCHEME],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if incompatible {
        Err(super::validation::invalid())
    } else {
        Ok(())
    }
}
