use crate::adapters::SqliteCatalog;
use crate::domain::ScanError;
use crate::ports::CatalogRepository;

pub(super) fn finish(
    mut catalog: SqliteCatalog,
    scan_id: &str,
    issue_count: u64,
    result: Result<(), ScanError>,
) -> Result<(), ScanError> {
    let result =
        result.map_err(
            |primary| match catalog.abandon_scan(scan_id, "failed", issue_count) {
                Ok(()) => primary,
                Err(cleanup) => scan_cleanup_failure(&primary, &cleanup),
            },
        );
    #[cfg(not(test))]
    let catalog_path = catalog.catalog_path().to_path_buf();
    drop(catalog);
    #[cfg(not(test))]
    crate::application::schedule_catalog_reclamation(catalog_path);
    result
}

pub(super) fn scan_cleanup_failure(primary: &ScanError, cleanup: &ScanError) -> ScanError {
    let primary_code = bounded_error_text(&primary.code, 256);
    let cleanup_code = bounded_error_text(&cleanup.code, 256);
    let primary_message = bounded_error_text(&primary.message, 1_600);
    let cleanup_message = bounded_error_text(&cleanup.message, 1_600);
    ScanError::new(
        "scan_failure_cleanup_failed",
        bounded_error_text(
            &format!(
                "Scan failed [{primary_code}]; cleanup failed [{cleanup_code}]; primary: \
                 {primary_message}; cleanup: {cleanup_message}",
            ),
            4_096,
        ),
    )
}

fn bounded_error_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
