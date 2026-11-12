use std::path::Path;

use crate::domain::file_operation_exports::FileOrganizationDestination;
use crate::domain::file_operation_journal::{AuthorizedFileOperation, FileOperationCatalogBinding};
use crate::domain::file_operation_selection::{FileOperationCandidate, FileOperationCommand};
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationDirectory, FileOperationRequest,
    FileOperationSource,
};
use crate::domain::{AssetLocationView, ScanError};
use crate::ports::file_operation_exports::NativeFilePreparation;

pub(super) fn inspect_source(
    location: &AssetLocationView,
    native: &dyn NativeFilePreparation,
) -> Result<FileOperationSource, ScanError> {
    let source = native
        .inspect_source(&location.absolute_path)
        .map_err(|error| ScanError::new("file_operation_source_unavailable", error.message))?;
    if source.state.file_identity != location.file_identity
        || source.state.source_revision != location.source_revision
        || source.state.file_size != location.file_size
        || source.state.modified_unix_ms != location.modified_unix_ms
    {
        return Err(ScanError::new(
            "file_operation_source_changed",
            "The selected file changed; refresh its selection before retrying",
        ));
    }
    Ok(source)
}

pub(super) fn authorize(
    candidate: &FileOperationCandidate,
    command: &FileOperationCommand,
    root: Option<&FileOperationDirectory>,
    native: &dyn NativeFilePreparation,
    batch_id: &str,
    authorized_unix_ms: i64,
) -> Result<AuthorizedFileOperation, ScanError> {
    let location = &candidate.location;
    let source = inspect_source(location, native)?;
    let destination = if matches!(
        command.action,
        FileAction::Copy | FileAction::Move | FileAction::Rename
    ) {
        let name = match &command.new_name {
            Some(name) => name.clone(),
            None => Path::new(&location.absolute_path)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    ScanError::new(
                        "file_operation_name_invalid",
                        "The selected file has no supported filename",
                    )
                })?
                .to_owned(),
        };
        let directory = match &candidate.organization_destination {
            FileOrganizationDestination::Folder(name) => native.organization_directory(
                root.ok_or_else(|| {
                    ScanError::new(
                        "file_operation_command_invalid",
                        "An organization rule needs an export root",
                    )
                })?,
                &[name],
            )?,
            FileOrganizationDestination::AlbumYear { album, year } => native
                .organization_directory(
                    root.ok_or_else(|| {
                        ScanError::new(
                            "file_operation_command_invalid",
                            "An organization rule needs an export root",
                        )
                    })?,
                    &[album, year],
                )?,
            FileOrganizationDestination::Direct => {
                root.cloned().unwrap_or_else(|| source.parent.clone())
            }
            FileOrganizationDestination::Skip(_) => {
                return Err(ScanError::new(
                    "file_operation_command_invalid",
                    "A skipped item cannot authorize a file operation",
                ));
            }
        };
        let destination = FileOperationDestination { directory, name };
        if command.action != FileAction::Copy
            && destination.directory.identity == source.parent.identity
            && Path::new(&source.state.absolute_path).file_name()
                == Some(std::ffi::OsStr::new(&destination.name))
        {
            return Err(ScanError::new(
                "file_operation_same_destination",
                "The source and destination are the same file",
            ));
        }
        native.destination_occupied(&destination).map_err(|error| {
            ScanError::new("file_operation_destination_unavailable", error.message)
        })?;
        Some(destination)
    } else {
        None
    };
    Ok(AuthorizedFileOperation {
        batch_id: batch_id.to_owned(),
        ordinal: candidate.ordinal,
        authorized_unix_ms,
        catalog: FileOperationCatalogBinding {
            asset_id: location.asset_id.clone(),
            location_id: location.location_id.clone(),
            source_generation: location.source_generation,
            root_id: location.root_id.clone(),
            root_generation: candidate.root_generation,
        },
        request: FileOperationRequest {
            action: command.action,
            source,
            destination,
        },
    })
}
