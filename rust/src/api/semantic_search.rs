use crate::domain::semantic_results::{SemanticResultManifest, SemanticSearchInput};
use crate::domain::{GalleryQuery, ScanError};

pub fn semantic_index_status()
-> Result<Option<crate::domain::semantic_results::SemanticIndexStatus>, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        crate::application::classification_desktop::semantic_status()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Ok(None)
    }
}

pub fn retry_semantic_index() -> Result<(), ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        crate::application::classification_desktop::retry_semantic_index()
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        Err(super::classification::unsupported())
    }
}

#[flutter_rust_bridge::frb(opaque)]
pub struct SemanticSearchRequest {
    #[cfg(all(windows, feature = "classification-worker"))]
    inner: std::sync::Arc<crate::application::semantic_requests::SearchTicket>,
}

pub fn prepare_semantic_search(
    scope: GalleryQuery,
    input: SemanticSearchInput,
) -> Result<SemanticSearchRequest, ScanError> {
    #[cfg(all(windows, feature = "classification-worker"))]
    {
        crate::application::classification_desktop::search(scope, input).map(|ticket| {
            SemanticSearchRequest {
                inner: std::sync::Arc::new(ticket),
            }
        })
    }
    #[cfg(not(all(windows, feature = "classification-worker")))]
    {
        let _ = (scope, input);
        Err(super::classification::unsupported())
    }
}

impl SemanticSearchRequest {
    pub async fn run(&self) -> Result<SemanticResultManifest, ScanError> {
        #[cfg(all(windows, feature = "classification-worker"))]
        {
            let ticket = self.inner.clone();
            flutter_rust_bridge::spawn_blocking_with(move || ticket.wait(), ())
                .await
                .map_err(|error| ScanError::new("semantic_session_failed", error.to_string()))?
        }
        #[cfg(not(all(windows, feature = "classification-worker")))]
        {
            Err(super::classification::unsupported())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        #[cfg(all(windows, feature = "classification-worker"))]
        self.inner.cancel();
    }
}
