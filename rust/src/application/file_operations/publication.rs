use std::path::Path;

use crate::domain::file_operation_journal::{FileOperationPhase, FileOperationRecord};
use crate::domain::file_operation_publication::{
    FileOperationDestinationPublication, FileOperationOutputMetadata, relative_output_path,
};
use crate::domain::file_operations::{FileAction, FileOperationSource};
use crate::domain::{
    AssetLocationView, IncrementalCatalogRoot, PreviewStatus, ScanError, stable_id,
    user_visible_path,
};
use crate::ports::file_operation_catalog::FileOperationCatalog;
use crate::ports::file_operation_output::{FileOperationOutput, FileOperationOutputReader};

pub fn publish_file_operation(
    catalog: &mut impl FileOperationCatalog,
    reader: &impl FileOperationOutputReader,
    id: u64,
    now_unix_ms: i64,
) -> Result<FileOperationRecord, ScanError> {
    let record = catalog.load(id)?;
    if record.checkpoint.phase == FileOperationPhase::Published {
        return Ok(record);
    }
    if record.checkpoint.phase != FileOperationPhase::NativeCompleted {
        return Err(ScanError::new(
            "file_operation_not_completed",
            "Catalog publication requires a completed native operation",
        ));
    }
    let receipt =
        record.checkpoint.receipt.as_ref().ok_or_else(|| {
            ScanError::new("file_operation_missing_receipt", "Missing native result")
        })?;
    let Some(output) = &receipt.destination else {
        return catalog.publish_operation(id, record.checkpoint.version, None, now_unix_ms);
    };
    let roots = catalog.publication_roots()?;
    let target = roots
        .iter()
        .filter(|root| root.active_scan_id.is_some())
        .filter_map(|root| {
            relative_output_path(&root.root_path, &output.state.absolute_path)
                .map(|relative| (root, relative))
        })
        .max_by_key(|(root, _)| Path::new(&root.root_path).components().count());
    let guard = reader
        .hold_output(
            output,
            target
                .as_ref()
                .map(|(root, relative)| (*root, relative.as_str())),
        )
        .map_err(|issue| ScanError::new("file_operation_output_changed", issue.message))?;
    let destination = target
        .map(|(root, relative)| {
            let metadata = guard.metadata().ok_or_else(|| {
                ScanError::new(
                    "file_operation_metadata_missing",
                    "Missing destination metadata",
                )
            })?;
            Ok(FileOperationDestinationPublication {
                root: root.clone(),
                location: location(&record, output, root, relative, metadata),
            })
        })
        .transpose()?;
    guard
        .revalidate()
        .map_err(|issue| ScanError::new("file_operation_output_changed", issue.message))?;
    catalog.publish_operation(
        id,
        record.checkpoint.version,
        destination.as_ref(),
        now_unix_ms,
    )
}

fn location(
    record: &FileOperationRecord,
    source: &FileOperationSource,
    root: &IncrementalCatalogRoot,
    relative_path: String,
    metadata: &FileOperationOutputMetadata,
) -> AssetLocationView {
    let asset_id = if record.authorization.request.action == FileAction::Copy {
        stable_id("file-operation-copy-v1", &record.id.to_string())
    } else {
        record.authorization.catalog.asset_id.clone()
    };
    AssetLocationView {
        asset_id,
        location_id: super::super::scan_library::stable_location_id(&root.root_id, &relative_path),
        root_id: root.root_id.clone(),
        scan_id: root.active_scan_id.clone().unwrap_or_default(),
        absolute_path: source.state.absolute_path.clone(),
        display_path: user_visible_path(&source.state.absolute_path),
        relative_path,
        preview_path: String::new(),
        file_size: source.state.file_size,
        created_unix_ms: metadata.created_unix_ms,
        modified_unix_ms: source.state.modified_unix_ms,
        file_identity: source.state.file_identity.clone(),
        source_revision: source.state.source_revision.clone(),
        source_generation: 0,
        width: metadata.inspection.width,
        height: metadata.inspection.height,
        preview_status: if metadata.issue.is_some() {
            PreviewStatus::Failed
        } else {
            PreviewStatus::Pending
        },
        preview_issue_code: metadata.issue.as_ref().map(|issue| issue.code.clone()),
        preview_issue_message: metadata.issue.as_ref().map(|issue| issue.message.clone()),
        metadata_engine_id: metadata.inspection.metadata.engine_id.clone(),
        metadata_engine_version: metadata.inspection.metadata.engine_version.clone(),
        camera: metadata.inspection.metadata.camera.clone(),
        capture_time: metadata.inspection.metadata.capture_time.clone(),
    }
}
