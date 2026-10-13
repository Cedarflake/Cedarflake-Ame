use std::collections::BTreeSet;

use rusqlite::{OptionalExtension, Transaction, params};

use super::super::{database_error, sqlite_integer};
use crate::domain::{PersistentJournalEnrollmentBatch, ScanError};

const RELOCATED: &str = "WITH relocated AS (
    SELECT (SELECT moves.current_path || substr(old.relative_path, length(moves.previous_path) + 1)
            FROM ame_journal_directory_moves AS moves
            WHERE old.relative_path = moves.previous_path
               OR substr(old.relative_path, 1, length(moves.previous_path) + 1) = moves.previous_path || '/'
            ORDER BY length(moves.previous_path) DESC LIMIT 1) AS relative_path,
           old.identity_scheme, old.identity_value
    FROM ame_journal_directory_projection AS old
)";

pub(crate) fn publish(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalEnrollmentBatch,
) -> Result<(), ScanError> {
    if batch.directory_changes.is_empty() {
        return Ok(());
    }
    let scan: Option<String> = transaction
        .query_row(
            "SELECT root.active_scan_id FROM library_roots AS root
         JOIN library_change_root_state AS active ON active.root_id = root.id
         WHERE root.id = ?1 AND active.generation = ?2 AND active.is_active = 1",
            params![
                batch.range.root_id,
                sqlite_integer(
                    batch.range.root_generation.value(),
                    "journal namespace generation"
                )?
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?
        .flatten();
    let scan = scan.ok_or_else(|| invalid("persistent_journal_namespace_stale"))?;
    transaction.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS ame_journal_directory_moves(
           previous_path TEXT PRIMARY KEY NOT NULL, current_path TEXT
         );
         CREATE TEMP TABLE IF NOT EXISTS ame_journal_directory_projection(
           relative_path TEXT PRIMARY KEY NOT NULL, identity_scheme TEXT NOT NULL, identity_value TEXT NOT NULL
         );
         DELETE FROM ame_journal_directory_moves;
         DELETE FROM ame_journal_directory_projection;"
    ).map_err(database_error)?;
    for change in &batch.directory_changes {
        let mut previous = BTreeSet::new();
        {
            let mut statement = transaction
                .prepare(
                    "SELECT relative_path FROM scan_directory_identities
                 WHERE scan_id = ?1 AND identity_value = ?2 ORDER BY relative_path LIMIT 65",
                )
                .map_err(database_error)?;
            for row in statement
                .query_map(params![scan, change.identity.value], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(database_error)?
            {
                previous.insert(row.map_err(database_error)?);
            }
        }
        if previous.len() > 64 {
            return Err(invalid("persistent_journal_namespace_capacity"));
        }
        for hinted in &change.previous_relative_paths {
            let owned = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM scan_directory_identities
                 WHERE scan_id = ?1 AND relative_path = ?2 AND identity_value = ?3)",
                    params![scan, hinted, change.identity.value],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if owned {
                previous.insert(hinted.clone());
            }
        }
        for previous in previous {
            transaction.execute(
                "INSERT INTO ame_journal_directory_moves(previous_path, current_path)
                 VALUES (?1, ?2) ON CONFLICT(previous_path) DO UPDATE SET current_path = excluded.current_path",
                params![previous, change.current_relative_path],
            ).map_err(database_error)?;
        }
    }
    let moves = {
        let mut statement = transaction
            .prepare("SELECT previous_path FROM ame_journal_directory_moves ORDER BY previous_path")
            .map_err(database_error)?;
        statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?
    };
    // Stage the immutable old projection first. Parent/child moves and path reuse cannot
    // consume a row produced by an earlier prefix rewrite in the same volume transaction.
    for previous in &moves {
        transaction
            .execute(
                "INSERT OR IGNORE INTO ame_journal_directory_projection
             SELECT relative_path, identity_scheme, identity_value FROM scan_directory_identities
             WHERE scan_id = ?1 AND relative_path = ?2",
                params![scan, previous],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO ame_journal_directory_projection
             SELECT relative_path, identity_scheme, identity_value FROM scan_directory_identities
             WHERE scan_id = ?1 AND relative_path >= ?2 AND relative_path < ?3",
                params![scan, format!("{previous}/"), format!("{previous}0")],
            )
            .map_err(database_error)?;
    }
    for previous in &moves {
        transaction
            .execute(
                "DELETE FROM scan_directory_identities WHERE scan_id = ?1
             AND (relative_path = ?2 OR (relative_path >= ?3 AND relative_path < ?4))",
                params![
                    scan,
                    previous,
                    format!("{previous}/"),
                    format!("{previous}0")
                ],
            )
            .map_err(database_error)?;
    }
    let collision = transaction.query_row(
        &format!("{RELOCATED}
         SELECT EXISTS(SELECT 1 FROM relocated AS target
           JOIN scan_directory_identities AS live
             ON live.scan_id = ?1 AND live.relative_path = target.relative_path
           WHERE target.relative_path IS NOT NULL AND live.identity_value != target.identity_value)"),
        [&scan], |row| row.get::<_, bool>(0),
    ).map_err(database_error)?;
    if collision {
        return Err(invalid("persistent_journal_namespace_collision"));
    }
    transaction.execute(
        &format!("{RELOCATED}
         INSERT INTO scan_directory_identities(scan_id, relative_path, identity_scheme, identity_value)
         SELECT ?1, relative_path, identity_scheme, identity_value FROM relocated WHERE relative_path IS NOT NULL
         ON CONFLICT(scan_id, relative_path) DO NOTHING"),
        [&scan],
    ).map_err(database_error)?;
    for change in &batch.directory_changes {
        if let Some(current) = &change.current_relative_path {
            let conflicting = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM scan_directory_identities
                 WHERE scan_id = ?1 AND relative_path = ?2 AND identity_value != ?3)",
                    params![scan, current, change.identity.value],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(database_error)?;
            if conflicting {
                return Err(invalid("persistent_journal_namespace_collision"));
            }
            transaction.execute(
                "INSERT INTO scan_directory_identities(scan_id, relative_path, identity_scheme, identity_value)
                 VALUES (?1, ?2, ?3, ?4) ON CONFLICT(scan_id, relative_path) DO NOTHING",
                params![scan, current, change.identity.scheme, change.identity.value],
            ).map_err(database_error)?;
        }
    }
    transaction.execute(
        "INSERT INTO scan_directory_journal_authority(scan_id, root_generation, journal_id, covered_until_usn)
         VALUES (?1, ?2, ?3, ?4) ON CONFLICT(scan_id) DO UPDATE SET
           root_generation = excluded.root_generation, journal_id = excluded.journal_id,
           covered_until_usn = excluded.covered_until_usn",
        params![scan, sqlite_integer(batch.range.root_generation.value(), "journal namespace generation")?,
            batch.range.journal_id.value().to_le_bytes().as_slice(), batch.range.covered_until_usn.value()],
    ).map_err(database_error)?;
    Ok(())
}

fn invalid(code: &str) -> ScanError {
    ScanError::new(
        code,
        "The journal directory publication lost its exact namespace ownership",
    )
}

#[cfg(test)]
mod tests;
