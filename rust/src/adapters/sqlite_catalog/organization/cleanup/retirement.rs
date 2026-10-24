use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization_cleanup::{OrganizationCleanupPhase, OrganizationCleanupProgress};

use super::super::super::database_error;

pub(super) fn apply(
    connection: &Connection,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_generations (
           source_generation INTEGER PRIMARY KEY NOT NULL
         );
         DELETE FROM temp.organization_cleanup_generations;
         INSERT OR IGNORE INTO temp.organization_cleanup_generations
         SELECT binding.source_generation FROM temp.organization_cleanup_groups AS selected
         CROSS JOIN organization_record_groups AS member USING(group_id)
         CROSS JOIN organization_bindings AS binding ON binding.record_id = member.record_id;
         CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_window (
           group_id TEXT PRIMARY KEY NOT NULL
         ) WITHOUT ROWID;",
        )
        .map_err(database_error)?;
    let mut removed = 0;
    loop {
        super::report(progress, OrganizationCleanupPhase::Applying, removed)?;
        connection
            .execute_batch("DELETE FROM temp.organization_cleanup_window;")
            .map_err(database_error)?;
        let selected = connection
            .execute(
                "INSERT INTO temp.organization_cleanup_window
             SELECT group_id FROM temp.organization_cleanup_groups ORDER BY group_id LIMIT 256",
                [],
            )
            .map_err(database_error)?;
        if selected == 0 {
            break;
        }
        connection
            .execute_batch(
                "DELETE FROM organization_review_scopes WHERE slot = 'bookmark' AND EXISTS(
               SELECT 1 FROM organization_review_bookmark AS bookmark
               JOIN organization_record_groups AS member ON member.record_id = bookmark.record_id
               JOIN temp.organization_cleanup_window AS selected USING(group_id));
             DELETE FROM organization_records WHERE record_id IN (
               SELECT member.record_id FROM temp.organization_cleanup_window AS selected
               CROSS JOIN organization_record_groups AS member USING(group_id));
             DELETE FROM temp.organization_cleanup_groups
               WHERE group_id IN (SELECT group_id FROM temp.organization_cleanup_window);",
            )
            .map_err(database_error)?;
        removed += selected as u64;
    }
    loop {
        super::report(progress, OrganizationCleanupPhase::Applying, removed)?;
        let selected = connection
            .execute(
                "DELETE FROM organization_recovery_duplicate_decisions
             WHERE (scheme, digest, byte_length) IN (
               SELECT scheme, digest, byte_length FROM temp.organization_cleanup_duplicates
               ORDER BY scheme, digest, byte_length LIMIT 256)",
                [],
            )
            .map_err(database_error)?;
        if selected == 0 {
            break;
        }
        connection
            .execute(
                "DELETE FROM temp.organization_cleanup_duplicates
             WHERE (scheme, digest, byte_length) IN (
               SELECT scheme, digest, byte_length FROM temp.organization_cleanup_duplicates
               ORDER BY scheme, digest, byte_length LIMIT 256)",
                [],
            )
            .map_err(database_error)?;
        removed += selected as u64;
    }
    retire_unused_generation_authority(connection, progress, removed)
}

fn retire_unused_generation_authority(
    connection: &Connection,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
    removed: u64,
) -> Result<(), ScanError> {
    connection.execute_batch(
        "DELETE FROM temp.organization_cleanup_generations
         WHERE source_generation IN (
           SELECT binding.source_generation FROM organization_bindings AS binding
           CROSS JOIN temp.organization_cleanup_generations AS selected USING(source_generation));
         DELETE FROM temp.organization_cleanup_generations AS selected
         WHERE EXISTS(SELECT 1 FROM asset_locations AS location
             JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             WHERE location.source_generation = selected.source_generation)
           OR EXISTS(SELECT 1 FROM organization_recovery_versions AS evidence
             JOIN organization_recovery_duplicate_decisions AS choice
               ON choice.scheme = evidence.fingerprint_scheme AND choice.digest = evidence.digest
                 AND choice.byte_length = evidence.byte_length
             WHERE evidence.source_generation = selected.source_generation);
         CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_generation_window (
           source_generation INTEGER PRIMARY KEY NOT NULL
         );",
    ).map_err(database_error)?;
    loop {
        super::report(progress, OrganizationCleanupPhase::Applying, removed)?;
        connection
            .execute_batch("DELETE FROM temp.organization_cleanup_generation_window;")
            .map_err(database_error)?;
        let selected = connection
            .execute(
                "INSERT INTO temp.organization_cleanup_generation_window
             SELECT source_generation FROM temp.organization_cleanup_generations
             ORDER BY source_generation LIMIT 256",
                [],
            )
            .map_err(database_error)?;
        if selected == 0 {
            break;
        }
        connection
            .execute_batch(
                "DELETE FROM organization_recovery_admissions WHERE source_generation IN (
               SELECT source_generation FROM temp.organization_cleanup_generation_window);
             DELETE FROM organization_admissions WHERE source_generation IN (
               SELECT source_generation FROM temp.organization_cleanup_generation_window);
             DELETE FROM organization_recovery_versions WHERE source_generation IN (
               SELECT source_generation FROM temp.organization_cleanup_generation_window);
             DELETE FROM temp.organization_cleanup_generations WHERE source_generation IN (
               SELECT source_generation FROM temp.organization_cleanup_generation_window);",
            )
            .map_err(database_error)?;
    }
    Ok(())
}
