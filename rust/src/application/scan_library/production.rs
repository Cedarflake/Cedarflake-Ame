use crate::domain::ScanError;
use crate::ports::MediaInspector;
use crate::ports::scan_discovery::ScanFileDiscovery;

pub(super) fn file_discovery(root_path: &str) -> Result<impl ScanFileDiscovery + use<>, ScanError> {
    crate::adapters::FileDiscovery::new(root_path)
}

pub(super) fn media_inspector() -> impl MediaInspector {
    crate::adapters::LocalMediaInspector::new()
}
