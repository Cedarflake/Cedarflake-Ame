use std::path::Path;

use crate::domain::{LibraryRootGeneration, ScanError};
use crate::journal_broker::{
    CallerClaim, PersistentChangeJournalConnection, PersistentChangeJournalOperationError,
    ProductionPersistentJournalRoot, describe_production_persistent_journal_root,
    production_persistent_journal_caller_claim,
};
use crate::ports::journal_boundary::{
    JournalBoundaryProbe, JournalBoundaryReader, JournalBoundarySession, JournalBoundarySource,
};

pub(crate) fn resume_boundary_source() -> impl JournalBoundarySource<Root: JournalBoundarySession> {
    ResumeBoundarySource
}

struct ResumeBoundarySource;

struct ResumeBoundarySession {
    connection: PersistentChangeJournalConnection,
    caller: CallerClaim,
    registration: ProductionPersistentJournalRoot,
}

impl JournalBoundarySource for ResumeBoundarySource {
    type Root = ResumeBoundarySession;

    fn open_root(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        root_path: &Path,
    ) -> Result<Self::Root, ScanError> {
        let registration = describe_production_persistent_journal_root(
            root_id,
            root_generation.value(),
            root_path,
        )
        .map_err(probe_error)?;
        let caller = production_persistent_journal_caller_claim().map_err(probe_error)?;
        let connection = super::super::production_identity_journal_factory().connect();
        Ok(ResumeBoundarySession {
            registration,
            caller,
            connection,
        })
    }
}

impl JournalBoundaryReader for ResumeBoundarySession {
    fn read_boundary(&self) -> Result<JournalBoundaryProbe, ScanError> {
        match &self.connection {
            PersistentChangeJournalConnection::Connected(session) => {
                super::probe_journal_boundary(session.as_ref(), &self.caller, &self.registration)
            }
            PersistentChangeJournalConnection::LiveOnly(_) => Ok(JournalBoundaryProbe::LiveOnly {
                protocol_version: crate::journal_broker::PROTOCOL_VERSION,
            }),
        }
    }
}

impl JournalBoundarySession for ResumeBoundarySession {
    fn close(&self) -> Result<(), ScanError> {
        self.connection.close().map_err(probe_error)
    }
}

fn probe_error(error: PersistentChangeJournalOperationError) -> ScanError {
    ScanError::new(
        "scan_resume_journal_unavailable",
        format!("The paused import's change coverage could not be checked: {error}"),
    )
}
