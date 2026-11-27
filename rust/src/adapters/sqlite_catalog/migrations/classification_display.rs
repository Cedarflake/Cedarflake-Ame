use rusqlite::{Connection, TransactionBehavior};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, SQLITE_APPLICATION_ID, database_error, schema_shape};

const OBJECTS: &[(&str, &str, &str)] = &[
    ("table", "classification_display_state", "CREATE TABLE classification_display_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        generation INTEGER NOT NULL CHECK(generation > 0),
        run_id TEXT REFERENCES classification_runs(run_id),
        rules_revision TEXT REFERENCES classification_rules(rules_revision),
        configuration_revision TEXT CHECK(configuration_revision IS NULL OR length(configuration_revision) = 64)
    ) STRICT"),
    ("table", "classification_display_predictions", "CREATE TABLE classification_display_predictions (
        fingerprint_scheme TEXT NOT NULL,
        digest TEXT NOT NULL,
        byte_length INTEGER NOT NULL,
        publication_id INTEGER NOT NULL REFERENCES classification_predictions(publication_id) ON DELETE CASCADE,
        generation INTEGER NOT NULL CHECK(generation > 0),
        category_id TEXT NOT NULL REFERENCES classification_categories(category_id),
        review TEXT NOT NULL CHECK(review IN ('suggested', 'needs_review')),
        PRIMARY KEY(fingerprint_scheme, digest, byte_length)
    ) STRICT, WITHOUT ROWID"),
    ("table", "classification_display_category_floors", "CREATE TABLE classification_display_category_floors (
        category_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_categories(category_id),
        first_publication_id INTEGER NOT NULL CHECK(first_publication_id > 0)
    ) STRICT, WITHOUT ROWID"),
    ("view", "classification_current_display_projection", "CREATE VIEW classification_current_display_projection AS
        SELECT prediction.* FROM classification_prediction_projection AS prediction
        LEFT JOIN classification_display_category_floors AS floor USING(category_id)
        WHERE prediction.publication_id >= COALESCE(floor.first_publication_id, 1)"),
    ("view", "classification_retained_display_projection", "CREATE VIEW classification_retained_display_projection AS
        SELECT display.publication_id, display.fingerprint_scheme, display.digest,
          display.byte_length, display.category_id, display.review
        FROM classification_display_predictions AS display
        JOIN classification_display_state AS retained ON retained.singleton = 1
          AND retained.generation = display.generation
        JOIN classification_state AS state ON state.singleton = 1
          AND state.active_rules = retained.rules_revision
          AND state.configuration_revision IS retained.configuration_revision
        JOIN classification_rule_members AS member ON member.rules_revision = state.active_rules
          AND member.category_id = display.category_id
        LEFT JOIN classification_display_category_floors AS floor ON floor.category_id = display.category_id
        WHERE display.publication_id >= COALESCE(floor.first_publication_id, 1)
          AND NOT EXISTS (
            SELECT 1 FROM classification_current_display_projection AS current
            WHERE current.fingerprint_scheme = display.fingerprint_scheme
              AND current.digest = display.digest AND current.byte_length = display.byte_length
          )"),
    ("view", "classification_display_projection", "CREATE VIEW classification_display_projection AS
        SELECT * FROM classification_current_display_projection
        UNION ALL SELECT * FROM classification_retained_display_projection"),
];

pub(super) fn create(connection: &Connection) -> Result<(), ScanError> {
    for (_, _, statement) in OBJECTS {
        connection
            .execute_batch(statement)
            .map_err(database_error)?;
    }
    connection
        .execute_batch(
            "INSERT INTO classification_display_state
         SELECT 1, 1, active_run_id, active_rules, configuration_revision FROM classification_state;
         INSERT INTO classification_display_predictions
         SELECT fingerprint_scheme, digest, byte_length, publication_id, 1, category_id, review
         FROM classification_prediction_projection
         WHERE publication_id IN (
           SELECT MAX(publication_id) FROM classification_prediction_projection
           GROUP BY fingerprint_scheme, digest, byte_length
         );",
        )
        .map_err(database_error)
}

pub(super) fn upgrade(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let markers: (i64, i64, i64) = transaction
        .query_row(
            "SELECT (SELECT CASE WHEN COUNT(*) = 1 THEN MIN(version) ELSE -1 END FROM schema_info),
          (SELECT user_version FROM pragma_user_version),
          (SELECT application_id FROM pragma_application_id)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(database_error)?;
    if markers == (SCHEMA_VERSION, SCHEMA_VERSION, SQLITE_APPLICATION_ID) {
        super::current_schema::validate_current_schema_contract(&transaction)?;
        return transaction.commit().map_err(database_error);
    }
    if markers != (83, 83, SQLITE_APPLICATION_ID) {
        return Err(ScanError::new(
            "catalog_schema_unsupported",
            "The catalog upgrade markers are inconsistent",
        ));
    }
    super::current_schema::validate_in_snapshot(&transaction, 83)?;
    create(&transaction)?;
    transaction
        .execute("UPDATE schema_info SET version = ?1", [SCHEMA_VERSION])
        .map_err(database_error)?;
    transaction
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(database_error)?;
    super::current_schema::validate_current_schema_contract(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, statement) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, statement)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let valid: bool = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM classification_display_state) = 1
          AND NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_display_state'))
          AND NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_display_predictions'))
          AND NOT EXISTS(SELECT 1 FROM pragma_foreign_key_check('classification_display_category_floors'))
          AND NOT EXISTS(
            SELECT 1 FROM classification_display_predictions AS display
            JOIN classification_predictions AS prediction USING(publication_id)
            CROSS JOIN classification_display_state AS state
            JOIN classification_runs AS run ON run.run_id = prediction.run_id
            LEFT JOIN classification_runs AS origin ON origin.run_id = state.run_id
            WHERE display.fingerprint_scheme <> prediction.fingerprint_scheme
              OR display.digest <> prediction.digest OR display.byte_length <> prediction.byte_length
              OR display.generation > state.generation
              OR display.category_id IS NOT (
                SELECT json_extract(score.value, '$.category')
                FROM json_each(COALESCE(json_extract(prediction.payload, '$.refined.categories'),
                  json_extract(prediction.payload, '$.initial.categories'))) AS score
                ORDER BY json_extract(score.value, '$.similarity') DESC,
                  json_extract(score.value, '$.category') COLLATE BINARY LIMIT 1)
              OR display.review <> CASE json_extract(prediction.payload, '$.review')
                WHEN 'routine_sampling' THEN 'suggested' ELSE 'needs_review' END
              OR (display.generation = state.generation AND (
                origin.run_id IS NULL
                OR json_extract(run.payload, '$.embedding_space') IS NOT json_extract(origin.payload, '$.embedding_space')
                OR json_extract(run.payload, '$.image_pipeline') IS NOT json_extract(origin.payload, '$.image_pipeline')
                OR json_extract(run.payload, '$.text_pipeline') IS NOT json_extract(origin.payload, '$.text_pipeline')
                OR json_extract(run.payload, '$.dimensions') IS NOT json_extract(origin.payload, '$.dimensions')
              ))
          )", [], |row| row.get(0),
    ).map_err(database_error)?;
    if valid { Ok(()) } else { Err(invalid()) }
}

fn invalid() -> ScanError {
    ScanError::new(
        "classification_display_invalid",
        "Classification display evidence is inconsistent",
    )
}

#[cfg(test)]
mod tests;
