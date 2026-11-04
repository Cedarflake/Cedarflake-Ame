use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::exact_duplicates::ExactContentReader;
use crate::ports::similar_images::{SimilarAnalysisRepository, SimilarImageReader};

pub(super) fn repository() -> Result<impl SimilarAnalysisRepository, ScanError> {
    let storage = crate::application::storage_paths()?;
    crate::application::catalog_session::open_catalog(
        &storage.catalog_path,
        LibraryChangeLane::Recovery,
    )
}

pub(super) fn fingerprint_reader() -> impl ExactContentReader {
    crate::adapters::GuardedExactContentReader
}

pub(super) fn image_reader() -> impl SimilarImageReader {
    crate::adapters::GuardedSimilarImageReader
}
