use crate::domain::{MetadataInventoryCleanupReport, ScanError};
use crate::ports::{InventoryCleanupControl, MetadataInventoryRepository};

use super::{MAX_INVENTORY_PAGE_ENTRIES, is_catalog_contention};

const MAX_INVENTORY_CLEANUP_RUNS: u32 = 128;
const INVENTORY_TERMINAL_RETENTION_MILLIS: i64 = 7 * 24 * 60 * 60 * 1_000;

pub(in crate::application) fn cleanup_terminal_inventory_batch(
    repository: &mut impl MetadataInventoryRepository,
    observed_unix_ms: i64,
    control: InventoryCleanupControl,
) -> Result<MetadataInventoryCleanupReport, ScanError> {
    repository.cleanup_terminal_metadata_inventories(
        observed_unix_ms.saturating_sub(INVENTORY_TERMINAL_RETENTION_MILLIS),
        MAX_INVENTORY_PAGE_ENTRIES,
        MAX_INVENTORY_CLEANUP_RUNS,
        control,
    )
}

pub(super) fn cleanup_pending(
    repository: &mut impl MetadataInventoryRepository,
    observed_unix_ms: i64,
) -> Result<bool, ScanError> {
    pending_after_attempt(cleanup_terminal_inventory_batch(
        repository,
        observed_unix_ms,
        InventoryCleanupControl::default(),
    ))
}

fn pending_after_attempt(
    result: Result<MetadataInventoryCleanupReport, ScanError>,
) -> Result<bool, ScanError> {
    match result {
        Ok(report) => Ok(report.has_more),
        Err(error) if is_catalog_contention(&error.code) => Ok(true),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_cleanup_keeps_remaining_work_and_contention_pending() {
        for has_more in [false, true] {
            assert_eq!(
                pending_after_attempt(Ok(MetadataInventoryCleanupReport {
                    has_more,
                    ..MetadataInventoryCleanupReport::default()
                }))
                .expect("bounded cleanup result"),
                has_more,
            );
        }
        for code in ["catalog_database_busy", "catalog_database_locked"] {
            assert!(
                pending_after_attempt(Err(ScanError::new(code, "retry later")))
                    .expect("contention cannot fail successful inventory work")
            );
        }
        assert_eq!(
            pending_after_attempt(Err(ScanError::new("cleanup_corrupt", "invalid storage")))
                .expect_err("non-contention failures remain actionable")
                .code,
            "cleanup_corrupt",
        );
    }
}
