use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::database_error;

pub(super) fn prepare(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_restore_identifiers (
           kind TEXT NOT NULL, original_id TEXT NOT NULL, retained_id TEXT,
           PRIMARY KEY(kind, original_id)
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_restore_identifiers;
         INSERT INTO temp.organization_restore_identifiers(kind, original_id)
         SELECT 'asset', asset_id FROM recovery.archive_binding
         UNION SELECT 'asset', asset_id FROM recovery.archive_version_evidence
         UNION SELECT 'location', location_id FROM recovery.archive_version_evidence
         UNION SELECT 'location', location_id FROM recovery.archive_bookmark
         UNION SELECT 'location', preferred_location_id FROM recovery.archive_duplicate_decision WHERE preferred_location_id IS NOT NULL
         UNION SELECT 'asset', preferred_asset_id FROM recovery.archive_duplicate_decision WHERE preferred_asset_id IS NOT NULL;",
        )
        .map_err(database_error)?;
    let mut after = (String::new(), String::new());
    loop {
        let values = connection
            .prepare(
                "SELECT kind, original_id FROM temp.organization_restore_identifiers
             WHERE (kind, original_id) > (?1, ?2) ORDER BY kind, original_id LIMIT 256",
            )
            .map_err(database_error)?
            .query_map(params![after.0, after.1], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let Some(last) = values.last() else { break };
        after = last.clone();
        for (kind, original) in values {
            let mut hasher = blake3::Hasher::new();
            for field in ["ame-restored-identifier-v1", archive_id, &kind, &original] {
                hasher.update(&(field.len() as u64).to_le_bytes());
                hasher.update(field.as_bytes());
            }
            let retained = format!("recovery-{}", hasher.finalize().to_hex());
            connection
                .execute(
                    "UPDATE temp.organization_restore_identifiers SET retained_id = ?3
                 WHERE kind = ?1 AND original_id = ?2",
                    params![kind, original, retained],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}
