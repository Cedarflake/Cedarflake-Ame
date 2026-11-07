use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use crate::domain::semantic_results::{
    SemanticIndexStatus, SemanticResultManifest, SemanticSearchInput,
};
use crate::domain::{GalleryQuery, ScanError};

use super::classification_dispatch::ClassificationDispatchSignals;

type SearchResult = Result<SemanticResultManifest, ScanError>;

pub(crate) struct SearchRequest {
    pub scope: GalleryQuery,
    pub input: SemanticSearchInput,
    pub cancelled: Arc<AtomicBool>,
    reply: mpsc::SyncSender<SearchResult>,
}

impl SearchRequest {
    pub(crate) fn complete(self, result: SearchResult) {
        let _ = self.reply.send(result);
    }
}

#[derive(Default)]
struct State {
    pending: Option<SearchRequest>,
    active: Option<Arc<AtomicBool>>,
    closed: bool,
    index: SemanticIndexStatus,
}

#[derive(Clone, Default)]
pub(crate) struct SearchRequests(Arc<Mutex<State>>);

impl SearchRequests {
    pub(crate) fn is_closed(&self) -> Result<bool, ScanError> {
        Ok(self.0.lock().map_err(|_| unavailable())?.closed)
    }
    pub(crate) fn index_status(&self) -> Result<SemanticIndexStatus, ScanError> {
        Ok(self.0.lock().map_err(|_| unavailable())?.index.clone())
    }

    pub(crate) fn publish_index(&self, status: SemanticIndexStatus) -> Result<(), ScanError> {
        self.0.lock().map_err(|_| unavailable())?.index = status;
        Ok(())
    }

    pub(crate) fn submit(
        &self,
        signals: &ClassificationDispatchSignals,
        scope: GalleryQuery,
        input: SemanticSearchInput,
    ) -> Result<SearchTicket, ScanError> {
        if let SemanticSearchInput::Text { description } = &input
            && (description.trim().is_empty() || description.chars().count() > 512)
        {
            return Err(ScanError::new(
                "semantic_query_invalid",
                "Enter a description of at most 512 characters",
            ));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let (reply, receiver) = mpsc::sync_channel(1);
        {
            let mut state = self.0.lock().map_err(|_| unavailable())?;
            if state.closed {
                return Err(unavailable());
            }
            if let Some(active) = &state.active {
                active.store(true, Ordering::Release);
            }
            if let Some(pending) = state.pending.take() {
                pending.cancelled.store(true, Ordering::Release);
            }
            state.pending = Some(SearchRequest {
                scope: super::load_catalog::normalize_gallery_query(scope),
                input,
                cancelled: cancelled.clone(),
                reply,
            });
        }
        signals.request_interactive()?;
        Ok(SearchTicket {
            cancelled,
            receiver: Mutex::new(Some(receiver)),
        })
    }

    pub(crate) fn take(&self) -> Result<Option<SearchRequest>, ScanError> {
        let mut state = self.0.lock().map_err(|_| unavailable())?;
        let request = state.pending.take();
        state.active = request.as_ref().map(|request| request.cancelled.clone());
        Ok(request)
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.closed = true;
            if let Some(active) = &state.active {
                active.store(true, Ordering::Release);
            }
            if let Some(pending) = state.pending.take() {
                pending.cancelled.store(true, Ordering::Release);
            }
        }
    }
}

pub(crate) struct SearchTicket {
    cancelled: Arc<AtomicBool>,
    receiver: Mutex<Option<mpsc::Receiver<SearchResult>>>,
}

impl SearchTicket {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn wait(&self) -> SearchResult {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_| unavailable())?
            .take()
            .ok_or_else(unavailable)?;
        loop {
            super::semantic_retrieval::check_cancelled(&self.cancelled)?;
            match receiver.recv_timeout(Duration::from_millis(25)) {
                Ok(result) => {
                    super::semantic_retrieval::check_cancelled(&self.cancelled)?;
                    return result;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(unavailable()),
            }
        }
    }
}

impl Drop for SearchTicket {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn unavailable() -> ScanError {
    ScanError::new(
        "semantic_session_unavailable",
        "The search session is no longer available",
    )
}
