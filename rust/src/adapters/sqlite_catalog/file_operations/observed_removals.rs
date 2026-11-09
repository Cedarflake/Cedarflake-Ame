use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicate_review::ExactDuplicateReviewKey;
use crate::domain::file_operation_journal::FileOperationRecord;

use super::super::{database_error, exact_duplicate_review, sqlite_unsigned};
use super::{reads, writes};

pub(in super::super) struct ObservedFileRemoval {
    record: FileOperationRecord,
    preference: Option<ExactDuplicateReviewKey>,
}

impl ObservedFileRemoval {
    pub(in super::super) fn capture(
        transaction: &Transaction<'_>,
        active_scan: &str,
        location_id: &str,
    ) -> Result<Option<Self>, ScanError> {
        let id: Option<i64> = transaction.query_row(
            "SELECT id FROM file_operation_items
             WHERE json_extract(authorization, '$.catalog.location_id') = ?1
               AND json_extract(authorization, '$.catalog.source_generation') = (
                   SELECT source_generation FROM asset_locations WHERE scan_id = ?2 AND location_id = ?1)
               AND phase IN ('executing', 'native_completed', 'uncertain')
               AND json_extract(authorization, '$.action') IN ('recycle', 'delete_permanently')
             ORDER BY id DESC LIMIT 1",
            params![location_id, active_scan], |row| row.get(0),
        ).optional().map_err(database_error)?;
        let Some(id) = id else {
            return Ok(None);
        };
        let record = reads::load(transaction, sqlite_unsigned(id, "observed operation")?)?;
        if !writes::catalog_matches(transaction, &record.authorization)? {
            return Ok(None);
        }
        let preference = exact_duplicate_review::file_operations::capture(transaction, &record)?;
        Ok(Some(Self { record, preference }))
    }

    pub(in super::super) fn publish(self, transaction: &Transaction<'_>) -> Result<(), ScanError> {
        if writes::catalog_matches(transaction, &self.record.authorization)? {
            return Ok(());
        }
        exact_duplicate_review::file_operations::publish(
            transaction,
            &self.record,
            self.preference,
            None,
        )
    }
}
