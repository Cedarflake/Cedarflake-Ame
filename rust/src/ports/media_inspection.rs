use std::io::{Read, Seek};

use crate::domain::{DiscoveredFile, MediaInspection};

use super::{MediaInspectionFailure, MediaInspector};

/// Open content through the admitted source binding without hydrating placeholders.
pub(crate) trait MediaSource {
    type Reader: Read + Seek;

    fn open_media_reader(&self, file: &DiscoveredFile) -> std::io::Result<Self::Reader>;
}

pub(crate) trait GuardedMediaInspector: MediaInspector {
    fn inspect_with_source(
        &self,
        source: &impl MediaSource,
        file: &DiscoveredFile,
    ) -> Result<MediaInspection, MediaInspectionFailure>;
}
