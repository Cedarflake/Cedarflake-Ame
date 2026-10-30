use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::classification::{
    CategoryDefinition, ClassificationRevision, HumanCategoryChoice,
};

use super::super::{database_error, exact_duplicates::read_unsigned, sqlite_integer};
use super::{codec, configuration, invalid_record, review};

pub(in crate::adapters::sqlite_catalog) fn category_version(
    id: String,
    label: String,
    semantic: &str,
    descriptions: &str,
) -> Result<CategoryDefinition, ScanError> {
    let values: Vec<String> = codec::decode(descriptions, 65536)?;
    let definition =
        CategoryDefinition::new(codec::category_id(id)?, label.clone(), values.clone())
            .map_err(|_| invalid_record())?;
    if codec::revision(definition.semantic_revision()) != semantic
        || definition.descriptions() != values
        || definition.label() != label
    {
        return Err(invalid_record());
    }
    Ok(definition)
}

pub(in crate::adapters::sqlite_catalog) fn choice_rules(
    category: Option<&str>,
    payload: Option<&str>,
) -> Result<Option<ClassificationRevision>, ScanError> {
    match (category, payload) {
        (None, None) => Ok(None),
        (Some(category), Some(payload)) => {
            let choice = codec::decode_choice(payload)?;
            if choice.category().as_str() != category {
                return Err(invalid_record());
            }
            Ok(Some(match choice {
                HumanCategoryChoice::Confirmed(reference) => reference.primary_rules,
                HumanCategoryChoice::Corrected { rules, .. } => rules,
            }))
        }
        _ => Err(invalid_record()),
    }
}

pub(in crate::adapters::sqlite_catalog) fn catalog(
    connection: &Connection,
) -> Result<(), ScanError> {
    let state = configuration::load(connection)?;
    let mut statement = connection.prepare(
        "SELECT version.category_id, category.label, semantic_revision, descriptions
         FROM classification_category_versions AS version JOIN classification_categories AS category USING(category_id)",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        category_version(
            row.get(0).map_err(database_error)?,
            row.get(1).map_err(database_error)?,
            &row.get::<_, String>(2).map_err(database_error)?,
            &row.get::<_, String>(3).map_err(database_error)?,
        )?;
    }
    let mut statement = connection
        .prepare("SELECT rules_revision FROM classification_rules")
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        configuration::load_rules(
            connection,
            &row.get::<_, String>(0).map_err(database_error)?,
        )?;
    }
    validate_choices(connection)?;
    validate_evidence(connection)?;
    validate_continuation(connection)?;
    for query in REVISION_CHECKS {
        let invalid: bool = connection
            .query_row(
                query,
                [sqlite_integer(state.revision, "classification revision")?],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if invalid {
            return Err(invalid_record());
        }
    }
    Ok(())
}

fn validate_choices(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT category_id, choice FROM classification_decisions
         UNION ALL SELECT json_extract(previous_payload, '$.choice.category'), previous_payload
         FROM classification_operation_changes WHERE field = 'category'",
        )
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let category: Option<String> = row.get(0).map_err(database_error)?;
        let payload: Option<String> = row.get(1).map_err(database_error)?;
        if let Some(rules) = choice_rules(category.as_deref(), payload.as_deref())? {
            let present: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM classification_rule_members WHERE rules_revision = ?1 AND category_id = ?2)",
                params![codec::revision(rules), category], |row| row.get(0),
            ).map_err(database_error)?;
            if !present {
                return Err(invalid_record());
            }
        }
    }
    Ok(())
}

fn validate_evidence(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection.prepare("SELECT run_id, primary_rules, configuration_revision, payload FROM classification_runs").map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let run = codec::decode_run(&row.get::<_, String>(3).map_err(database_error)?)?;
        if run.id().as_str() != row.get::<_, String>(0).map_err(database_error)?
            || codec::revision(run.primary_rules())
                != row.get::<_, String>(1).map_err(database_error)?
            || codec::revision(run.configuration())
                != row.get::<_, String>(2).map_err(database_error)?
        {
            return Err(invalid_record());
        }
        if let Some(refinement) = run.refinement() {
            configuration::load_rules(connection, &codec::revision(refinement.rules))?;
        }
    }
    let mut statement = connection.prepare(
        "SELECT prediction.prediction_id, prediction.configuration_revision, fingerprint_scheme, digest, byte_length,
                prediction.payload, run.payload FROM classification_predictions AS prediction
         JOIN classification_runs AS run USING(run_id)",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let run = codec::decode_run(&row.get::<_, String>(6).map_err(database_error)?)?;
        let evidence =
            codec::prediction::load(&row.get::<_, String>(5).map_err(database_error)?, run)?;
        let content = evidence.content();
        let taxonomy = configuration::load_rules(
            connection,
            &codec::revision(evidence.run().primary_rules()),
        )?;
        let expected: std::collections::BTreeSet<_> = taxonomy
            .categories()
            .iter()
            .map(|value| value.id())
            .collect();
        let actual: std::collections::BTreeSet<_> = evidence
            .initial()
            .categories()
            .iter()
            .map(|value| &value.category_id)
            .collect();
        if expected != actual {
            return Err(invalid_record());
        }
        if evidence.id().as_str() != row.get::<_, String>(0).map_err(database_error)?
            || codec::revision(evidence.run().configuration())
                != row.get::<_, String>(1).map_err(database_error)?
            || content.scheme.as_str() != row.get::<_, String>(2).map_err(database_error)?
            || codec::digest(&content.digest) != row.get::<_, String>(3).map_err(database_error)?
            || content.byte_length != read_unsigned(row, 4).map_err(database_error)?
        {
            return Err(invalid_record());
        }
    }
    Ok(())
}

fn validate_continuation(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection.prepare(
        "SELECT scope FROM classification_review_bookmark UNION ALL SELECT scope FROM classification_review_session"
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let scope = review::scope::decode(&row.get::<_, String>(0).map_err(database_error)?)?;
        if let Some(category) = scope.category {
            let present: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM classification_categories WHERE category_id = ?1)",
                    [category.as_str()],
                    |row| row.get(0),
                )
                .map_err(database_error)?;
            if !present {
                return Err(invalid_record());
            }
        }
    }
    if let Some(bookmark) = review::position::load(connection)? {
        for id in [
            &bookmark.record_id,
            &bookmark.position.root_id,
            &bookmark.position.location_id,
        ] {
            crate::domain::organization::validate_identifier(id)?;
        }
    }
    Ok(())
}

const REVISION_CHECKS: &[&str] = &[
    "SELECT EXISTS(SELECT 1 FROM classification_categories AS category WHERE revision > ?1
       OR NOT EXISTS(SELECT 1 FROM classification_category_versions WHERE category_id = category.category_id))",
    "SELECT EXISTS(SELECT 1 FROM classification_decisions WHERE choice_revision > ?1 OR review_revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM classification_operations WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM classification_operation_changes AS change JOIN classification_operations AS operation USING(operation_id)
     WHERE applied_revision > ?1 OR applied_revision <> operation.revision OR previous_revision >= applied_revision)",
    "SELECT EXISTS(SELECT 1 FROM classification_review_bookmark WHERE revision > ?1)",
    "SELECT EXISTS(SELECT 1 FROM classification_state AS state JOIN classification_runs AS run ON run.run_id = state.active_run_id
       WHERE ?1 >= 0 AND state.active_rules IS NOT run.primary_rules)",
    "SELECT EXISTS(SELECT 1 FROM classification_operation_changes AS change
       LEFT JOIN classification_operation_targets AS target USING(operation_id, record_id, field)
       WHERE ?1 >= 0 AND target.operation_id IS NULL)",
    "SELECT EXISTS(SELECT 1 FROM classification_operation_targets AS target
       JOIN organization_records AS record USING(record_id)
       LEFT JOIN classification_operation_changes AS change USING(operation_id, record_id, field)
       LEFT JOIN classification_decisions AS decision ON decision.record_id = target.record_id
       WHERE ?1 >= 0 AND (change.operation_id IS NULL OR decision.record_id IS NULL))",
];
