use crate::domain::{
    AssetLocationView, CatalogCursor, CatalogSnapshot, GalleryLayoutManifestChunk,
    GalleryLayoutManifestCursor, GalleryQuery, GalleryQueryAnchor, GalleryQuerySnapshot,
    GalleryTimeAnchor, GalleryTimeIntent, GalleryTimeSnapshot, GalleryTimeline,
    IncrementalCatalogRoot, LibraryFolderCursor, LibraryFolderPage, ScanError,
};

/// Bounded reads return owned values and retire each read snapshot before returning.
pub(crate) trait CatalogReader {
    fn load_snapshot(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&CatalogCursor>,
        before: Option<&CatalogCursor>,
        anchor: Option<&GalleryTimeAnchor>,
    ) -> Result<CatalogSnapshot, ScanError>;

    fn load_query_snapshot(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor: Option<&GalleryQueryAnchor>,
    ) -> Result<GalleryQuerySnapshot, ScanError>;

    fn load_time_snapshot(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        intent: &GalleryTimeIntent,
    ) -> Result<GalleryTimeSnapshot, ScanError>;

    fn load_snapshot_around_location(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor_location_id: &str,
    ) -> Result<CatalogSnapshot, ScanError>;

    fn load_snapshot_around_asset(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        requested_location_id: &str,
        anchor_asset_id: &str,
        fallback_ordinal: u64,
    ) -> Result<CatalogSnapshot, ScanError>;

    fn load_gallery_timeline(
        &self,
        query: &GalleryQuery,
        query_id: &str,
    ) -> Result<GalleryTimeline, ScanError>;

    fn load_gallery_layout_manifest_chunk(
        &self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        after: Option<&GalleryLayoutManifestCursor>,
    ) -> Result<GalleryLayoutManifestChunk, ScanError>;

    fn load_folder_page(
        &self,
        root_id: &str,
        parent_relative_path: &str,
        max_items: u32,
        after: Option<&LibraryFolderCursor>,
    ) -> Result<LibraryFolderPage, ScanError>;

    fn load_active_location_by_asset_id(
        &self,
        asset_id: &str,
        preferred_location_id: Option<&str>,
    ) -> Result<Option<AssetLocationView>, ScanError>;

    fn load_preview_context(
        &self,
        location_id: &str,
    ) -> Result<(Option<AssetLocationView>, Option<IncrementalCatalogRoot>), ScanError>;
}
