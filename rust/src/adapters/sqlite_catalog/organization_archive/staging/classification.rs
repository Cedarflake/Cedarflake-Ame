use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{CategoryId, ClassificationRecordId, Taxonomy};
use crate::domain::organization::validate_identifier;

use super::super::super::classification::validation::{category_version, choice_rules};
use super::super::{codec::Cell, database_error, error};

pub(super) fn record(kind: &str, values: &[Cell]) -> Result<(), ScanError> {
    let ids: &[usize] = match kind {
        "classification_category" | "classification_version" => {
            let Some(Cell::Text(value)) = values.first() else {
                return Err(invalid());
            };
            CategoryId::new(value.clone()).map_err(|_| invalid())?;
            &[]
        }
        "classification_operation" | "classification_change" | "classification_target" => {
            let Some(Cell::Text(value)) = values.first() else {
                return Err(invalid());
            };
            ClassificationRecordId::new(value.clone()).map_err(|_| invalid())?;
            if kind == "classification_operation" {
                &[]
            } else {
                &[1]
            }
        }
        "classification_decision" => &[0],
        "classification_bookmark" => {
            let Some(Cell::Text(payload)) = values.get(1) else {
                return Err(invalid());
            };
            super::super::super::classification::review::scope::decode(payload)?;
            &[0, 3, 4]
        }
        "classification_context" | "classification_rules" | "classification_member" => &[],
        _ => return Err(invalid()),
    };
    for index in ids {
        let Some(Cell::Text(value)) = values.get(*index) else {
            return Err(invalid());
        };
        validate_identifier(value).map_err(|_| invalid())?;
    }
    Ok(())
}

pub(super) fn validate(
    connection: &Connection,
    checkpoint: &mut dyn FnMut() -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let ceiling: Option<i64> = connection
        .query_row(
            "SELECT revision FROM archive_classification_context",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    if ceiling.is_none() {
        let present: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM archive_classification_category)
             OR EXISTS(SELECT 1 FROM archive_classification_operation)
             OR EXISTS(SELECT 1 FROM archive_classification_bookmark)",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if present {
            return Err(invalid());
        }
    }
    for query in RELATIONSHIPS {
        checkpoint()?;
        let invalid: bool = connection
            .query_row(query, [ceiling.unwrap_or(0)], |row| row.get(0))
            .map_err(database_error)?;
        if invalid {
            return Err(self::invalid());
        }
    }
    definitions(connection, checkpoint)?;
    choices(connection, checkpoint)
}

fn definitions(
    connection: &Connection,
    checkpoint: &mut dyn FnMut() -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let mut statement = connection.prepare(
        "SELECT version.category_id, category.label, semantic_revision, descriptions FROM archive_classification_version AS version
         JOIN archive_classification_category AS category USING(category_id)",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        checkpoint()?;
        category_version(
            row.get(0).map_err(database_error)?,
            row.get(1).map_err(database_error)?,
            &row.get::<_, String>(2).map_err(database_error)?,
            &row.get::<_, String>(3).map_err(database_error)?,
        )?;
    }
    let mut statement = connection
        .prepare("SELECT rules_revision FROM archive_classification_rules")
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        checkpoint()?;
        let revision: String = row.get(0).map_err(database_error)?;
        let mut members = connection.prepare_cached(
            "SELECT member.category_id, category.label, member.semantic_revision, version.descriptions
             FROM archive_classification_member AS member JOIN archive_classification_category AS category USING(category_id)
             JOIN archive_classification_version AS version USING(category_id, semantic_revision)
             WHERE member.rules_revision = ?1 ORDER BY member.category_id LIMIT 257",
        ).map_err(database_error)?;
        let mut members = members.query([&revision]).map_err(database_error)?;
        let mut categories = Vec::new();
        while let Some(member) = members.next().map_err(database_error)? {
            categories.push(category_version(
                member.get(0).map_err(database_error)?,
                member.get(1).map_err(database_error)?,
                &member.get::<_, String>(2).map_err(database_error)?,
                &member.get::<_, String>(3).map_err(database_error)?,
            )?);
        }
        let taxonomy = Taxonomy::new(categories).map_err(|_| invalid())?;
        if blake3::Hash::from_bytes(*taxonomy.revision().bytes())
            .to_hex()
            .as_str()
            != revision
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn choices(
    connection: &Connection,
    checkpoint: &mut dyn FnMut() -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT category_id, choice FROM archive_classification_decision
         UNION ALL SELECT json_extract(previous_payload, '$.choice.category'), previous_payload
         FROM archive_classification_change WHERE field = 'category'",
        )
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        checkpoint()?;
        let category: Option<String> = row.get(0).map_err(database_error)?;
        let payload: Option<String> = row.get(1).map_err(database_error)?;
        if let Some(rules) = choice_rules(category.as_deref(), payload.as_deref())? {
            let present: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM archive_classification_member WHERE rules_revision = ?1 AND category_id = ?2)",
                params![blake3::Hash::from_bytes(*rules.bytes()).to_hex().as_str(), category], |row| row.get(0),
            ).map_err(database_error)?;
            if !present {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

const RELATIONSHIPS: &[&str] = &[
    "SELECT EXISTS(SELECT 1 FROM archive_classification_category AS category WHERE revision > ?1
       OR NOT EXISTS(SELECT 1 FROM archive_classification_version WHERE category_id = category.category_id))",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_decision WHERE choice_revision > ?1 OR review_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_operation WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_bookmark WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_bookmark AS bookmark
       WHERE ?1 >= 0 AND (
         (json_extract(scope, '$.root_id') IS NOT NULL AND NOT EXISTS(
           SELECT 1 FROM archive_root_evidence WHERE root_id = json_extract(bookmark.scope, '$.root_id')))
         OR (json_extract(scope, '$.category') IS NOT NULL AND NOT EXISTS(
           SELECT 1 FROM archive_classification_category WHERE category_id = json_extract(bookmark.scope, '$.category')))))",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_change AS change
       JOIN archive_classification_operation AS operation USING(operation_id)
       LEFT JOIN archive_classification_target AS target USING(operation_id, record_id, field)
       WHERE applied_revision > ?1 OR previous_revision >= applied_revision
         OR applied_revision <> operation.revision OR target.operation_id IS NULL)",
    "SELECT EXISTS(SELECT 1 FROM archive_classification_target AS target
       JOIN archive_record AS record USING(record_id)
       LEFT JOIN archive_classification_change AS change USING(operation_id, record_id, field)
       LEFT JOIN archive_classification_decision AS decision ON decision.record_id = target.record_id
       WHERE ?1 >= 0 AND (change.operation_id IS NULL OR decision.record_id IS NULL))",
];

fn invalid() -> ScanError {
    error(
        "records_invalid",
        "The backup contains inconsistent classification records",
    )
}
