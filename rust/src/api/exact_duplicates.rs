use crate::application::exact_duplicates;
use crate::domain::{
    ExactAnalysisProgress, ExactDuplicateGroupCursor, ExactDuplicateGroupPage,
    ExactDuplicateLocationPage, ScanError,
};
use crate::frb_generated::StreamSink;

pub fn load_gallery_duplicates(
    revision: u64,
    location_ids: Vec<String>,
) -> Result<crate::domain::GalleryDuplicatePage, ScanError> {
    crate::application::gallery_duplicates::load_gallery_duplicates(revision, location_ids)
}

pub fn analyze_exact_duplicates(
    run_id: String,
    sink: StreamSink<ExactAnalysisProgress>,
) -> Result<(), ScanError> {
    exact_duplicates::run_exact_analysis(run_id, |progress| sink.add(progress).is_ok())
}

pub fn cancel_exact_duplicate_analysis(run_id: String) -> Result<(), ScanError> {
    exact_duplicates::cancel_exact_analysis(&run_id)
}

pub fn load_exact_duplicate_groups(
    after: Option<ExactDuplicateGroupCursor>,
) -> Result<ExactDuplicateGroupPage, ScanError> {
    exact_duplicates::load_exact_groups(after)
}

pub fn load_exact_duplicate_locations(
    digest: String,
    byte_length: u64,
    after_location_id: Option<String>,
) -> Result<ExactDuplicateLocationPage, ScanError> {
    exact_duplicates::load_exact_locations(digest, byte_length, after_location_id)
}
