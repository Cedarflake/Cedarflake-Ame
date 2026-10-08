use std::fs::{File, Metadata};
use std::path::Path;

#[cfg(not(windows))]
use super::file_identity;
use super::{
    CheckedDirectoryEntry, ReparseKind, checked_directory_entry_from_metadata, modified_unix_ms,
    path_containment_issue,
};
#[cfg(windows)]
use super::{file_identity_from_handle, open_validation_handle, source_revision_from_handle};
use crate::domain::{
    ExpectedFileState, FileIdentityEvidence, MetadataInventoryPlaceholderState, ScanIssue,
    SourceRevisionEvidence,
};

pub fn revalidate_file_state(expected: &ExpectedFileState) -> Result<(), ScanIssue> {
    let path = Path::new(&expected.absolute_path);
    let metadata = path.symlink_metadata().map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "source_revalidation_failed".to_owned(),
        message: error.to_string(),
    })?;
    let evidence = checked_directory_entry_from_metadata(String::new(), path, metadata)?;
    if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
        return Err(path_containment_issue(path));
    }
    revalidate_file_state_with_metadata(expected, path, &evidence)
}

pub(crate) fn revalidate_open_preview_source(
    file: &File,
    expected: &ExpectedFileState,
) -> Result<Option<SourceRevisionEvidence>, ScanIssue> {
    let metadata = file.metadata().map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "source_revalidation_failed".to_owned(),
        message: error.to_string(),
    })?;
    #[cfg(windows)]
    let (actual_identity, actual_revision) = (
        file_identity_from_handle(file).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?,
        Some(
            source_revision_from_handle(file).map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "source_revision_unavailable".to_owned(),
                message: error.to_string(),
            })?,
        ),
    );
    #[cfg(not(windows))]
    let (actual_identity, actual_revision) = (None, None);
    revalidate_file_state_values(
        expected,
        &metadata,
        actual_identity,
        actual_revision.clone(),
    )?;
    Ok(actual_revision)
}

pub(super) fn revalidate_file_state_with_metadata(
    expected: &ExpectedFileState,
    path: &Path,
    evidence: &CheckedDirectoryEntry,
) -> Result<(), ScanIssue> {
    if evidence.placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_became_unavailable".to_owned(),
            message: "The file is no longer locally available".to_owned(),
        });
    }
    #[cfg(windows)]
    {
        let identity_error_code = if expected.file_identity.is_some() {
            "source_identity_unavailable"
        } else {
            "source_revision_unavailable"
        };
        let file = open_validation_handle(path).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: identity_error_code.to_owned(),
            message: error.to_string(),
        })?;
        let actual_identity = file_identity_from_handle(&file).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: identity_error_code.to_owned(),
            message: error.to_string(),
        })?;
        let actual_revision = source_revision_from_handle(&file).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_revision_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let metadata = file.metadata().map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_revalidation_failed".to_owned(),
            message: error.to_string(),
        })?;
        revalidate_file_state_values(expected, &metadata, actual_identity, Some(actual_revision))
    }
    #[cfg(not(windows))]
    {
        let actual_identity = if expected.file_identity.is_some() {
            file_identity(path).map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "source_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?
        } else {
            None
        };
        revalidate_file_state_values(expected, &evidence.metadata, actual_identity, None)
    }
}

pub(super) fn revalidate_file_state_values(
    expected: &ExpectedFileState,
    metadata: &Metadata,
    actual_identity: Option<FileIdentityEvidence>,
    actual_revision: Option<SourceRevisionEvidence>,
) -> Result<(), ScanIssue> {
    if metadata.len() != expected.file_size
        || modified_unix_ms(metadata) != expected.modified_unix_ms
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_changed_during_scan".to_owned(),
            message: "The file size or modification time changed during the scan".to_owned(),
        });
    }
    if let Some(expected_identity) = &expected.file_identity
        && actual_identity.as_ref() != Some(expected_identity)
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_replaced_during_scan".to_owned(),
            message: "The file identity changed during the scan".to_owned(),
        });
    }
    if let Some(expected_revision) = &expected.source_revision
        && actual_revision.as_ref() != Some(expected_revision)
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_revision_changed_during_scan".to_owned(),
            message: "The filesystem source revision changed during the scan".to_owned(),
        });
    }
    Ok(())
}

#[cfg(all(windows, test))]
mod tests;
