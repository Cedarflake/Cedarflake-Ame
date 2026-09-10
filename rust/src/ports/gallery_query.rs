use crate::domain::{GalleryQuery, GalleryQueryAnchor, GalleryQuerySnapshot, ScanError};

pub trait GalleryQueryRepository {
    fn load_query_snapshot(
        &mut self,
        max_items: u32,
        query: &GalleryQuery,
        query_id: &str,
        anchor: Option<&GalleryQueryAnchor>,
    ) -> Result<GalleryQuerySnapshot, ScanError>;
}
