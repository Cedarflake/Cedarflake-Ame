use crate::application::similar_images;
use crate::domain::ScanError;
use crate::domain::similar_images::SimilarAnalysisProgress;

#[flutter_rust_bridge::frb(opaque)]
pub struct SimilarImageAnalysis {
    inner: similar_images::SimilarImageAnalysis,
}

pub fn open_similar_image_analysis(run_id: String) -> Result<SimilarImageAnalysis, ScanError> {
    similar_images::open(run_id).map(|inner| SimilarImageAnalysis { inner })
}

impl SimilarImageAnalysis {
    pub fn run(&self) -> Result<SimilarAnalysisProgress, ScanError> {
        self.inner.run()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn progress(&self) -> Result<SimilarAnalysisProgress, ScanError> {
        self.inner.progress()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        self.inner.cancel();
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn suspend(&self, suspended: bool) {
        self.inner.suspend(suspended);
    }
}
