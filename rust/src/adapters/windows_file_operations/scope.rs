use std::path::Path;

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::file_operations::{
    FileAction, FileOperationDestination, FileOperationDirectory, FileOperationIssue,
    FileOperationIssueKind as Kind, FileOperationRequest, FileOperationSource,
};
use crate::domain::{
    ExpectedFileState, MetadataInventoryEntryKind, MetadataInventoryPlaceholderState,
};

pub(super) struct OperationScope {
    pub source: PublicationGuardedFileDiscovery,
    pub source_name: String,
    pub destination: Option<PublicationGuardedFileDiscovery>,
}

pub(super) fn open_source(
    source: &FileOperationSource,
) -> Result<(PublicationGuardedFileDiscovery, String), FileOperationIssue> {
    let source_path = Path::new(&source.state.absolute_path);
    if source_path.parent() != Some(Path::new(&source.parent.path))
        || source.state.file_identity.is_none()
        || source.state.source_revision.is_none()
    {
        return Err(FileOperationIssue::new(
            Kind::InvalidRequest,
            "The request needs exact source and parent identities",
        ));
    }
    let name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| FileOperationIssue::new(Kind::InvalidRequest, "Invalid source filename"))?
        .to_owned();
    validate_name(&name)?;
    let guard = open_directory(&source.parent, Kind::SourceChanged)?;
    guard
        .revalidate_relative_file_state(&name, &source.state)
        .map_err(|issue| FileOperationIssue::new(Kind::SourceChanged, issue.message))?;
    Ok((guard, name))
}

impl OperationScope {
    pub fn open(request: &FileOperationRequest) -> Result<Self, FileOperationIssue> {
        let (source, source_name) = open_source(&request.source)?;
        let requires_destination = matches!(
            request.action,
            FileAction::Copy | FileAction::Move | FileAction::Rename
        );
        if requires_destination != request.destination.is_some() {
            return Err(FileOperationIssue::new(
                Kind::InvalidRequest,
                "The action and destination do not match",
            ));
        }
        let destination = request
            .destination
            .as_ref()
            .map(|destination| {
                validate_name(&destination.name)?;
                if request.action == FileAction::Rename
                    && destination.directory.identity != request.source.parent.identity
                {
                    return Err(FileOperationIssue::new(
                        Kind::InvalidRequest,
                        "Rename must retain the source directory",
                    ));
                }
                let guard = open_directory(&destination.directory, Kind::DestinationChanged)?;
                match guard.operation_path(&destination.name).symlink_metadata() {
                    Ok(_) => Err(FileOperationIssue::new(
                        Kind::Collision,
                        "The destination name is already occupied",
                    )),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(guard),
                    Err(error) => Err(FileOperationIssue::new(
                        Kind::Unavailable,
                        error.to_string(),
                    )),
                }
            })
            .transpose()?;
        Ok(Self {
            source,
            source_name,
            destination,
        })
    }

    pub fn revalidate(&self, request: &FileOperationRequest) -> Result<(), FileOperationIssue> {
        self.source
            .revalidate_relative_file_state(&self.source_name, &request.source.state)
            .map_err(|issue| FileOperationIssue::new(Kind::SourceChanged, issue.message))
    }
}

pub(super) fn inspect_directory(path: &str) -> Result<FileOperationDirectory, FileOperationIssue> {
    let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(path, None)
        .map_err(|error| FileOperationIssue::new(Kind::Unavailable, error.to_string()))?;
    let identity = guard
        .metadata_inventory_root_identity()
        .map_err(|error| FileOperationIssue::new(Kind::Unavailable, error.to_string()))?
        .ok_or_else(|| {
            FileOperationIssue::new(Kind::Unavailable, "Directory identity is unavailable")
        })?;
    Ok(FileOperationDirectory {
        path: path.to_owned(),
        identity,
    })
}

pub(super) fn inspect_source(path: &str) -> Result<FileOperationSource, FileOperationIssue> {
    let path = Path::new(path);
    let parent = path
        .parent()
        .and_then(|value| value.to_str())
        .ok_or_else(|| FileOperationIssue::new(Kind::InvalidRequest, "Missing source directory"))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| FileOperationIssue::new(Kind::InvalidRequest, "Missing source filename"))?;
    validate_name(name)?;
    let parent = inspect_directory(parent)?;
    let guard = open_directory(&parent, Kind::SourceChanged)?;
    inspect_guarded(&guard, &parent, name)
}

pub(super) fn inspect_guarded(
    guard: &PublicationGuardedFileDiscovery,
    parent: &FileOperationDirectory,
    name: &str,
) -> Result<FileOperationSource, FileOperationIssue> {
    let entry = guard
        .metadata_inventory_entry(name)
        .map_err(|issue| FileOperationIssue::new(Kind::Unavailable, issue.message))?;
    if entry.kind != MetadataInventoryEntryKind::File
        || entry.placeholder_state != MetadataInventoryPlaceholderState::Available
        || entry.file_identity.is_none()
        || entry.source_revision.is_none()
    {
        return Err(FileOperationIssue::new(
            Kind::Unavailable,
            "An operation requires a locally available file with identity evidence",
        ));
    }
    let file_size = entry
        .file_size
        .ok_or_else(|| FileOperationIssue::new(Kind::Unavailable, "File size is unavailable"))?;
    Ok(FileOperationSource {
        parent: parent.clone(),
        state: ExpectedFileState {
            absolute_path: Path::new(&parent.path)
                .join(name)
                .to_string_lossy()
                .into_owned(),
            file_size,
            modified_unix_ms: entry.modified_unix_ms,
            file_identity: entry.file_identity,
            source_revision: entry.source_revision,
        },
    })
}

pub(super) fn destination_occupied(
    destination: &FileOperationDestination,
) -> Result<bool, FileOperationIssue> {
    validate_name(&destination.name)?;
    let guard = open_directory(&destination.directory, Kind::DestinationChanged)?;
    occupied(&guard, &destination.name)
}

pub(super) fn inspect_location(
    location: &FileOperationDestination,
) -> Result<Option<FileOperationSource>, FileOperationIssue> {
    validate_name(&location.name)?;
    let guard = open_directory(&location.directory, Kind::DestinationChanged)?;
    if !occupied(&guard, &location.name)? {
        return Ok(None);
    }
    inspect_guarded(&guard, &location.directory, &location.name).map(Some)
}

fn occupied(
    guard: &PublicationGuardedFileDiscovery,
    name: &str,
) -> Result<bool, FileOperationIssue> {
    match guard.operation_path(name).symlink_metadata() {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(FileOperationIssue::new(
            Kind::Unavailable,
            error.to_string(),
        )),
    }
}

fn open_directory(
    directory: &FileOperationDirectory,
    kind: Kind,
) -> Result<PublicationGuardedFileDiscovery, FileOperationIssue> {
    PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &directory.path,
        Some(&directory.identity),
    )
    .map_err(|error| FileOperationIssue::new(kind, error.to_string()))
}

fn validate_name(name: &str) -> Result<(), FileOperationIssue> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|number| {
            matches!(
                number,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name.encode_utf16().count() > 255
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|value| value < ' ' || "<>:\"/\\|?*".contains(value))
        || device
    {
        return Err(FileOperationIssue::new(
            Kind::InvalidRequest,
            "The filename is not a supported Windows file name",
        ));
    }
    Ok(())
}
