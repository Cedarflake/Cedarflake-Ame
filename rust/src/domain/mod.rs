use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub struct ScanRequest {
    pub scan_id: String,
    pub root_path: String,
    pub max_items: Option<u32>,
    pub max_entries: Option<u32>,
    pub preview_edge: u32,
}

#[derive(Clone, Debug)]
pub struct RecoverableScan {
    pub scan_id: String,
    pub root_path: String,
    pub display_root_path: String,
    pub max_items: Option<u32>,
    pub max_entries: Option<u32>,
    pub preview_edge: u32,
    pub visited_entries: u64,
    pub accepted_items: u64,
    pub issue_count: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ScanCheckpoint {
    pub last_visited_relative_path: Option<String>,
    pub visited_entries: u64,
    pub accepted_items: u64,
    pub issue_count: u64,
}

#[derive(Clone, Debug)]
pub struct StorageConfiguration {
    pub catalog_path: String,
    pub preview_root: String,
    pub preview_budget_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct StorageStatus {
    pub settings_path: String,
    pub active_catalog_path: String,
    pub active_preview_root: String,
    pub configured_catalog_path: String,
    pub configured_preview_root: String,
    pub configured_catalog_display_path: String,
    pub configured_preview_display_path: String,
    pub preview_budget_bytes: u64,
    pub preview_used_bytes: u64,
    pub catalog_used_bytes: u64,
    pub requires_restart: bool,
}

#[derive(Clone, Debug)]
pub struct StorageSettingsUpdate {
    pub catalog_directory: Option<String>,
    pub preview_cache_directory: Option<String>,
    pub preview_budget_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileIdentityEvidence {
    pub scheme: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct AssetLocationView {
    pub asset_id: String,
    pub location_id: String,
    pub root_id: String,
    pub absolute_path: String,
    pub display_path: String,
    pub relative_path: String,
    pub preview_path: String,
    pub file_size: u64,
    pub created_unix_ms: Option<i64>,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
    pub width: u32,
    pub height: u32,
    pub preview_status: PreviewStatus,
    pub preview_issue_code: Option<String>,
    pub preview_issue_message: Option<String>,
    pub metadata_engine_id: String,
    pub metadata_engine_version: String,
    pub capture_time: Option<CaptureTimeEvidence>,
}

#[derive(Clone, Debug)]
pub enum PreviewStatus {
    Pending,
    Ready,
    Failed,
}

#[derive(Clone, Debug)]
pub struct CaptureTimeEvidence {
    pub local_time: String,
    pub offset_minutes: Option<i16>,
    pub source: CaptureTimeSource,
    pub raw_value: String,
}

#[derive(Clone, Debug)]
pub enum CaptureTimeSource {
    Original,
    Digitized,
    Image,
}

#[derive(Clone, Debug)]
pub struct MetadataInspection {
    pub engine_id: String,
    pub engine_version: String,
    pub capture_time: Option<CaptureTimeEvidence>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Clone, Debug)]
pub struct MediaInspection {
    pub width: u32,
    pub height: u32,
    pub metadata: MetadataInspection,
}

#[derive(Clone, Debug)]
pub struct PreviewRequest {
    pub location_id: String,
    pub preview_edge: u32,
}

#[derive(Clone, Debug)]
pub struct LibraryRootView {
    pub root_id: String,
    pub path: String,
    pub display_path: String,
    pub active_scan_id: Option<String>,
    pub created_unix_ms: i64,
    pub asset_count: u64,
    pub issue_count: u64,
    pub availability: LibraryRootAvailability,
    pub availability_message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum LibraryRootAvailability {
    Unknown,
    Available,
    Missing,
    Inaccessible,
    Offline,
}

#[derive(Clone, Debug)]
pub struct RootAvailabilityEvidence {
    pub availability: LibraryRootAvailability,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CatalogSnapshot {
    pub catalog_path: String,
    pub revision: u64,
    pub query_id: String,
    pub roots: Vec<LibraryRootView>,
    pub assets: Vec<AssetLocationView>,
    pub previous_cursor: Option<CatalogCursor>,
    pub next_cursor: Option<CatalogCursor>,
}

#[derive(Clone, Debug)]
pub struct CatalogCursor {
    pub revision: u64,
    pub query_id: String,
    pub primary_missing: bool,
    pub primary_text: String,
    pub primary_number: i64,
    pub root_id: String,
    pub location_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GallerySortKey {
    CaptureTime,
    CreatedTime,
    ModifiedTime,
    FileName,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GallerySortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryQuery {
    pub root_id: Option<String>,
    pub folder_relative_path: Option<String>,
    pub include_descendants: bool,
    pub search_text: String,
    pub sort_key: GallerySortKey,
    pub sort_direction: GallerySortDirection,
}

impl Default for GalleryQuery {
    fn default() -> Self {
        Self {
            root_id: None,
            folder_relative_path: None,
            include_descendants: true,
            search_text: String::new(),
            sort_key: GallerySortKey::CaptureTime,
            sort_direction: GallerySortDirection::Descending,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryTimeBucket {
    pub month_key: Option<String>,
    pub item_count: u64,
    pub aspect_ratio_milli_sum: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryTimeline {
    pub revision: u64,
    pub query_id: String,
    pub total_items: u64,
    pub buckets: Vec<GalleryTimeBucket>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryTimeAnchor {
    pub revision: u64,
    pub query_id: String,
    pub month_key: Option<String>,
    pub item_offset: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFolderView {
    pub root_id: String,
    pub relative_path: String,
    pub name: String,
    pub direct_asset_count: u64,
    pub descendant_asset_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFolderCursor {
    pub revision: u64,
    pub root_id: String,
    pub parent_relative_path: String,
    pub relative_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFolderPage {
    pub revision: u64,
    pub root_id: String,
    pub parent_relative_path: String,
    pub folders: Vec<LibraryFolderView>,
    pub next_cursor: Option<LibraryFolderCursor>,
}

#[derive(Clone, Debug)]
pub struct ScanIssue {
    pub path: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum ScanEvent {
    Started {
        scan_id: String,
        root_path: String,
        item_limit: Option<u32>,
        entry_limit: Option<u32>,
    },
    Progress {
        scan_id: String,
        visited_entries: u64,
        accepted_items: u64,
        issue_count: u64,
    },
    AssetDiscovered {
        scan_id: String,
        asset: Box<AssetLocationView>,
    },
    Issue {
        scan_id: String,
        issue: ScanIssue,
    },
    Completed {
        scan_id: String,
        root_id: String,
        asset_count: u64,
        issue_count: u64,
        catalog_path: String,
        was_limited: bool,
    },
    Cancelled {
        scan_id: String,
        accepted_items: u64,
        issue_count: u64,
    },
    Paused {
        scan_id: String,
        visited_entries: u64,
        accepted_items: u64,
        issue_count: u64,
    },
    Stale {
        scan_id: String,
        accepted_items: u64,
        issue_count: u64,
    },
}

#[derive(Clone, Debug)]
pub struct ScanError {
    pub code: String,
    pub message: String,
}

impl ScanError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl Display for ScanError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ScanError {}

#[derive(Clone, Debug)]
pub struct DiscoveredFile {
    pub absolute_path: String,
    pub relative_path: String,
    pub file_size: u64,
    pub created_unix_ms: Option<i64>,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
    pub issues: Vec<ScanIssue>,
}

#[derive(Clone, Debug)]
pub struct PreviewArtifact {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct ExpectedFileState {
    pub absolute_path: String,
    pub file_size: u64,
    pub modified_unix_ms: i64,
    pub file_identity: Option<FileIdentityEvidence>,
}
