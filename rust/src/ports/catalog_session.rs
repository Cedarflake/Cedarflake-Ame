use crate::domain::{LibraryChangeLane, ScanError};

/// Opens an owned lane connection from an already validated, shareable catalog session.
pub(crate) trait CatalogLaneSession: Send + Sync + 'static {
    type Catalog;

    fn open_in_lane(&self, lane: LibraryChangeLane) -> Result<Self::Catalog, ScanError>;
}
