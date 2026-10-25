#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GalleryScrollSectionKind {
    Name,
    NewArrivals,
    Earlier,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryScrollSection {
    pub kind: GalleryScrollSectionKind,
    pub key: String,
    pub start_ordinal: u64,
    pub item_count: u64,
}
