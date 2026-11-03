use std::path::Path;

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRootGeneration,
    PersistentJournalVolumeIdentity, ScanError,
};
use crate::journal_broker::{
    BrokerResponse, CallerClaim, JournalCapability, PersistentChangeJournalOperationError,
    PersistentChangeJournalSession, ProductionPersistentJournalRoot, QueryJournalRequest,
    RegisterRootRequest, describe_production_persistent_journal_root,
};
use crate::ports::journal_boundary::{
    JournalBoundaryProbe, JournalBoundaryReader, JournalBoundarySource, SupportedJournalBoundary,
};

mod resume;
pub(crate) use resume::resume_boundary_source;

#[cfg(test)]
mod tests;

const JOURNAL_BOUNDARY_TIMEOUT_MILLIS: u32 = 10_000;

pub(crate) fn session_boundary_source<'a>(
    session: &'a dyn PersistentChangeJournalSession,
    caller: &'a CallerClaim,
) -> SessionBoundarySource<'a> {
    SessionBoundarySource { session, caller }
}

pub(crate) struct SessionBoundarySource<'a> {
    session: &'a dyn PersistentChangeJournalSession,
    caller: &'a CallerClaim,
}

pub(crate) struct SessionBoundaryRoot<'a> {
    session: &'a dyn PersistentChangeJournalSession,
    caller: &'a CallerClaim,
    registration: ProductionPersistentJournalRoot,
}

impl<'a> JournalBoundarySource for SessionBoundarySource<'a> {
    type Root = SessionBoundaryRoot<'a>;

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
        .map_err(map_journal_operation_error)?;
        Ok(SessionBoundaryRoot {
            session: self.session,
            caller: self.caller,
            registration,
        })
    }
}

impl JournalBoundaryReader for SessionBoundaryRoot<'_> {
    fn read_boundary(&self) -> Result<JournalBoundaryProbe, ScanError> {
        probe_journal_boundary(self.session, self.caller, &self.registration)
    }
}

fn probe_journal_boundary(
    session: &dyn PersistentChangeJournalSession,
    caller: &CallerClaim,
    registration: &ProductionPersistentJournalRoot,
) -> Result<JournalBoundaryProbe, ScanError> {
    let capability = session
        .register_root(RegisterRootRequest {
            caller: caller.clone(),
            root: registration.authorization.clone(),
            client_root_handle: registration.client_root_handle,
            timeout_ms: JOURNAL_BOUNDARY_TIMEOUT_MILLIS,
        })
        .map_err(map_journal_operation_error)?;
    match session
        .query_journal(QueryJournalRequest {
            caller: caller.clone(),
            root: registration.authorization.clone(),
            root_capability: capability,
            timeout_ms: JOURNAL_BOUNDARY_TIMEOUT_MILLIS,
        })
        .map_err(map_journal_operation_error)?
    {
        BrokerResponse::Journal {
            capability: JournalCapability::Supported,
            journal_id: Some(journal_id),
            first_usn: Some(first_usn),
            next_usn: Some(next_usn),
            ..
        } if first_usn >= 0 && next_usn >= first_usn => {
            Ok(JournalBoundaryProbe::Supported(SupportedJournalBoundary {
                volume: if let Some(source) = session.identity_source() {
                    source.volume_identity(&crate::ports::IdentityJournalRoot {
                        root_id: registration.authorization.root_id.clone(),
                        root_generation: LibraryRootGeneration::new(
                            registration.authorization.root_generation,
                        )
                        .ok_or_else(|| {
                            ScanError::new(
                                "identity_journal_generation_invalid",
                                "The journal root generation is invalid",
                            )
                        })?,
                    })?
                } else {
                    PersistentJournalVolumeIdentity {
                        volume_guid: registration.authorization.volume_id.clone(),
                        volume_serial: registration.volume_serial,
                    }
                },
                root_file_reference: JournalFileReference::from_bytes(
                    &registration.authorization.root_identity,
                )?,
                journal_id: JournalIdentifier::new(journal_id)?,
                first_usn: JournalUsn::new(first_usn)?,
                next_usn: JournalUsn::new(next_usn)?,
            }))
        }
        BrokerResponse::Journal {
            capability: JournalCapability::LiveOnly,
            journal_id: None,
            first_usn: None,
            next_usn: None,
            ..
        } => Ok(JournalBoundaryProbe::LiveOnly),
        _ => Err(ScanError::new(
            "persistent_journal_boundary_invalid",
            "The retained change boundary response is incomplete or inconsistent",
        )),
    }
}

fn map_journal_operation_error(error: PersistentChangeJournalOperationError) -> ScanError {
    ScanError::new(
        "persistent_journal_boundary_unavailable",
        format!("The retained change boundary is unavailable: {error}"),
    )
}
