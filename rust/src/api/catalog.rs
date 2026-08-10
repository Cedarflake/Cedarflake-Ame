use crate::application::{
    cancel_scan, load_catalog, load_catalog_at_time, load_gallery_layout_manifest_chunk,
    load_gallery_timeline, load_library_folders, load_paused_scan, load_recoverable_scan,
    pause_scan, run_scan, unregister_library_root,
};
use crate::domain::{
    CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk, GalleryLayoutManifestCursor,
    GalleryQuery, GalleryTimeAnchor, GalleryTimeline, LibraryFolderCursor, LibraryFolderPage,
    RecoverableScan, ScanError, ScanEvent, ScanRequest,
};
use crate::frb_generated::StreamSink;

pub fn scan_library(request: ScanRequest, sink: StreamSink<ScanEvent>) -> Result<(), ScanError> {
    run_scan(request, |event| sink.add(event).is_ok())
}

pub fn load_library_catalog(
    max_items: u32,
    query: GalleryQuery,
    after: Option<CatalogCursor>,
    before: Option<CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog(max_items, query, after, before)
}

#[flutter_rust_bridge::frb(sync)]
pub fn load_library_gallery_timeline(query: GalleryQuery) -> Result<GalleryTimeline, ScanError> {
    load_gallery_timeline(query)
}

pub fn load_library_gallery_layout_manifest_chunk(
    max_items: u32,
    query: GalleryQuery,
    after: Option<GalleryLayoutManifestCursor>,
) -> Result<GalleryLayoutManifestChunk, ScanError> {
    load_gallery_layout_manifest_chunk(max_items, query, after)
}

#[flutter_rust_bridge::frb(sync)]
pub fn load_library_folder_page(
    root_id: String,
    parent_relative_path: String,
    max_items: u32,
    after: Option<LibraryFolderCursor>,
) -> Result<LibraryFolderPage, ScanError> {
    load_library_folders(root_id, parent_relative_path, max_items, after)
}

pub fn load_library_catalog_at_time(
    max_items: u32,
    query: GalleryQuery,
    anchor: GalleryTimeAnchor,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_at_time(max_items, query, anchor)
}

#[flutter_rust_bridge::frb(sync)]
pub fn remove_library_root(root_id: String) -> Result<bool, ScanError> {
    unregister_library_root(root_id)
}

#[flutter_rust_bridge::frb(sync)]
pub fn load_recoverable_library_scan() -> Result<Option<RecoverableScan>, ScanError> {
    load_recoverable_scan()
}

#[flutter_rust_bridge::frb(sync)]
pub fn load_paused_library_scan() -> Result<Option<RecoverableScan>, ScanError> {
    load_paused_scan()
}

#[flutter_rust_bridge::frb(sync)]
pub fn cancel_library_scan(scan_id: String) -> bool {
    cancel_scan(&scan_id)
}

#[flutter_rust_bridge::frb(sync)]
pub fn pause_library_scan(scan_id: String) -> bool {
    pause_scan(&scan_id)
}
