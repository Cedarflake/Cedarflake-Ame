use std::io::{self, Read, Seek};

use crate::domain::{DiscoveredFile, PreviewMaterialization, ScanIssue};

pub trait PreviewSource {
    type Reader: Read + Seek;

    /// Read the admitted source from byte zero without reopening its path.
    /// Readers may share a cursor and must be consumed sequentially.
    fn reader_from_start(&self) -> io::Result<Self::Reader>;
}

pub trait PreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        source: &impl PreviewSource,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
        force_regenerate: bool,
    ) -> Result<PreviewMaterialization, ScanIssue>;
}
