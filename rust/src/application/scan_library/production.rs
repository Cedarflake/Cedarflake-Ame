use std::path::Path;

use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::scan_admission::ScanAdmissionRepository;
use crate::ports::scan_discovery::ScanFileDiscovery;
use crate::ports::scan_publication::ScanPublicationRepository;
use crate::ports::source_publication::PublicationNamespaceFactory;
use crate::ports::source_state::SourceStateProbe;
use crate::ports::{MediaInspector, PersistentJournalRepository};

pub(super) fn catalog(
    path: &Path,
    scan_id: &str,
) -> Result<
    impl ScanAdmissionRepository + ScanPublicationRepository + PersistentJournalRepository + use<>,
    ScanError,
> {
    crate::application::catalog_session::open_catalog_for_foreground_scan(
        path,
        LibraryChangeLane::Recovery,
        scan_id,
    )
}

pub(super) fn publication_namespaces() -> impl PublicationNamespaceFactory {
    crate::adapters::LocalPublicationNamespaces
}

pub(super) fn source_state() -> impl SourceStateProbe {
    crate::adapters::LocalSourceState
}

pub(super) fn file_discovery(root_path: &str) -> Result<impl ScanFileDiscovery + use<>, ScanError> {
    crate::adapters::FileDiscovery::new(root_path)
}

pub(super) fn media_inspector() -> impl MediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
