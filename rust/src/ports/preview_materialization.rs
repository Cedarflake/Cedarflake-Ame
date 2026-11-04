use std::io::{self, Read, Seek};

use crate::domain::{DiscoveredFile, PreviewArtifact, PreviewMaterialization, ScanIssue};

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
    fn commit(&self, materialization: PreviewMaterialization)
    -> Result<PreviewArtifact, ScanIssue>;
    fn discard_staged(&self, materialization: &PreviewMaterialization) -> Result<(), ScanIssue>;
    fn has_usable_artifact(&self, artifact_path: &str) -> bool;
    fn take_rejected_reservation_bytes(&self) -> u64;
}
