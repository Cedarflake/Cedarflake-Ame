use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::{database_error, error};

pub(super) fn generations(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_restore_generations (
           source_generation INTEGER PRIMARY KEY NOT NULL
         );
         DELETE FROM temp.organization_restore_generations;
         INSERT INTO temp.organization_restore_generations
         SELECT source_generation FROM recovery.archive_binding
         UNION SELECT source_generation FROM recovery.archive_admission
         UNION SELECT source_generation FROM recovery.archive_version_evidence;",
        )
        .map_err(database_error)?;
    let (first, count): (i64, i64) = connection
        .query_row(
            "SELECT next_source_generation,
           (SELECT COUNT(*) FROM temp.organization_restore_generations AS source
            WHERE NOT EXISTS(SELECT 1 FROM organization_archive_generations AS known
              WHERE known.archive_id = ?1 AND known.source_generation = source.source_generation))
         FROM catalog_state",
            [archive_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(database_error)?;
    let next = first.checked_add(count).ok_or_else(|| {
        error(
            "identity_capacity",
            "The catalog cannot allocate more recovery identities",
        )
    })?;
    connection.execute(
        "INSERT INTO organization_archive_generations(archive_id, source_generation, local_generation)
         SELECT ?1, source_generation, ?2 + ROW_NUMBER() OVER (ORDER BY source_generation) - 1
         FROM temp.organization_restore_generations AS source
         WHERE NOT EXISTS(SELECT 1 FROM organization_archive_generations AS known
           WHERE known.archive_id = ?1 AND known.source_generation = source.source_generation)",
        params![archive_id, first],
    ).map_err(database_error)?;
    connection
        .execute(
            "UPDATE catalog_state SET next_source_generation = ?1",
            [next],
        )
        .map_err(database_error)?;
    Ok(())
}

pub(super) fn roots(connection: &Connection, archive_id: &str) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_restore_roots (
           original_root_id TEXT PRIMARY KEY NOT NULL,
           retained_root_id TEXT NOT NULL, current_root_id TEXT
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_restore_roots;",
        )
        .map_err(database_error)?;
    let mut after: Option<String> = None;
    loop {
        let roots = connection
            .prepare(
                "SELECT root_id FROM recovery.archive_root_evidence
             WHERE ?1 IS NULL OR root_id > ?1 ORDER BY root_id LIMIT 256",
            )
            .map_err(database_error)?
            .query_map([&after], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let Some(last) = roots.last() else { break };
        after = Some(last.clone());
        for root in roots {
            let retained = root_identity(archive_id, &root);
            connection
                .execute(
                    "INSERT OR IGNORE INTO organization_recovery_roots
                 SELECT ?1, ?2, root_id, path, identity_scheme, identity_value
                 FROM recovery.archive_root_evidence WHERE root_id = ?3",
                    params![retained, archive_id, root],
                )
                .map_err(database_error)?;
            connection
                .execute(
                    "INSERT INTO temp.organization_restore_roots
                 SELECT source.root_id, ?1,
                   (SELECT MIN(root.id) FROM library_root_publication_namespaces AS namespace
                    JOIN library_change_root_state AS state ON state.root_id = namespace.root_id
                      AND state.generation = namespace.root_generation AND state.is_active = 1
                    JOIN library_roots AS root ON root.id = state.root_id
                    WHERE namespace.identity_scheme = source.identity_scheme
                      AND namespace.identity_value = source.identity_value
                      AND source.identity_scheme = 'windows-file-id-128-v1'
                    HAVING COUNT(*) = 1)
                 FROM recovery.archive_root_evidence AS source WHERE source.root_id = ?2",
                    params![retained, root],
                )
                .map_err(database_error)?;
        }
    }
    Ok(())
}

fn root_identity(archive_id: &str, root_id: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ame-restored-root-v1\0");
    hasher.update(archive_id.as_bytes());
    hasher.update(root_id.as_bytes());
    format!("recovery-{}", hasher.finalize().to_hex())
}
