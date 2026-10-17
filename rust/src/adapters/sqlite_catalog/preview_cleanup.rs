use crate::domain::ScanError;

use super::{SqliteCatalog, database_error};

pub(super) fn reset_all(catalog: &mut SqliteCatalog) -> Result<u64, ScanError> {
    catalog.flush_pending_locations()?;
    let transaction = catalog.begin_write()?;
    let updated = transaction
        .execute(
            "UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> '' OR preview_status <> 'pending'
               OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_catch_up_handoffs
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> '' OR preview_status <> 'pending'
               OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_scan_handoff_items
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> '' OR preview_status <> 'pending'
               OR preview_issue_code IS NOT NULL OR preview_issue_message IS NOT NULL",
            [],
        )
        .map_err(database_error)?;
    transaction
        .execute("DELETE FROM preview_artifacts", [])
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    u64::try_from(updated).map_err(|_| {
        ScanError::new(
            "preview_cleanup_count_invalid",
            "The preview cleanup update count exceeds the supported range",
        )
    })
}

pub(super) fn reset_outside_root(
    catalog: &mut SqliteCatalog,
    preview_root_prefix: &str,
) -> Result<u64, ScanError> {
    if preview_root_prefix.is_empty() {
        return Err(ScanError::new(
            "preview_root_prefix_empty",
            "The active preview root prefix is required",
        ));
    }
    catalog.flush_pending_locations()?;
    let transaction = catalog.begin_write()?;
    let updated = transaction
        .execute(
            "UPDATE asset_locations
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> ''
               AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
            [preview_root_prefix],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_catch_up_handoffs
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> ''
               AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
            [preview_root_prefix],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "UPDATE library_change_scan_handoff_items
             SET preview_path = '', preview_status = 'pending',
                 preview_issue_code = NULL, preview_issue_message = NULL
             WHERE preview_path <> ''
               AND lower(substr(preview_path, 1, length(?1))) <> lower(?1)",
            [preview_root_prefix],
        )
        .map_err(database_error)?;
    transaction
        .execute(
            "DELETE FROM preview_artifacts
             WHERE lower(substr(artifact_path, 1, length(?1))) <> lower(?1)",
            [preview_root_prefix],
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    u64::try_from(updated).map_err(|_| {
        ScanError::new(
            "preview_root_reset_count_invalid",
            "The preview-root reset count exceeds the supported range",
        )
    })
}
