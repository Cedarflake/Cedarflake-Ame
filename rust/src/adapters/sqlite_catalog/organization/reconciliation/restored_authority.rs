use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::super::database_error;
use super::super::restoration_schema::RestorationSchema;

pub(super) fn prepare(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_merge_authority (
           record_id TEXT NOT NULL, field TEXT NOT NULL, restored INTEGER NOT NULL,
           priority INTEGER NOT NULL, PRIMARY KEY(field, record_id)
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_merge_authority;",
        )
        .map_err(database_error)?;
    if matches!(
        RestorationSchema::read(connection)?,
        RestorationSchema::LocalDecisionsOnly
    ) {
        return connection
            .execute_batch(
                "INSERT INTO temp.organization_merge_authority
             SELECT selected.group_id, field.name, 0, 2
             FROM temp.organization_merge_groups AS selected
             CROSS JOIN (SELECT 'organization' AS name UNION ALL SELECT 'favorite') AS field;",
            )
            .map_err(database_error);
    }
    connection.execute_batch(
        "INSERT INTO temp.organization_merge_authority
         SELECT record.record_id, field.name, retained.applied_revision IS NOT NULL,
           CASE WHEN CASE field.name WHEN 'organization' THEN record.organization_revision
                      ELSE record.favorite_revision END = 0 THEN 0
                WHEN retained.applied_revision IS NOT NULL THEN 1 ELSE 2 END
         FROM temp.organization_merge_groups AS selected
         CROSS JOIN organization_records AS record ON record.record_id = selected.group_id
         CROSS JOIN (SELECT 'organization' AS name UNION ALL SELECT 'favorite') AS field
         LEFT JOIN organization_restored_fields AS retained
           ON retained.record_id = record.record_id AND retained.field = field.name
             AND retained.applied_revision = CASE field.name WHEN 'organization'
               THEN record.organization_revision ELSE record.favorite_revision END;
         WITH maximum AS MATERIALIZED (
           SELECT field, MAX(priority) AS priority FROM temp.organization_merge_authority GROUP BY field
         )
         DELETE FROM temp.organization_merge_authority AS candidate
         WHERE candidate.priority < (SELECT priority FROM maximum WHERE field = candidate.field);",
    ).map_err(database_error)
}

pub(super) fn retain(
    connection: &Connection,
    canonical: &str,
    organization: Option<i64>,
    favorite: Option<i64>,
) -> Result<(), ScanError> {
    if matches!(
        RestorationSchema::read(connection)?,
        RestorationSchema::LocalDecisionsOnly
    ) {
        return Ok(());
    }
    connection
        .execute(
            "DELETE FROM organization_restored_fields WHERE record_id = ?1",
            [canonical],
        )
        .map_err(database_error)?;
    for (field, revision) in [("organization", organization), ("favorite", favorite)] {
        if let Some(revision) = revision {
            connection
                .execute(
                    "INSERT INTO organization_restored_fields(record_id, field, applied_revision)
                 VALUES (?1, ?2, ?3)",
                    params![canonical, field, revision],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}
