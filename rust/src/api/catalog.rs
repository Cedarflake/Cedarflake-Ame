use crate::application::{
    cancel_scan, load_catalog, load_catalog_around_asset, load_catalog_around_location,
    load_catalog_asset_by_id, load_catalog_at_time, load_gallery_layout_manifest_chunk,
    load_gallery_timeline, load_library_folders, load_paused_scan, load_recoverable_scan,
    pause_scan, resume_scan, run_scan, unregister_library_root,
};
use crate::domain::{
    CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk, GalleryLayoutManifestCursor,
    GalleryQuery, GalleryTimeAnchor, GalleryTimeline, LibraryFolderCursor, LibraryFolderPage,
    RecoverableScan, ScanError, ScanEvent, ScanRequest,
};
use crate::frb_generated::StreamSink;

pub fn scan_library(request: ScanRequest, sink: StreamSink<ScanEvent>) -> Result<(), ScanError> {
    scan_library_with(
        request,
        |event| sink.add(event).is_ok(),
        |request, publish| run_scan(request, publish),
    )
}

pub fn resume_library_scan(
    request: ScanRequest,
    sink: StreamSink<ScanEvent>,
) -> Result<(), ScanError> {
    scan_library_with(
        request,
        |event| sink.add(event).is_ok(),
        |request, publish| resume_scan(request, publish),
    )
}

fn scan_library_with(
    request: ScanRequest,
    mut emit: impl FnMut(ScanEvent) -> bool,
    runner: impl FnOnce(ScanRequest, &mut dyn FnMut(ScanEvent) -> bool) -> Result<(), ScanError>,
) -> Result<(), ScanError> {
    let scan_id = request.scan_id.clone();
    let result = {
        let mut publish = |event| {
            if matches!(&event, ScanEvent::AssetDiscovered { .. }) {
                return true;
            }
            emit(event)
        };
        runner(request, &mut publish)
    };
    if let Err(error) = result {
        let _ = emit(ScanEvent::Failed {
            scan_id,
            code: error.code,
            message: error.message,
        });
    }
    Ok(())
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

pub fn load_library_catalog_around_location(
    max_items: u32,
    query: GalleryQuery,
    anchor_location_id: String,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_around_location(max_items, query, anchor_location_id)
}

pub fn load_library_catalog_around_asset(
    max_items: u32,
    query: GalleryQuery,
    requested_location_id: String,
    anchor_asset_id: String,
    fallback_ordinal: u64,
) -> Result<CatalogSnapshot, ScanError> {
    load_catalog_around_asset(
        max_items,
        query,
        requested_location_id,
        anchor_asset_id,
        fallback_ordinal,
    )
}

pub fn load_library_asset_by_id(
    asset_id: String,
    preferred_location_id: Option<String>,
) -> Result<Option<crate::domain::AssetLocationView>, ScanError> {
    load_catalog_asset_by_id(asset_id, preferred_location_id)
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

#[cfg(test)]
mod tests {
    use crate::domain::{AssetLocationView, PreviewStatus};

    use super::*;

    #[test]
    fn scan_errors_are_emitted_as_one_terminal_failure_without_asset_events() {
        let request = ScanRequest {
            scan_id: "scan-failure".to_owned(),
            root_path: "unused".to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 256,
        };
        let mut emitted = Vec::new();

        let result = scan_library_with(
            request,
            |event| {
                emitted.push(event);
                true
            },
            |_request, publish| {
                assert!(publish(ScanEvent::AssetDiscovered {
                    scan_id: "scan-failure".to_owned(),
                    asset: Box::new(AssetLocationView {
                        asset_id: "asset".to_owned(),
                        location_id: "location".to_owned(),
                        root_id: "root".to_owned(),
                        absolute_path: "C:\\Pictures\\asset.png".to_owned(),
                        display_path: "C:\\Pictures\\asset.png".to_owned(),
                        relative_path: "asset.png".to_owned(),
                        preview_path: String::new(),
                        file_size: 1,
                        created_unix_ms: None,
                        modified_unix_ms: 1,
                        file_identity: None,
                        width: 1,
                        height: 1,
                        preview_status: PreviewStatus::Pending,
                        preview_issue_code: None,
                        preview_issue_message: None,
                        metadata_engine_id: "fixture".to_owned(),
                        metadata_engine_version: "1".to_owned(),
                        capture_time: None,
                    }),
                }));
                Err(ScanError::new(
                    "catalog_database_busy",
                    "catalog writer remained busy",
                ))
            },
        );

        assert!(result.is_ok());
        assert_eq!(emitted.len(), 1);
        assert!(matches!(
            &emitted[0],
            ScanEvent::Failed {
                scan_id,
                code,
                message,
            } if scan_id == "scan-failure"
                && code == "catalog_database_busy"
                && message == "catalog writer remained busy"
        ));
    }
}
