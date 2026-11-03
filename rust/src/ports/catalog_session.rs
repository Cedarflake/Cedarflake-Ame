use std::sync::Arc;

use crate::domain::{LibraryChangeLane, ScanError};

use super::PersistentJournalNamespaceReader;

/// Opens an owned lane connection from an already validated, shareable catalog session.
pub(crate) trait CatalogLaneSession: Send + Sync + 'static {
    type Catalog;

    fn open_in_lane(&self, lane: LibraryChangeLane) -> Result<Self::Catalog, ScanError>;
}

/// Shares the same validated session with bounded journal namespace reads.
pub(crate) trait JournalCatalogSession: CatalogLaneSession {
    fn namespace_reader(self: Arc<Self>) -> Arc<dyn PersistentJournalNamespaceReader>;
}
