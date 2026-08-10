use std::path::Path;
use std::sync::OnceLock;

use crate::adapters::{LocalPreviewStore, SqliteCatalog, revalidate_file_state};
use crate::domain::{
    AssetLocationView, DiscoveredFile, ExpectedFileState, PreviewRequest, PreviewStatus, ScanError,
    ScanIssue,
};
use crate::ports::{CatalogRepository, PreviewStore};

use super::{StoragePaths, storage_paths};

static ACTIVE_PREVIEW_STORE: OnceLock<Result<LocalPreviewStore, ScanError>> = OnceLock::new();

pub fn materialize_preview(request: PreviewRequest) -> Result<AssetLocationView, ScanError> {
    validate_request(&request)?;
    let storage = storage_paths()?;
    let preview_store = active_preview_store(&storage)?;
    materialize_preview_with_store(request, storage, preview_store)
}

#[cfg(test)]
pub(crate) fn materialize_preview_with_storage(
    request: PreviewRequest,
    storage: StoragePaths,
) -> Result<AssetLocationView, ScanError> {
    let preview_store =
        LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
            .map_err(|issue| ScanError::new(issue.code, issue.message))?;
    materialize_preview_with_store(request, storage, &preview_store)
}

fn materialize_preview_with_store(
    request: PreviewRequest,
    storage: StoragePaths,
    preview_store: &LocalPreviewStore,
) -> Result<AssetLocationView, ScanError> {
    validate_request(&request)?;
    let mut catalog = SqliteCatalog::open(storage.catalog_path)?;
    let mut location = catalog
        .load_active_location(&request.location_id)?
        .ok_or_else(|| {
            ScanError::new(
                "preview_location_not_found",
                "The requested location is not active in the catalog",
            )
        })?;
    if matches!(location.preview_status, PreviewStatus::Ready)
        && !location.preview_path.is_empty()
        && Path::new(&location.preview_path).is_file()
        && !request.retry_failed
    {
        return Ok(location);
    }
    if matches!(location.preview_status, PreviewStatus::Failed) && !request.retry_failed {
        return Ok(location);
    }

    let expected = ExpectedFileState {
        absolute_path: location.absolute_path.clone(),
        file_size: location.file_size,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
    };
    if let Err(issue) = revalidate_file_state(&expected) {
        apply_failure(&mut location, &issue);
        catalog.update_active_preview(&location)?;
        return Ok(location);
    }
    let file = DiscoveredFile {
        absolute_path: location.absolute_path.clone(),
        relative_path: location.relative_path.clone(),
        file_size: location.file_size,
        created_unix_ms: location.created_unix_ms,
        modified_unix_ms: location.modified_unix_ms,
        file_identity: location.file_identity.clone(),
        issues: Vec::new(),
    };
    match preview_store.materialize(&file, request.preview_edge, location.width, location.height) {
        Ok(preview) => {
            location.preview_path = preview.path;
            location.width = preview.width;
            location.height = preview.height;
            location.preview_status = PreviewStatus::Ready;
            location.preview_issue_code = None;
            location.preview_issue_message = None;
        }
        Err(issue) => apply_failure(&mut location, &issue),
    }
    catalog.update_active_preview(&location)?;
    Ok(location)
}

pub(crate) fn active_preview_store(
    storage: &StoragePaths,
) -> Result<&'static LocalPreviewStore, ScanError> {
    ACTIVE_PREVIEW_STORE
        .get_or_init(|| {
            LocalPreviewStore::new(storage.preview_root.clone(), storage.preview_budget_bytes)
                .map_err(|issue| {
                    ScanError::new(
                        issue.code,
                        format!("Preview cache initialization failed: {}", issue.message),
                    )
                })
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn apply_failure(location: &mut AssetLocationView, issue: &ScanIssue) {
    location.preview_status = PreviewStatus::Failed;
    location.preview_issue_code = Some(issue.code.clone());
    location.preview_issue_message = Some(issue.message.clone());
}

fn validate_request(request: &PreviewRequest) -> Result<(), ScanError> {
    if request.location_id.trim().is_empty() {
        return Err(ScanError::new(
            "preview_location_id_empty",
            "The preview location identifier is required",
        ));
    }
    if !(96..=1024).contains(&request.preview_edge) {
        return Err(ScanError::new(
            "preview_edge_invalid",
            "Preview edge must be between 96 and 1024 pixels",
        ));
    }
    Ok(())
}
