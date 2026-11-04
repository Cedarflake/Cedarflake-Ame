use std::sync::atomic::AtomicBool;

use crate::domain::{IncrementalCatalogRoot, ScanError};

use super::source_publication::PublicationNamespace;

/// Retains the located directory and ancestor bindings through publication or rollback.
pub(crate) struct LocatedLibraryRoot {
    path: String,
    guard: Box<dyn PublicationNamespace + Send + Sync>,
}

impl LocatedLibraryRoot {
    pub(crate) fn new(path: String, guard: Box<dyn PublicationNamespace + Send + Sync>) -> Self {
        Self { path, guard }
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn guard(&self) -> &dyn PublicationNamespace {
        self.guard.as_ref()
    }
}

pub(crate) trait RootLocationRepository {
    fn recover_located_root(
        &mut self,
        expected: &IncrementalCatalogRoot,
        located: &LocatedLibraryRoot,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError>;
}
