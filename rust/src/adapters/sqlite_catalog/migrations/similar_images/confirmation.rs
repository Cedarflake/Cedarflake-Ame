use rusqlite::{Connection, Transaction};

use crate::domain::ScanError;
use crate::domain::similar_images::SIMILARITY_SCHEME;

#[cfg(test)]
use super::super::schema_shape;
use super::super::{current_schema, database_error};

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

pub(in crate::adapters::sqlite_catalog::migrations) fn migrate(
    connection: &mut Connection,
) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog::migrations) fn migrate_transaction(
    transaction: &Transaction<'_>,
) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 74)?;
    for (table, ddl) in [
        ("similarity_variants", VARIANTS),
        ("similarity_reference_variants", REFERENCES),
    ] {
        let replacement = format!("{table}_with_confirmation");
        transaction
            .execute_batch(&ddl.replacen(table, &replacement, 1))
            .map_err(database_error)?;
        transaction
            .execute_batch(&format!(
                "INSERT INTO {replacement} SELECT *, NULL FROM {table};
             DROP TABLE {table};
             ALTER TABLE {replacement} RENAME TO {table};"
            ))
            .map_err(database_error)?;
    }
    for (kind, name, ddl) in super::schema::OBJECTS {
        if *kind == "index"
            && (name.starts_with("similarity_variants_band")
                || name.starts_with("similarity_references_band"))
        {
            transaction
                .execute_batch(&current_ddl(name, ddl))
                .map_err(database_error)?;
        }
    }
    // Old continuations have no confirmation evidence and cannot publish new-scheme members.
    transaction
        .execute_batch(
            "UPDATE similarity_match_work SET phase = 'complete';
         UPDATE schema_info SET version = 75; PRAGMA user_version = 75;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 75)
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

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() != 75 {
        return;
    }
    for table in ["similarity_reference_variants", "similarity_variants"] {
        let ddl = super::schema::OBJECTS
            .iter()
            .find(|(_, name, _)| *name == table)
            .unwrap()
            .2;
        let replacement = format!("{table}_legacy");
        let identity = if table == "similarity_variants" {
            "evidence_id"
        } else {
            "group_id"
        };
        connection
            .execute_batch(&format!(
                "CREATE TEMP TABLE {replacement} AS
                 SELECT {identity}, variant, word0, word1, word2, word3 FROM {table};
                 DROP TABLE {table}; {ddl};
                 INSERT INTO {table} SELECT * FROM {replacement}; DROP TABLE {replacement};"
            ))
            .unwrap();
    }
    for (kind, name, ddl) in super::schema::OBJECTS {
        if *kind == "index"
            && (name.starts_with("similarity_variants_band")
                || name.starts_with("similarity_references_band"))
        {
            connection.execute_batch(ddl).unwrap();
        }
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 74; PRAGMA user_version = 74;")
        .unwrap();
}
