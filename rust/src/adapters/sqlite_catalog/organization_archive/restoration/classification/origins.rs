use rusqlite::Connection;

use crate::domain::ScanError;

use super::super::super::{database_error, error};

pub(super) fn apply(connection: &Connection, base: i64) -> Result<(), ScanError> {
    let conflicts: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM recovery.archive_classification_origin AS incoming
         JOIN classification_managed_categories AS current USING(category_id)
         WHERE current.concept_id <> incoming.concept_id
            OR current.purpose_category_id IS NOT incoming.purpose_category_id)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if conflicts {
        return Err(error(
            "classification_origin_conflict",
            "Managed category identity conflicts with retained origin",
        ));
    }
    connection.execute(
        "INSERT INTO classification_managed_categories(category_id, concept_id, purpose_category_id,
             operation_id, admitted_label, admitted_semantic_revision, protected)
         SELECT incoming.category_id, incoming.concept_id, incoming.purpose_category_id,
             incoming.operation_id, incoming.admitted_label, incoming.admitted_semantic_revision,
             incoming.protected OR (category.revision < ?1 AND NOT EXISTS(
                 SELECT 1 FROM classification_managed_categories AS known WHERE known.category_id = incoming.category_id))
         FROM recovery.archive_classification_origin AS incoming
         JOIN classification_categories AS category USING(category_id) WHERE 1
         ON CONFLICT(category_id) DO UPDATE SET protected =
             classification_managed_categories.protected OR excluded.protected
             OR classification_managed_categories.admitted_label <> excluded.admitted_label
             OR classification_managed_categories.admitted_semantic_revision <> excluded.admitted_semantic_revision",
        [base],
    ).map_err(database_error)?;
    Ok(())
}
