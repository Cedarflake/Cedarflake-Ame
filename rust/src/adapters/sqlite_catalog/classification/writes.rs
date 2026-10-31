use rusqlite::{Connection, params};

use crate::application::classification_decisions::plan_classification_change;
use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationAction, ClassificationChange, ClassificationRecordId,
};
use crate::domain::classification_catalog::{
    ClassificationWriteReceipt, PreparedClassificationSelection,
};

use super::super::{database_error, organization::records};
use super::{changed, codec, configuration, observations, operations, request};

pub(super) fn apply(
    connection: &Connection,
    id: &ClassificationRecordId,
    prepared: &PreparedClassificationSelection,
    action: &ClassificationAction,
) -> Result<ClassificationWriteReceipt, ScanError> {
    let digest = request::digest(prepared, action)?;
    if let Some(receipt) = operations::replay(connection, id, &digest)? {
        return Ok(receipt);
    }
    let current = observations::prepare(connection, &prepared.selections)?;
    if current.groups.len() != prepared.groups.len() {
        return Err(changed());
    }
    let taxonomy = configuration::load(connection)?
        .taxonomy
        .ok_or_else(changed)?;
    let plans = prepared
        .groups
        .iter()
        .zip(&current.groups)
        .map(|(observed, current)| {
            plan_classification_change(observed, current, action, &taxonomy)
                .map_err(|error| ScanError::new("classification_changed", error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let kind = if matches!(action, ClassificationAction::Defer) {
        "defer"
    } else {
        "category"
    };
    let revision = operations::begin(connection, id, &digest, kind)?;
    let mut changed_groups = 0;
    for (group, change) in current.groups.iter().zip(plans) {
        if change == ClassificationChange::Unchanged {
            continue;
        }
        if records::materialize(connection, &observations::key(&group.target.subject))? {
            return Err(changed());
        }
        connection.execute(&format!(
            "INSERT OR IGNORE INTO classification_decisions(record_id, choice_revision, choice_restored,
             deferred, review_revision, review_restored) SELECT record_id, 0, 0, 0, 0, 0 FROM ({})",
            records::CURRENT_RECORDS,
        ), []).map_err(database_error)?;
        match change {
            ClassificationChange::Choose(choice) => {
                remember(connection, id, "category", revision)?;
                remember(connection, id, "review", revision)?;
                connection.execute(&format!(
                    "UPDATE classification_decisions SET category_id = ?1, choice = ?2, choice_revision = ?3,
                     choice_restored = 0, deferred = 0, review_revision = ?3, review_restored = 0
                     WHERE record_id IN ({})", records::CURRENT_RECORDS,
                ), params![choice.category().as_str(), codec::encode_choice(&choice)?, revision]).map_err(database_error)?;
            }
            ClassificationChange::Defer => {
                remember(connection, id, "review", revision)?;
                connection.execute(&format!(
                    "UPDATE classification_decisions SET deferred = 1, review_revision = ?1, review_restored = 0
                     WHERE record_id IN ({})", records::CURRENT_RECORDS,
                ), [revision]).map_err(database_error)?;
            }
            ClassificationChange::Unchanged => unreachable!(),
        }
        changed_groups += 1;
    }
    operations::finish(connection, id, revision, changed_groups, false)
}

fn remember(
    connection: &Connection,
    id: &ClassificationRecordId,
    field: &str,
    revision: i64,
) -> Result<(), ScanError> {
    connection.execute(&format!(
        "INSERT INTO classification_operation_changes(operation_id, record_id, field, previous_payload,
         previous_revision, previous_restored, applied_revision)
         SELECT ?1, record_id, ?2,
           CASE WHEN ?2 = 'category' THEN choice WHEN deferred THEN 'true' ELSE 'false' END,
           CASE WHEN ?2 = 'category' THEN choice_revision ELSE review_revision END,
           CASE WHEN ?2 = 'category' THEN choice_restored ELSE review_restored END, ?3
         FROM classification_decisions WHERE record_id IN ({})", records::CURRENT_RECORDS,
    ), params![id.as_str(), field, revision]).map_err(database_error)?;
    Ok(())
}
