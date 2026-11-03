use std::path::Path;
use std::sync::Arc;

use crate::domain::{
    LibraryRootGeneration, PersistentJournalBaseline, PersistentJournalReadFailure, ScanError,
};
use crate::journal_broker::{
    CallerClaim, PersistentChangeJournalSession, ProductionPersistentJournalRoot,
    describe_production_persistent_journal_root,
};
use crate::ports::PersistentJournalNamespaceReader;
use crate::ports::journal_volume::PersistentJournalVolumeSource;

use super::production_reader::ProductionPersistentJournalVolumeReader;
use super::session_reader::{
    PersistentJournalBrokerRoot, classify_persistent_journal_operation_failure,
};

pub(crate) fn volume_source(
    session: Arc<dyn PersistentChangeJournalSession>,
    caller: CallerClaim,
) -> SessionVolumeSource {
    SessionVolumeSource { session, caller }
}

pub(crate) struct SessionVolumeSource {
    session: Arc<dyn PersistentChangeJournalSession>,
    caller: CallerClaim,
}

impl PersistentJournalVolumeSource for SessionVolumeSource {
    type Registration = ProductionPersistentJournalRoot;
    type Reader = ProductionPersistentJournalVolumeReader;

    fn open_root(
        &self,
        root_id: &str,
        root_generation: LibraryRootGeneration,
        root_path: &Path,
    ) -> Result<Self::Registration, PersistentJournalReadFailure> {
        describe_production_persistent_journal_root(root_id, root_generation.value(), root_path)
            .map_err(classify_persistent_journal_operation_failure)
    }

    fn into_reader(
        self,
        registrations: &[Self::Registration],
        namespace: Arc<dyn PersistentJournalNamespaceReader>,
        baselines: Vec<PersistentJournalBaseline>,
    ) -> Result<Self::Reader, ScanError> {
        let roots = registrations
            .iter()
            .map(|registration| PersistentJournalBrokerRoot {
                authorization: registration.authorization.clone(),
                client_root_handle: registration.client_root_handle,
                volume_serial: registration.volume_serial,
            })
            .collect();
        ProductionPersistentJournalVolumeReader::new(
            self.session,
            self.caller,
            roots,
            namespace,
            baselines,
        )
    }
}
