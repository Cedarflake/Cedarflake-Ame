use std::fs::{File, Metadata};
use std::path::Path;

use super::{
    CheckedDirectoryEntry, ReparseKind, checked_directory_entry_from_metadata, modified_unix_ms,
    path_containment_issue, validated_relative_path,
};
#[cfg(windows)]
use super::{
    FILE_READ_ATTRIBUTES, file_attribute_tag_info_from_handle, file_identity_from_handle,
    final_path_from_handle, open_root_relative_handle_with_options, open_validation_handle,
    path_metadata_issue, path_text, source_revision_from_handle,
    validate_present_file_revalidation_attributes, windows_ordinal_equals,
};
#[cfg(not(windows))]
use super::{file_identity, relative_path_text};
use crate::domain::{
    ExpectedFileState, FileIdentityEvidence, MetadataInventoryPlaceholderState, ScanIssue,
    SourceRevisionEvidence,
};

impl super::FileDiscovery {
    pub fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue> {
        let relative_path = validated_relative_path(relative_path)?;
        #[cfg(windows)]
        {
            let path = self.canonical_root.join(relative_path);
            let file = open_root_relative_handle_with_options(
                &self.root_proof.handle,
                relative_path,
                FILE_READ_ATTRIBUTES,
                Some(false),
                true,
                true,
            )
            .map_err(|error| path_metadata_issue(&path, error))?;
            let info = file_attribute_tag_info_from_handle(&file).map_err(|error| ScanIssue {
                path: Some(path_text(&path)),
                code: "source_revalidation_failed".to_owned(),
                message: error.to_string(),
            })?;
            validate_present_file_revalidation_attributes(&path, &info)?;
            let current_path =
                final_path_from_handle(&file).map_err(|error| path_metadata_issue(&path, error))?;
            let current_path = std::path::PathBuf::from(crate::domain::user_visible_path(
                &current_path.to_string_lossy(),
            ));
            let expected_path =
                std::path::PathBuf::from(crate::domain::user_visible_path(&expected.absolute_path));
            let same_parent = match (current_path.parent(), expected_path.parent()) {
                (Some(current), Some(expected)) => {
                    windows_ordinal_equals(current.as_os_str(), expected.as_os_str(), true)
                        .map_err(|error| path_metadata_issue(&path, error))?
                }
                _ => false,
            };
            if !same_parent || current_path.file_name() != expected_path.file_name() {
                return Err(ScanIssue {
                    path: Some(expected.absolute_path.clone()),
                    code: "source_changed_during_scan".to_owned(),
                    message: "The file name changed before catalog publication".to_owned(),
                });
            }
            let metadata = file
                .metadata()
                .map_err(|error| path_metadata_issue(&path, error))?;
            let actual_identity = file_identity_from_handle(&file).map_err(|error| ScanIssue {
                path: Some(path_text(&path)),
                code: "source_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
            let actual_revision =
                source_revision_from_handle(&file).map_err(|error| ScanIssue {
                    path: Some(path_text(&path)),
                    code: "source_revision_unavailable".to_owned(),
                    message: error.to_string(),
                })?;
            revalidate_file_state_values(
                expected,
                &metadata,
                actual_identity,
                Some(actual_revision),
            )
        }
        #[cfg(not(windows))]
        {
            let (path, metadata) = self.checked_existing_path(relative_path)?;
            let evidence = checked_directory_entry_from_metadata(
                relative_path_text(relative_path),
                &path,
                metadata,
            )?;
            if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
                return Err(path_containment_issue(&path));
            }
            revalidate_file_state_with_metadata(expected, &path, &evidence)
        }
    }
}

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
