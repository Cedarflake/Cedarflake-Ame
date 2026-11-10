use std::fs::File;
use std::time::UNIX_EPOCH;

use crate::adapters::{LocalMediaInspector, PublicationGuardedFileDiscovery};
use crate::domain::file_operation_publication::FileOperationOutputMetadata;
use crate::domain::file_operations::{
    FileOperationIssue, FileOperationIssueKind as Kind, FileOperationSource,
};
use crate::domain::{
    CameraMetadata, DiscoveredFile, IncrementalCatalogRoot, MediaInspection, MetadataInspection,
};
use crate::ports::MediaInspector;
use crate::ports::file_operation_output::{FileOperationOutput, FileOperationOutputReader};
use crate::ports::media_inspection::GuardedMediaInspector;

use super::{WindowsFileOperations, scope};

pub struct OperationOutputGuard {
    _file: File,
    parent: PublicationGuardedFileDiscovery,
    _root: Option<PublicationGuardedFileDiscovery>,
    name: String,
    source: FileOperationSource,
    metadata: Option<FileOperationOutputMetadata>,
}

impl FileOperationOutput for OperationOutputGuard {
    fn metadata(&self) -> Option<&FileOperationOutputMetadata> {
        self.metadata.as_ref()
    }

    fn revalidate(&self) -> Result<(), FileOperationIssue> {
        self.parent
            .revalidate_relative_file_state(&self.name, &self.source.state)
            .map_err(|issue| FileOperationIssue::new(Kind::DestinationChanged, issue.message))
    }
}

impl FileOperationOutputReader for WindowsFileOperations {
    type Output = OperationOutputGuard;

    fn hold_output(
        &self,
        source: &FileOperationSource,
        root: Option<(&IncrementalCatalogRoot, &str)>,
    ) -> Result<Self::Output, FileOperationIssue> {
        let (parent, name) = scope::open_source(source)?;
        let file = parent
            .open_operation_source(&name, &source.state, false)
            .map_err(|error| {
                FileOperationIssue::new(Kind::DestinationChanged, error.to_string())
            })?;
        let root_guard = root
            .map(|(root, relative)| {
                let identity = root.publication_root_identity.as_ref().ok_or_else(|| {
                    FileOperationIssue::new(
                        Kind::DestinationChanged,
                        "The library has no retained directory identity",
                    )
                })?;
                let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
                    &root.root_path,
                    Some(identity),
                )
                .map_err(|error| {
                    FileOperationIssue::new(Kind::DestinationChanged, error.message)
                })?;
                guard
                    .revalidate_relative_file_state(relative, &source.state)
                    .map_err(|issue| {
                        FileOperationIssue::new(Kind::DestinationChanged, issue.message)
                    })?;
                Ok(guard)
            })
            .transpose()?;
        let metadata = if root.is_some() {
            let created_unix_ms = file
                .metadata()
                .ok()
                .and_then(|metadata| metadata.created().ok())
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| i64::try_from(duration.as_millis()).ok());
            let discovered = DiscoveredFile {
                source_root_path: source.parent.path.clone(),
                absolute_path: source.state.absolute_path.clone(),
                relative_path: name.clone(),
                file_size: source.state.file_size,
                created_unix_ms,
                modified_unix_ms: source.state.modified_unix_ms,
                file_identity: source.state.file_identity.clone(),
                source_revision: source.state.source_revision.clone(),
                source_generation: 0,
                issues: Vec::new(),
            };
            let inspector = LocalMediaInspector::new();
            let (inspection, issue) = match inspector.inspect_with_source(&parent, &discovered) {
                Ok(inspection) => (inspection, None),
                Err(failure) => (
                    MediaInspection {
                        width: 0,
                        height: 0,
                        metadata: MetadataInspection {
                            engine_id: inspector.metadata_engine_id().to_owned(),
                            engine_version: inspector.metadata_engine_version().to_owned(),
                            camera: CameraMetadata::default(),
                            capture_time: None,
                            issues: Vec::new(),
                        },
                    },
                    Some(failure.issue),
                ),
            };
            Some(FileOperationOutputMetadata {
                created_unix_ms,
                inspection,
                issue,
            })
        } else {
            None
        };
        let guard = OperationOutputGuard {
            _file: file,
            parent,
            _root: root_guard,
            name,
            source: source.clone(),
            metadata,
        };
        guard.revalidate()?;
        Ok(guard)
    }
}
