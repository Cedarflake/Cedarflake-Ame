use crate::domain::{
    GalleryQuery, GalleryQueryAnchor, GalleryQuerySnapshot, GalleryTimeIntent, GalleryTimeSnapshot,
    ScanError,
};

pub trait GalleryQueryRepository {
    fn load_time_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        intent: &GalleryTimeIntent,
    ) -> Result<GalleryTimeSnapshot, ScanError>;

    fn load_query_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor: Option<&GalleryQueryAnchor>,
    ) -> Result<GalleryQuerySnapshot, ScanError>;
}
