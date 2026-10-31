use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{CategoryDefinition, ClassificationRecordId, Taxonomy};
use crate::domain::classification_catalog::{
    ClassificationCatalogState, ClassificationWriteReceipt,
};

use super::super::{database_error, exact_duplicates::read_unsigned};
use super::{codec, invalid_record, operations};

pub(super) fn load(connection: &Connection) -> Result<ClassificationCatalogState, ScanError> {
    let (revision, rules, configuration, active_run_id): (u64, Option<String>, Option<String>, Option<String>) = connection.query_row(
        "SELECT revision, active_rules, configuration_revision, active_run_id FROM classification_state WHERE singleton = 1",
        [], |row| Ok((read_unsigned(row, 0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).map_err(database_error)?;
    let taxonomy = rules
        .as_ref()
        .map(|rules| load_rules(connection, rules))
        .transpose()?;
    Ok(ClassificationCatalogState {
        revision,
        taxonomy,
        configuration: configuration
            .as_deref()
            .map(codec::read_revision)
            .transpose()?,
        active_run_id,
    })
}

pub(super) fn load_rules(connection: &Connection, revision: &str) -> Result<Taxonomy, ScanError> {
    let mut statement = connection.prepare_cached(
        "SELECT member.category_id, category.label, version.descriptions
         FROM classification_rule_members AS member
         JOIN classification_categories AS category USING(category_id)
         JOIN classification_category_versions AS version
           ON version.category_id = member.category_id AND version.semantic_revision = member.semantic_revision
         WHERE member.rules_revision = ?1 ORDER BY member.category_id LIMIT 257",
    ).map_err(database_error)?;
    let rows = statement
        .query_map([revision], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(database_error)?;
    let mut categories = Vec::new();
    for row in rows {
        let (id, label, descriptions) = row.map_err(database_error)?;
        categories.push(
            CategoryDefinition::new(
                codec::category_id(id)?,
                label,
                codec::decode(&descriptions, 65536)?,
            )
            .map_err(|_| invalid_record())?,
        );
    }
    let taxonomy = Taxonomy::new(categories).map_err(|_| invalid_record())?;
    if codec::revision(taxonomy.revision()) != revision {
        return Err(invalid_record());
    }
    Ok(taxonomy)
}

pub(super) fn configure(
    connection: &Connection,
    id: &ClassificationRecordId,
    expected_revision: u64,
    taxonomy: &Taxonomy,
) -> Result<ClassificationWriteReceipt, ScanError> {
    let digest = request_digest(expected_revision, taxonomy);
    if let Some(receipt) = operations::replay(connection, id, &digest)? {
        return Ok(receipt);
    }
    operations::require_revision(connection, expected_revision)?;
    let previous = load(connection)?;
    let revision = operations::begin(connection, id, &digest, "configure")?;
    let changed = apply_rules(connection, &previous, taxonomy, revision)?;
    operations::finish(connection, id, revision, 0, changed)
}

pub(super) fn apply_rules(
    connection: &Connection,
    previous: &ClassificationCatalogState,
    taxonomy: &Taxonomy,
    revision: i64,
) -> Result<bool, ScanError> {
    let changed = previous.taxonomy.as_ref() != Some(taxonomy);
    if changed {
        save_rules(connection, taxonomy, revision, true)?;
        let semantic_change =
            previous.taxonomy.as_ref().map(Taxonomy::revision) != Some(taxonomy.revision());
        connection
            .execute(
                "UPDATE classification_state SET active_rules = ?1,
             configuration_revision = CASE WHEN ?2 THEN NULL ELSE configuration_revision END,
             active_run_id = CASE WHEN ?2 THEN NULL ELSE active_run_id END WHERE singleton = 1",
                params![codec::revision(taxonomy.revision()), semantic_change],
            )
            .map_err(database_error)?;
    }
    Ok(changed)
}

pub(super) fn save_rules(
    connection: &Connection,
    taxonomy: &Taxonomy,
    revision: i64,
    update_labels: bool,
) -> Result<(), ScanError> {
    for category in taxonomy.categories() {
        connection.execute(
            "INSERT INTO classification_categories(category_id, label, revision) VALUES (?1, ?2, ?3)
             ON CONFLICT(category_id) DO UPDATE SET label = excluded.label, revision = excluded.revision WHERE ?4",
            params![category.id().as_str(), category.label(), revision, update_labels],
        ).map_err(database_error)?;
        let descriptions = codec::encode(&category.descriptions(), 65536)?;
        let semantic = codec::revision(category.semantic_revision());
        let existing: Option<String> = connection.query_row(
            "SELECT descriptions FROM classification_category_versions WHERE category_id = ?1 AND semantic_revision = ?2",
            params![category.id().as_str(), semantic], |row| row.get(0),
        ).optional().map_err(database_error)?;
        if existing
            .as_ref()
            .is_some_and(|saved| saved != &descriptions)
        {
            return Err(invalid_record());
        }
        connection.execute(
            "INSERT OR IGNORE INTO classification_category_versions(category_id, semantic_revision, descriptions) VALUES (?1, ?2, ?3)",
            params![category.id().as_str(), semantic, descriptions],
        ).map_err(database_error)?;
    }
    let rules = codec::revision(taxonomy.revision());
    connection
        .execute(
            "INSERT OR IGNORE INTO classification_rules VALUES (?1)",
            [&rules],
        )
        .map_err(database_error)?;
    for category in taxonomy.categories() {
        connection
            .execute(
                "INSERT OR IGNORE INTO classification_rule_members VALUES (?1, ?2, ?3)",
                params![
                    rules,
                    category.id().as_str(),
                    codec::revision(category.semantic_revision())
                ],
            )
            .map_err(database_error)?;
    }
    if load_rules(connection, &rules)?.revision() != taxonomy.revision() {
        return Err(invalid_record());
    }
    Ok(())
}

fn request_digest(expected_revision: u64, taxonomy: &Taxonomy) -> String {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.configure.v1");
    operations::field(&mut hash, &expected_revision.to_le_bytes());
    operations::field(&mut hash, taxonomy.revision().bytes());
    for category in taxonomy.categories() {
        operations::field(&mut hash, category.id().as_str().as_bytes());
        operations::field(&mut hash, category.label().as_bytes());
    }
    hash.finalize().to_hex().to_string()
}
