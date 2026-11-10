use rusqlite::{Transaction, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicate_review::ExactDuplicateReviewKey;

use super::super::{database_error, exact_duplicate_review, sqlite_integer, sqlite_unsigned};
use super::{reads, writes};

pub(in super::super) struct ScanFileRemovals;

impl ScanFileRemovals {
    pub(in super::super) fn capture(
        transaction: &Transaction<'_>,
        root_id: &str,
    ) -> Result<Self, ScanError> {
        transaction
            .execute_batch(
                "CREATE TEMP TABLE file_operation_scan_removals (
                id INTEGER PRIMARY KEY, digest TEXT NOT NULL, byte_length INTEGER NOT NULL
            );",
            )
            .map_err(database_error)?;
        let mut pending = transaction
            .prepare(
                "SELECT id FROM file_operation_items INDEXED BY file_operation_removal_source
             WHERE phase IN ('executing', 'native_completed', 'uncertain')
               AND json_extract(authorization, '$.action') IN ('recycle', 'delete_permanently')
               AND json_extract(authorization, '$.catalog.root_id') = ?1",
            )
            .map_err(database_error)?;
        let rows = pending
            .query_map([root_id], |row| row.get::<_, i64>(0))
            .map_err(database_error)?;
        for id in rows {
            let id = id.map_err(database_error)?;
            let record = reads::load(transaction, sqlite_unsigned(id, "pending removal")?)?;
            if writes::catalog_matches(transaction, &record.authorization)?
                && let Some(key) =
                    exact_duplicate_review::file_operations::capture(transaction, &record)?
            {
                transaction
                    .execute(
                        "INSERT INTO temp.file_operation_scan_removals VALUES (?1, ?2, ?3)",
                        params![
                            id,
                            key.digest,
                            sqlite_integer(key.byte_length, "duplicate length")?
                        ],
                    )
                    .map_err(database_error)?;
            }
        }
        Ok(Self)
    }

    pub(in super::super) fn publish(self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        {
            let mut statement = transaction
                .prepare("SELECT id, digest, byte_length FROM temp.file_operation_scan_removals")
                .map_err(database_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(database_error)?;
            for row in rows {
                let (id, digest, length) = row.map_err(database_error)?;
                let record = reads::load(transaction, sqlite_unsigned(id, "scanned removal")?)?;
                if !writes::catalog_matches(transaction, &record.authorization)? {
                    exact_duplicate_review::file_operations::publish(
                        transaction,
                        &record,
                        Some(ExactDuplicateReviewKey {
                            digest,
                            byte_length: sqlite_unsigned(length, "duplicate length")?,
                        }),
                        None,
                    )?;
                }
            }
        }
        transaction
            .execute_batch("DROP TABLE temp.file_operation_scan_removals;")
            .map_err(database_error)
    }
}
