use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::catalog_session::CatalogLaneSession;

use super::{SqliteCatalog, SqliteCatalogSession};

impl CatalogLaneSession for SqliteCatalogSession {
    type Catalog = SqliteCatalog;

    fn open_in_lane(&self, lane: LibraryChangeLane) -> Result<Self::Catalog, ScanError> {
        SqliteCatalogSession::open_in_lane(self, lane)
    }
}
