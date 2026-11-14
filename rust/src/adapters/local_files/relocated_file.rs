use std::io;

use windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT,
};

use crate::domain::{FileIdentityEvidence, MetadataInventoryPlaceholderState, ScanIssue};

use super::{
    PublicationGuardedFileDiscovery, file_attribute_tag_info_from_handle,
    file_identity_from_handle, final_path_from_handle, metadata_placeholder_state_from_attributes,
    path_metadata_issue, relative_path_text,
};

pub(super) fn locate_relative_file(
    source: &PublicationGuardedFileDiscovery,
    expected: &FileIdentityEvidence,
) -> Result<Option<String>, ScanIssue> {
    let discovery = &source.discovery;
    let issue = |error| path_metadata_issue(&discovery.canonical_root, error);
    let identity = super::identity_lookup::parse_identity(expected).map_err(|error| ScanIssue {
        path: None,
        code: error.code,
        message: error.message,
    })?;
    let Some((volume, file_id)) = identity else {
        return Ok(None);
    };
    if volume != discovery.root_proof.volume_serial {
        return Ok(None);
    }
    let file = match super::identity_lookup::open_by_id(
        &discovery.root_proof.handle,
        file_id,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL,
    ) {
        Ok(file) => file,
        // NTFS can return invalid-parameter for a complete identity whose file was deleted.
        Err(error)
            if error.kind() == io::ErrorKind::NotFound
                || error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(issue(error)),
    };
    if file_identity_from_handle(&file).map_err(issue)?.as_ref() != Some(expected) {
        return Ok(None);
    }
    let attributes = file_attribute_tag_info_from_handle(&file)
        .map_err(issue)?
        .FileAttributes;
    if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Ok(None);
    }
    if metadata_placeholder_state_from_attributes(attributes)
        != MetadataInventoryPlaceholderState::Available
    {
        return Err(ScanIssue {
            path: None,
            code: "cloud_placeholder_skipped".into(),
            message: "The relocated source is not available locally".into(),
        });
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Ok(None);
    }
    let path = final_path_from_handle(&file).map_err(issue)?;
    let Ok(relative) = path.strip_prefix(&discovery.canonical_root) else {
        return Ok(None);
    };
    Ok(Some(relative_path_text(relative)))
}

#[cfg(test)]
mod tests;
