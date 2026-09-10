use super::*;

use crate::adapters::{load_retained_scan_issue_window, retained_scan_has_unclassified_issues};

const TERMINAL_CODES: &[&str] = &[
    "image_dimensions_failed",
    "image_format_unsupported",
    "image_decode_invalid",
    "image_limits_exceeded",
    "image_decoder_rejected",
    "image_dimensions_exceeded",
    "media_type_unsupported",
];
const RETRYABLE_CODES: &[&str] = &[
    "image_open_failed",
    "image_header_read_failed",
    "source_changed_during_scan",
    "source_replaced_during_scan",
    "source_revalidation_failed",
    "source_became_unavailable",
    "source_identity_unavailable",
    "source_revision_unavailable",
    "source_revision_changed_during_scan",
];
const NONBLOCKING_CODES: &[&str] = &[
    "file_identity_unavailable",
    "orientation_read_failed",
    "metadata_read_failed",
    "metadata_size_exceeded",
    "metadata_parse_failed",
    "capture_time_invalid",
];

pub(super) fn restore(
    plan: &mut FinalizationPlan,
    catalog: &SqliteCatalog,
    scan_id: &str,
    canonical_root: &Path,
    requested_root: &Path,
) -> Result<bool, ScanError> {
    let codes: Vec<&str> = TERMINAL_CODES
        .iter()
        .copied()
        .chain(
            RETRYABLE_CODES
                .iter()
                .copied()
                .filter(|_| plan.mode == FinalizationMode::AuthoritativeRecovery),
        )
        .collect();
    let mut after_id = 0;
    let mut has_recoverable_issue = false;
    loop {
        let window = load_retained_scan_issue_window(
            catalog,
            scan_id,
            &codes,
            after_id,
            FINALIZATION_WINDOW,
        )?;
        if window.is_empty() {
            break;
        }
        for issue in window {
            let relative_path = issue
                .path
                .as_deref()
                .and_then(|path| {
                    authoritative_retry_relative_path(path, canonical_root, requested_root)
                })
                .ok_or_else(unverifiable_retained_rejections)?;
            let paths = match plan.mode {
                FinalizationMode::Foreground => &mut plan.foreground_revalidation.paths,
                FinalizationMode::AuthoritativeRecovery => &mut plan.authoritative_retry_paths,
            };
            if !retain_bounded_path(paths, &relative_path) {
                return Err(unverifiable_retained_rejections());
            }
            has_recoverable_issue = true;
            after_id = issue.id;
        }
    }
    if plan.mode == FinalizationMode::Foreground {
        return Ok(false);
    }
    let classified: Vec<&str> = codes
        .into_iter()
        .chain(NONBLOCKING_CODES.iter().copied())
        .collect();
    Ok(has_recoverable_issue
        && !retained_scan_has_unclassified_issues(catalog, scan_id, &classified)?)
}

fn authoritative_retry_relative_path(
    issue_path: &str,
    canonical_root: &Path,
    requested_root: &Path,
) -> Option<String> {
    let relative_path = Path::new(issue_path)
        .strip_prefix(requested_root)
        .or_else(|_| Path::new(issue_path).strip_prefix(canonical_root))
        .ok()?;
    if relative_path.as_os_str().is_empty() {
        return None;
    }
    Some(relative_path.to_string_lossy().replace('\\', "/"))
}

fn unverifiable_retained_rejections() -> ScanError {
    ScanError::new(
        "scan_rejected_input_evidence_unverifiable",
        "Retained media rejections cannot be bound to a bounded set of root-relative paths",
    )
}

#[cfg(all(test, windows))]
#[path = "retained_issue_recovery_tests.rs"]
mod tests;
