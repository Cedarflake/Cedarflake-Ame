use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;

use super::super::{database_error, error};

pub(super) mod bookmark;
mod history;

pub(super) fn apply(connection: &Connection) -> Result<Option<i64>, ScanError> {
    let source_revision: Option<i64> = connection
        .query_row(
            "SELECT revision FROM recovery.archive_classification_context",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    let Some(source_revision) = source_revision else {
        return Ok(None);
    };
    let current: i64 = connection
        .query_row(
            "SELECT revision FROM classification_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let base = current.checked_add(1).ok_or_else(revision_exhausted)?;
    let completed = base
        .checked_add(source_revision)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(revision_exhausted)?;
    definitions(connection, base)?;
    choices(connection, base)?;
    history::apply(connection, base)?;
    connection.execute(
        "UPDATE classification_state SET revision = ?1,
         active_rules = COALESCE(active_rules, (SELECT active_rules FROM recovery.archive_classification_context))
         WHERE singleton = 1", [completed],
    ).map_err(database_error)?;
    connection
        .execute("DELETE FROM classification_review_session", [])
        .map_err(database_error)?;
    Ok(Some(base))
}

fn definitions(connection: &Connection, base: i64) -> Result<(), ScanError> {
    let conflicts: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM recovery.archive_classification_version AS source
           JOIN classification_category_versions AS current USING(category_id, semantic_revision)
           WHERE source.descriptions <> current.descriptions)
         OR EXISTS(SELECT 1 FROM recovery.archive_classification_member AS source
           JOIN classification_rule_members AS current USING(rules_revision, category_id)
           WHERE source.semantic_revision <> current.semantic_revision)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if conflicts {
        return Err(error(
            "classification_conflict",
            "Category history conflicts with the retained definitions",
        ));
    }
    connection
        .execute(
            "INSERT INTO classification_categories(category_id, label, revision)
         SELECT category_id, label, ?1 + revision FROM recovery.archive_classification_category WHERE 1
         ON CONFLICT(category_id) DO NOTHING",
            [base],
        )
        .map_err(database_error)?;
    connection.execute_batch(
        "INSERT INTO classification_category_versions SELECT * FROM recovery.archive_classification_version WHERE 1
           ON CONFLICT(category_id, semantic_revision) DO NOTHING;
         INSERT INTO classification_rules SELECT * FROM recovery.archive_classification_rules WHERE 1
           ON CONFLICT(rules_revision) DO NOTHING;
         INSERT INTO classification_rule_members SELECT * FROM recovery.archive_classification_member WHERE 1
           ON CONFLICT(rules_revision, category_id) DO NOTHING;",
    ).map_err(database_error)
}

fn choices(connection: &Connection, base: i64) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO classification_decisions(record_id, category_id, choice, choice_revision, choice_restored,
         deferred, review_revision, review_restored)
         SELECT record_id, category_id, choice,
           CASE WHEN choice_revision = 0 THEN 0 ELSE ?1 + choice_revision END, choice_revision > 0,
           deferred, CASE WHEN review_revision = 0 THEN 0 ELSE ?1 + review_revision END, review_revision > 0
         FROM recovery.archive_classification_decision WHERE 1
         ON CONFLICT(record_id) DO UPDATE SET
           category_id = CASE WHEN classification_decisions.choice_revision = 0 THEN excluded.category_id ELSE classification_decisions.category_id END,
           choice = CASE WHEN classification_decisions.choice_revision = 0 THEN excluded.choice ELSE classification_decisions.choice END,
           choice_restored = CASE WHEN classification_decisions.choice_revision = 0 THEN excluded.choice_restored ELSE classification_decisions.choice_restored END,
           choice_revision = CASE WHEN classification_decisions.choice_revision = 0 THEN excluded.choice_revision ELSE classification_decisions.choice_revision END,
           deferred = CASE WHEN classification_decisions.review_revision = 0 THEN excluded.deferred ELSE classification_decisions.deferred END,
           review_restored = CASE WHEN classification_decisions.review_revision = 0 THEN excluded.review_restored ELSE classification_decisions.review_restored END,
           review_revision = CASE WHEN classification_decisions.review_revision = 0 THEN excluded.review_revision ELSE classification_decisions.review_revision END",
        params![base],
    ).map_err(database_error)?;
    Ok(())
}

fn revision_exhausted() -> ScanError {
    error(
        "revision_exhausted",
        "Classification history cannot allocate a recovery revision",
    )
}
