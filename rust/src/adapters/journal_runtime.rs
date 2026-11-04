use std::sync::Arc;

use crate::domain::ScanError;
use crate::journal_broker::{
    CallerClaim, PROTOCOL_VERSION, PersistentChangeJournal, PersistentChangeJournalConnection,
    PersistentChangeJournalLiveOnlyReason, PersistentChangeJournalSession,
    production_persistent_journal_caller_claim,
};
use crate::ports::PersistentIdentityJournalSource;
use crate::ports::journal_runtime::{
    JournalRuntimeConnection, JournalRuntimeFactory, JournalRuntimeSession,
};

#[derive(Clone)]
pub(crate) struct NativeJournalConnection {
    connection: PersistentChangeJournalConnection,
    caller: Option<CallerClaim>,
}

#[derive(Clone)]
pub(crate) struct NativeJournalSession {
    session: Arc<dyn PersistentChangeJournalSession>,
    caller: CallerClaim,
}

struct NativeJournalFactory {
    factory: Arc<dyn PersistentChangeJournal>,
}

pub(crate) fn production_journal_runtime_factory()
-> Arc<dyn JournalRuntimeFactory<Connection = NativeJournalConnection>> {
    Arc::new(NativeJournalFactory {
        factory: super::production_identity_journal_factory(),
    })
}

impl JournalRuntimeFactory for NativeJournalFactory {
    type Connection = NativeJournalConnection;

    fn connect(&self) -> Self::Connection {
        let mut connection = self.factory.connect();
        let mut caller = None;
        if matches!(connection, PersistentChangeJournalConnection::Connected(_)) {
            match production_persistent_journal_caller_claim() {
                Ok(claim) => caller = Some(claim),
                Err(_) => {
                    let connected = std::mem::replace(
                        &mut connection,
                        PersistentChangeJournalConnection::LiveOnly(
                            PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
                        ),
                    );
                    let _ = connected.close();
                }
            }
        }
        NativeJournalConnection { connection, caller }
    }
}

impl JournalRuntimeConnection for NativeJournalConnection {
    type Session = NativeJournalSession;

    fn unavailable() -> Self {
        Self {
            connection: PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::TransportUnavailable,
            ),
            caller: None,
        }
    }

    fn protocol_version(&self) -> u16 {
        PROTOCOL_VERSION
    }

    fn is_connected(&self) -> bool {
        matches!(
            self.connection,
            PersistentChangeJournalConnection::Connected(_)
        )
    }

    fn session(&self) -> Option<Self::Session> {
        match (&self.connection, &self.caller) {
            (PersistentChangeJournalConnection::Connected(session), Some(caller)) => {
                Some(NativeJournalSession {
                    session: Arc::clone(session),
                    caller: caller.clone(),
                })
            }
            _ => None,
        }
    }

    fn close(&self) -> Result<(), ScanError> {
        self.connection.close().map_err(|error| {
            ScanError::new(
                "persistent_change_journal_close_failed",
                format!("The persistent change journal did not close cleanly: {error}"),
            )
        })
    }
}

impl JournalRuntimeSession for NativeJournalSession {
    type BoundarySource<'a> = super::journal_boundary::SessionBoundarySource<'a>;
    type VolumeSource = super::persistent_journal::SessionVolumeSource;

    fn identity_source(&self) -> Option<Arc<dyn PersistentIdentityJournalSource>> {
        self.session.identity_source()
    }

    fn boundary_source(&self) -> Self::BoundarySource<'_> {
        super::journal_boundary::session_boundary_source(self.session.as_ref(), &self.caller)
    }

    fn volume_source(self) -> Self::VolumeSource {
        super::persistent_journal::volume_source(self.session, self.caller)
    }
}

#[cfg(test)]
impl NativeJournalConnection {
    pub(crate) fn for_test(connection: PersistentChangeJournalConnection) -> Self {
        Self {
            connection,
            caller: None,
        }
    }

    pub(crate) fn set_caller_for_test(&mut self, caller: CallerClaim) {
        self.caller = Some(caller);
    }
}

#[cfg(test)]
pub(crate) fn journal_runtime_factory_for_test(
    factory: Arc<dyn PersistentChangeJournal>,
) -> Arc<dyn JournalRuntimeFactory<Connection = NativeJournalConnection>> {
    Arc::new(NativeJournalFactory { factory })
}
