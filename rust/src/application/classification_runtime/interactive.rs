use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::domain::semantic_results::SemanticSearchInput;
use crate::domain::{LibraryChangeLane, ScanError};
use crate::ports::classification_encoder::ClassificationEncoderKind;
use crate::ports::classification_execution::ClassificationWorkExecutor;

use super::super::catalog_session::open_catalog;
use super::super::semantic_requests::SearchRequests;
use super::ClassificationDispatchSignals;
use super::executor::NativeExecutor;
use super::semantic::SemanticExecution;

pub(super) struct SemanticSearchWorker {
    requests: SearchRequests,
    signals: ClassificationDispatchSignals,
    thread: Option<JoinHandle<Result<(), ScanError>>>,
}

impl SemanticSearchWorker {
    pub(super) fn start(
        mut executor: NativeExecutor,
        requests: SearchRequests,
        signals: ClassificationDispatchSignals,
    ) -> Result<Self, ScanError> {
        let search = requests.clone();
        let notification = signals.clone();
        let thread = thread::Builder::new()
            .name("ame-semantic-search".into())
            .spawn(move || {
                let result = (|| {
                    let mut catalog =
                        open_catalog(executor.database(), LibraryChangeLane::Recovery)?;
                    let scheduler = executor.scheduler();
                    let mut execution = SemanticExecution::default();
                    loop {
                        let observed = notification.notification()?;
                        if search.is_closed()? || notification.is_closed()? {
                            return Ok(());
                        }
                        if let Some(request) = search.take()? {
                            let kind = match request.input {
                                SemanticSearchInput::Text { .. } => ClassificationEncoderKind::Text,
                                SemanticSearchInput::Image { .. } => {
                                    ClassificationEncoderKind::Image
                                }
                            };
                            let session = scheduler.foreground(kind)?;
                            let result = execution.search(&mut catalog, &mut executor, &request);
                            let result = execution.finish(&mut catalog).and(result);
                            request.complete(result);
                            drop(session);
                            notification.request_interactive()?;
                        } else {
                            executor.retire_if_idle()?;
                            notification.wait(observed, Some(Duration::from_secs(2)))?;
                        }
                    }
                })();
                search.close();
                result
            })
            .map_err(|error| ScanError::new("semantic_worker_unavailable", error.to_string()))?;
        Ok(Self {
            requests,
            signals,
            thread: Some(thread),
        })
    }

    pub(super) fn stop(&mut self) -> Result<(), ScanError> {
        self.requests.close();
        let wake = self.signals.request_interactive();
        let joined = if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| {
                ScanError::new(
                    "semantic_worker_failed",
                    "The search worker stopped unexpectedly",
                )
            })?
        } else {
            Ok(())
        };
        joined.and(wake)
    }
}

impl Drop for SemanticSearchWorker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
