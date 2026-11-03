use std::sync::Mutex;

use super::*;
use crate::journal_broker::{
    PersistentChangeJournalRead, ReadJournalRangeRequest, ReadJournalVolumeRequest,
    ResponseBinding, RootCapability,
};

struct RecordingBoundarySession {
    registered: Mutex<Option<RegisterRootRequest>>,
    queried: Mutex<Option<QueryJournalRequest>>,
}

impl RecordingBoundarySession {
    fn new() -> Self {
        Self {
            registered: Mutex::new(None),
            queried: Mutex::new(None),
        }
    }
}

impl PersistentChangeJournalSession for RecordingBoundarySession {
    fn register_root(
        &self,
        request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        *self.registered.lock().expect("record register request") = Some(request);
        Ok(RootCapability([7; 32]))
    }

    fn query_journal(
        &self,
        request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        *self.queried.lock().expect("record query request") = Some(request.clone());
        Ok(BrokerResponse::Journal {
            request_id: 1,
            binding: ResponseBinding::from_request(&request.caller, &request.root),
            capability: JournalCapability::Supported,
            journal_id: Some(44),
            first_usn: Some(1),
            next_usn: Some(20),
        })
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        _request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        Ok(())
    }
}

#[test]
fn boundary_probe_binds_register_and_query_requests_to_the_same_root() {
    let directory = tempfile::tempdir().expect("test directory");
    let source_root = directory.path().join("source");
    std::fs::create_dir_all(&source_root).expect("source root");
    let caller = CallerClaim {
        process_id: std::process::id(),
        session_id: 1,
        client_instance: [9; 16],
    };
    let session = RecordingBoundarySession::new();

    let source = SessionBoundarySource {
        session: &session,
        caller: &caller,
    };
    let root = source
        .open_root("root-a", LibraryRootGeneration::initial(), &source_root)
        .expect("describe journal root");
    let boundary = root.read_boundary().expect("probe journal boundary");
    let registration = &root.registration;

    let register = session
        .registered
        .lock()
        .expect("registered request")
        .clone()
        .expect("register request");
    let query = session
        .queried
        .lock()
        .expect("queried request")
        .clone()
        .expect("query request");
    assert_eq!(register.caller, caller);
    assert_eq!(register.root, registration.authorization);
    assert_eq!(register.client_root_handle, registration.client_root_handle);
    assert_eq!(register.timeout_ms, JOURNAL_BOUNDARY_TIMEOUT_MILLIS);
    assert_eq!(query.caller, caller);
    assert_eq!(query.root, register.root);
    assert_eq!(query.root_capability, RootCapability([7; 32]));
    assert_eq!(query.timeout_ms, JOURNAL_BOUNDARY_TIMEOUT_MILLIS);
    assert!(matches!(
        boundary,
        JournalBoundaryProbe::Supported(SupportedJournalBoundary { next_usn, protocol_version, .. })
            if next_usn.value() == 20 && protocol_version == PROTOCOL_VERSION
    ));
}

#[test]
fn broker_failure_maps_to_boundary_unavailable() {
    let error =
        map_journal_operation_error(PersistentChangeJournalOperationError::TransportUnavailable);

    assert_eq!(error.code, "persistent_journal_boundary_unavailable");
    assert!(error.message.contains("transport is unavailable"));
}
