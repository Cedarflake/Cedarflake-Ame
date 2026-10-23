use std::collections::BTreeMap;

use crate::adapters::SqliteCatalog;
use crate::domain::{
    LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason, LibraryRootGeneration,
    PersistentJournalCapabilityState, PersistentJournalFailure, PersistentJournalRootFailure,
    PersistentJournalRootFailureKind, ScanError,
};
use crate::journal_broker::PersistentChangeJournalConnection;
use crate::ports::PersistentJournalRepository;

#[derive(Default)]
pub(super) struct JournalFallbackSession {
    seen: BTreeMap<String, LibraryRootGeneration>,
    interrupted: BTreeMap<String, LibraryRootGeneration>,
}

impl JournalFallbackSession {
    pub(super) fn prepare(
        &mut self,
        catalog: &mut SqliteCatalog,
        now: i64,
        policy: LibraryChangeQueuePolicy,
        connection: &PersistentChangeJournalConnection,
    ) -> Result<(), ScanError> {
        let capabilities = catalog.load_persistent_journal_capabilities()?;
        let disconnected = matches!(connection, PersistentChangeJournalConnection::LiveOnly(_));
        self.seen.retain(|id, generation| {
            capabilities
                .iter()
                .any(|root| root.root_id == *id && root.root_generation == *generation)
        });
        self.interrupted
            .retain(|id, generation| self.seen.get(id) == Some(generation));
        for capability in capabilities {
            let id = &capability.root_id;
            let generation = capability.root_generation;
            let first_observation = self.seen.insert(id.clone(), generation) != Some(generation);
            if first_observation && catalog.has_retained_recovery_inventory(id, generation)? {
                self.interrupted.insert(id.clone(), generation);
            }
            let is_fallback = capability.failure.as_ref().is_some_and(|failure| {
                failure.code == crate::domain::persistent_journal::METADATA_FALLBACK_CODE
            });
            if !is_fallback {
                if disconnected
                    && capability.state == PersistentJournalCapabilityState::Supported
                    && catalog
                        .load_persistent_journal_checkpoint(id, generation)?
                        .is_some()
                {
                    admit_verification(
                        catalog,
                        id,
                        generation,
                        now,
                        policy,
                        "persistent_journal_connection_unavailable",
                    )?;
                }
                continue;
            }
            if first_observation {
                if !catalog.resume_journal_after_metadata_fallback(id, generation, now)? {
                    self.interrupted.insert(id.clone(), generation);
                } else if disconnected {
                    admit_verification(
                        catalog,
                        id,
                        generation,
                        now,
                        policy,
                        "persistent_journal_connection_unavailable",
                    )?;
                }
            } else if self.interrupted.get(id) == Some(&generation)
                && catalog.resume_journal_after_metadata_fallback(id, generation, now)?
            {
                // An inventory resumed across process shutdown has no watcher coverage
                // for the closed interval. Finish its work, then take one fresh pass.
                admit_verification(
                    catalog,
                    id,
                    generation,
                    now,
                    policy,
                    "metadata_fallback_interrupted",
                )?;
                self.interrupted.remove(id);
            }
        }
        Ok(())
    }
}

fn admit_verification(
    catalog: &mut SqliteCatalog,
    root_id: &str,
    generation: LibraryRootGeneration,
    now: i64,
    policy: LibraryChangeQueuePolicy,
    failure_code: &str,
) -> Result<(), ScanError> {
    catalog.persist_persistent_journal_root_failure(
        &PersistentJournalRootFailure {
            root_id: root_id.to_owned(),
            root_generation: generation,
            kind: PersistentJournalRootFailureKind::AutomaticMetadataFallback(
                LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure,
            ),
            failure: PersistentJournalFailure {
                code: failure_code.to_owned(),
                message: "The closed interval requires metadata verification".into(),
            },
            opening_boundary: None,
        },
        now,
        policy,
    )?;
    Ok(())
}
