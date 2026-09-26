use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::adapters::PreviewPublicationGuard;
#[cfg(windows)]
use crate::adapters::open_viewer_source_guard;
use crate::domain::{AssetLocationView, IncrementalCatalogRoot, ScanError, SourceRevisionEvidence};

const MAX_ENCODED_SOURCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SOURCE_PATH_UNITS: usize = 32_767;
const MAX_SOURCE_PATH_COMPONENTS: usize = 256;
const MAX_SOURCE_LEASES: usize = 2;
static ACTIVE_SOURCE_LEASES: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
pub struct ViewerSourceRequest {
    pub location_id: String,
    pub expected_root_id: String,
    pub expected_scan_id: String,
    pub expected_source_revision: Option<SourceRevisionEvidence>,
    pub expected_source_generation: u64,
}

pub struct ViewerSourceLease {
    path: String,
    _guard: PreviewPublicationGuard,
    _admission: SourceLeaseAdmission,
}

impl ViewerSourceLease {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

pub(crate) fn acquire_viewer_source(
    request: ViewerSourceRequest,
) -> Result<ViewerSourceLease, ScanError> {
    validate_request(&request)?;
    let admission = SourceLeaseAdmission::acquire(&ACTIVE_SOURCE_LEASES)?;
    let storage = super::storage_paths()?;
    acquire_with_catalog(&request, &storage.catalog_path, admission)
}

fn acquire_with_catalog(
    request: &ViewerSourceRequest,
    catalog_path: &Path,
    admission: SourceLeaseAdmission,
) -> Result<ViewerSourceLease, ScanError> {
    let reader = super::catalog_session::open_catalog_reader(catalog_path)?;
    let (location, root) = reader.load_preview_context(&request.location_id)?;
    let (location, root) = validate_context(request, location, root)?;
    let (path, guard) = acquire_guard(&location, &root)?;
    #[cfg(test)]
    tests::after_source_guard();
    let (current_location, current_root) = reader.load_preview_context(&request.location_id)?;
    let (current_location, current_root) =
        validate_context(request, current_location, current_root)?;
    if current_location.absolute_path != location.absolute_path
        || current_location.file_identity != location.file_identity
        || current_location.file_size != location.file_size
        || current_location.relative_path != location.relative_path
        || current_location.modified_unix_ms != location.modified_unix_ms
        || current_root.root_generation != root.root_generation
        || current_root.publication_root_identity != root.publication_root_identity
        || current_root.root_path != root.root_path
    {
        return Err(superseded());
    }
    Ok(ViewerSourceLease {
        path,
        _guard: guard,
        _admission: admission,
    })
}

fn validate_context(
    request: &ViewerSourceRequest,
    location: Option<AssetLocationView>,
    root: Option<IncrementalCatalogRoot>,
) -> Result<(AssetLocationView, IncrementalCatalogRoot), ScanError> {
    let (Some(location), Some(root)) = (location, root) else {
        return Err(superseded());
    };
    if location.location_id != request.location_id
        || root.root_id != request.expected_root_id
        || root.active_scan_id.as_deref() != Some(request.expected_scan_id.as_str())
        || location.root_id != request.expected_root_id
        || location.scan_id != request.expected_scan_id
        || location.source_revision != request.expected_source_revision
        || location.source_generation != request.expected_source_generation
    {
        return Err(superseded());
    }
    if location.file_size == 0 || location.file_size > MAX_ENCODED_SOURCE_BYTES {
        return Err(ScanError::new(
            "viewer_source_size_exceeded",
            "Original-image viewing requires a nonempty encoded source no larger than 256 MiB",
        ));
    }
    if location.absolute_path.encode_utf16().count() > MAX_SOURCE_PATH_UNITS
        || Path::new(&location.absolute_path).components().count() > MAX_SOURCE_PATH_COMPONENTS
    {
        return Err(ScanError::new(
            "viewer_source_path_exceeded",
            "The source path exceeds the Windows path limit",
        ));
    }
    Ok((location, root))
}

fn acquire_guard(
    location: &AssetLocationView,
    root: &IncrementalCatalogRoot,
) -> Result<(String, PreviewPublicationGuard), ScanError> {
    #[cfg(not(windows))]
    {
        let _ = (location, root);
        Err(ScanError::new(
            "viewer_source_platform_unsupported",
            "Guarded original-image viewing requires Windows 11 x64",
        ))
    }
    #[cfg(windows)]
    {
        let identity = root.publication_root_identity.as_ref().ok_or_else(|| {
            ScanError::new(
                "viewer_source_identity_unproven",
                "The source root has no durable publication identity",
            )
        })?;
        let expected = crate::domain::ExpectedFileState {
            absolute_path: location.absolute_path.clone(),
            file_size: location.file_size,
            modified_unix_ms: location.modified_unix_ms,
            file_identity: location.file_identity.clone(),
            source_revision: location.source_revision.clone(),
        };
        open_viewer_source_guard(
            &expected,
            Path::new(&root.root_path),
            Path::new(&location.relative_path),
            identity,
        )
        .map_err(source_error)
    }
}

#[cfg(windows)]
fn source_error(issue: crate::domain::ScanIssue) -> ScanError {
    ScanError::new(
        "viewer_source_unavailable",
        format!("The original cannot be read safely ({})", issue.code),
    )
}

fn validate_request(request: &ViewerSourceRequest) -> Result<(), ScanError> {
    if [
        &request.location_id,
        &request.expected_root_id,
        &request.expected_scan_id,
    ]
    .iter()
    .any(|value| value.trim().is_empty() || value.len() > 512)
        || request.expected_source_generation == 0
    {
        return Err(ScanError::new(
            "viewer_source_request_invalid",
            "Original-image viewing requires a bounded current catalog source lease",
        ));
    }
    if request
        .expected_source_revision
        .as_ref()
        .is_some_and(|revision| {
            revision.scheme != "windows-file-change-time-100ns-v1"
                || revision.value.len() != 16
                || !revision
                    .value
                    .bytes()
                    .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
        })
    {
        return Err(ScanError::new(
            "viewer_source_request_invalid",
            "The source revision is not a canonical Windows ChangeTime token",
        ));
    }
    Ok(())
}

fn superseded() -> ScanError {
    ScanError::new(
        "viewer_source_superseded",
        "The selected original-image source is no longer current",
    )
}

struct SourceLeaseAdmission {
    active: &'static AtomicUsize,
}

impl SourceLeaseAdmission {
    fn acquire(active: &'static AtomicUsize) -> Result<Self, ScanError> {
        active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < MAX_SOURCE_LEASES).then_some(count + 1)
            })
            .map_err(|_| {
                ScanError::new(
                    "viewer_source_busy",
                    "The original-image reader is finishing an earlier read",
                )
            })?;
        Ok(Self { active })
    }
}

impl Drop for SourceLeaseAdmission {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests;
