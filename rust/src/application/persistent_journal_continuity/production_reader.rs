use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::domain::{
    PersistentJournalCheckpoint, PersistentJournalPendingRename, PersistentJournalReadFailure,
    PersistentJournalRootReadOutcome, ScanError,
};
use crate::journal_broker::{CallerClaim, PersistentChangeJournalSession};
use crate::ports::{PersistentJournalNamespaceReader, PersistentJournalVolumeReader};

use super::identity_reader::IdentityPersistentJournalVolumeReader;
use super::{PersistentJournalBrokerRoot, SessionBackedPersistentJournalVolumeReader};

pub(crate) enum ProductionPersistentJournalVolumeReader {
    Named(SessionBackedPersistentJournalVolumeReader),
    Identity(IdentityPersistentJournalVolumeReader),
}

impl ProductionPersistentJournalVolumeReader {
    pub(crate) fn new(
        session: Arc<dyn PersistentChangeJournalSession>,
        caller: CallerClaim,
        roots: Vec<PersistentJournalBrokerRoot>,
        namespace: Arc<dyn PersistentJournalNamespaceReader>,
    ) -> Result<Self, ScanError> {
        let named = SessionBackedPersistentJournalVolumeReader::with_retained_session(
            session.clone(),
            caller.clone(),
            roots.clone(),
        )?;
        if let Some(source) = session.identity_source() {
            Ok(Self::Identity(IdentityPersistentJournalVolumeReader {
                session,
                caller,
                roots,
                source,
                namespace,
            }))
        } else {
            Ok(Self::Named(named))
        }
    }
}

impl PersistentJournalVolumeReader for ProductionPersistentJournalVolumeReader {
    fn read_volume(
        &self,
        checkpoints: &[PersistentJournalCheckpoint],
        pending: &[PersistentJournalPendingRename],
        observed_unix_ms: i64,
        cancelled: &AtomicBool,
    ) -> Result<Vec<PersistentJournalRootReadOutcome>, PersistentJournalReadFailure> {
        match self {
            Self::Named(reader) => {
                reader.read_volume(checkpoints, pending, observed_unix_ms, cancelled)
            }
            Self::Identity(reader) => {
                reader.read_volume(checkpoints, pending, observed_unix_ms, cancelled)
            }
        }
    }
}
