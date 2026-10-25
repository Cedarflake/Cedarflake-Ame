use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::{capacity_error, database_error};

pub(super) fn restore(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_restore_admissions (
           original_id INTEGER PRIMARY KEY, source_generation INTEGER NOT NULL UNIQUE,
           restored_position INTEGER NOT NULL UNIQUE
         ); DELETE FROM temp.organization_restore_admissions;",
        )
        .map_err(database_error)?;
    connection.execute(
        "INSERT INTO temp.organization_restore_admissions
         SELECT source.admission_id, generation.local_generation, ROW_NUMBER() OVER (ORDER BY source.admission_id)
         FROM recovery.archive_admission AS source
         JOIN organization_archive_generations AS generation
           ON generation.archive_id = ?1 AND generation.source_generation = source.source_generation", [archive_id],
    ).map_err(database_error)?;
    let (count, missing): (i64, bool) = connection
        .query_row(
            "SELECT COUNT(*), EXISTS(SELECT 1 FROM temp.organization_restore_admissions AS source
           WHERE NOT EXISTS(SELECT 1 FROM organization_recovery_admissions AS known
             WHERE known.source_generation = source.source_generation))
         FROM temp.organization_restore_admissions",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(database_error)?;
    if !missing {
        return Ok(());
    }
    prepend(connection, count)?;
    connection.execute_batch(
        "UPDATE organization_review_bookmark AS bookmark SET admission_id = (
           SELECT source.restored_position FROM temp.organization_restore_admissions AS source
           JOIN organization_recovery_admissions AS known USING(source_generation)
           WHERE known.admission_id = bookmark.admission_id)
         WHERE EXISTS(SELECT 1 FROM temp.organization_restore_admissions AS source
           JOIN organization_recovery_admissions AS known USING(source_generation)
           WHERE known.admission_id = bookmark.admission_id);
         INSERT INTO organization_recovery_admissions(source_generation, admission_id)
         SELECT source_generation, restored_position FROM temp.organization_restore_admissions WHERE 1
         ON CONFLICT(source_generation) DO UPDATE SET admission_id = excluded.admission_id;
         UPDATE organization_review_cursor SET primary_number = (
           SELECT admission_id FROM organization_review_bookmark)
         WHERE EXISTS(SELECT 1 FROM organization_review_sort
           WHERE scope_slot = 'bookmark' AND sort_key = 'library_admission');",
    ).map_err(database_error)
}

fn prepend(connection: &Connection, count: i64) -> Result<(), ScanError> {
    let maximum: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(value), 0) FROM (
           SELECT seq AS value FROM sqlite_sequence WHERE name = 'organization_admissions'
           UNION ALL SELECT MAX(admission_id) FROM organization_admissions
           UNION ALL SELECT MAX(admission_id) FROM organization_recovery_admissions
           UNION ALL SELECT admission_id FROM organization_review_bookmark)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let shifted = maximum.checked_add(count).ok_or_else(capacity_error)?;
    let offset = shifted.checked_add(1).ok_or_else(capacity_error)?;
    maximum.checked_add(offset).ok_or_else(capacity_error)?;
    // Two disjoint positive ranges avoid row-order-dependent primary-key collisions. The enclosing
    // write transaction retains every source reference and rolls back both moves on failure.
    connection
        .execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(database_error)?;
    for delta in [offset, count - offset] {
        for table in [
            "organization_admissions",
            "organization_admission_sources",
            "organization_recovery_admissions",
        ] {
            connection
                .execute(
                    &format!("UPDATE {table} SET admission_id = admission_id + ?1"),
                    [delta],
                )
                .map_err(database_error)?;
        }
    }
    connection
        .execute(
            "UPDATE organization_review_bookmark SET admission_id = admission_id + ?1",
            [count],
        )
        .map_err(database_error)?;
    let changed = connection
        .execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'organization_admissions'",
            [shifted],
        )
        .map_err(database_error)?;
    if changed == 0 {
        connection
            .execute(
                "INSERT INTO sqlite_sequence(name, seq) VALUES ('organization_admissions', ?1)",
                params![shifted],
            )
            .map_err(database_error)?;
    }
    Ok(())
}
