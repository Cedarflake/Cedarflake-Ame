use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::adapters::{SqliteCatalog, SqliteCatalogSession};
use crate::application::{
    PersistentJournalBrokerRoot, ProductionPersistentJournalVolumeReader,
    catch_up_persistent_journal_window, classify_persistent_journal_operation_failure,
};
use crate::domain::{
    LibraryChangeLane, LibraryChangeQueuePolicy, PersistentJournalCheckpoint,
    PersistentJournalEnrollmentReport, PersistentJournalRootFailure,
    PersistentJournalRootFailureKind, ScanError,
};
use crate::journal_broker::{
    CallerClaim, PersistentChangeJournalOperationError, PersistentChangeJournalSession,
    ProductionPersistentJournalRoot, describe_production_persistent_journal_root,
};
use crate::ports::PersistentJournalRepository;

use super::fallback::{FallbackReader, JournalReadFailures};
use super::{JournalTaskOutcome, JournalVolumeWork};

pub(super) struct JournalPageControl {
    pub(super) failures: Arc<JournalReadFailures>,
    pub(super) cancelled: Arc<AtomicBool>,
}

pub(super) fn run(
    work: JournalVolumeWork,
    session: Arc<dyn PersistentChangeJournalSession>,
    caller: CallerClaim,
    observed_unix_ms: i64,
    catalog_session: Arc<SqliteCatalogSession>,
    policy: LibraryChangeQueuePolicy,
    control: JournalPageControl,
) -> Result<JournalTaskOutcome, ScanError> {
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    let mut report = PersistentJournalEnrollmentReport::default();
    let mut registrations = Vec::<ProductionPersistentJournalRoot>::new();
    let mut broker_roots = Vec::new();
    let mut checkpoints = Vec::new();
    for root in work.roots {
        let checkpoint = root.checkpoint;
        let registration = match describe_production_persistent_journal_root(
            &checkpoint.root_id,
            checkpoint.root_generation.value(),
            &root.root_path,
        ) {
            Ok(registration) => registration,
            Err(error) => {
                let failure = persist_journal_registration_failure(
                    &mut catalog,
                    &checkpoint,
                    &control.failures,
                    error,
                    observed_unix_ms,
                    policy,
                )?;
                report.failed_root_count = report.failed_root_count.saturating_add(1);
                report.root_failures.push(failure);
                continue;
            }
        };
        broker_roots.push(PersistentJournalBrokerRoot {
            authorization: registration.authorization.clone(),
            client_root_handle: registration.client_root_handle,
            volume_serial: registration.volume_serial,
        });
        registrations.push(registration);
        checkpoints.push(checkpoint);
    }
    if checkpoints.is_empty() {
        return Ok(JournalTaskOutcome::Page(report.into()));
    }
    let reader = ProductionPersistentJournalVolumeReader::new(
        session,
        caller,
        broker_roots,
        Arc::new(crate::adapters::SqliteJournalNamespaceReader(Arc::clone(
            &catalog_session,
        ))),
        catalog.load_persistent_journal_baselines()?,
    )?;
    let reader = FallbackReader {
        reader,
        failures: control.failures,
    };
    let window = catch_up_persistent_journal_window(
        &mut catalog,
        &reader,
        &checkpoints,
        observed_unix_ms,
        policy,
        &control.cancelled,
    )?;
    drop(registrations);
    Ok(JournalTaskOutcome::Page(window.with_registration(report)))
}

fn persist_journal_registration_failure(
    catalog: &mut SqliteCatalog,
    checkpoint: &PersistentJournalCheckpoint,
    failures: &JournalReadFailures,
    error: PersistentChangeJournalOperationError,
    failed_unix_ms: i64,
    policy: crate::domain::LibraryChangeQueuePolicy,
) -> Result<PersistentJournalRootFailure, ScanError> {
    let mut classified = classify_persistent_journal_operation_failure(error);
    failures.classify(checkpoint, &mut classified);
    let failure = PersistentJournalRootFailure {
        root_id: checkpoint.root_id.clone(),
        root_generation: checkpoint.root_generation,
        kind: classified.kind,
        failure: classified.failure,
        opening_boundary: classified.opening_boundary.map(|boundary| *boundary),
    };
    failure.validate()?;
    if failure.kind != PersistentJournalRootFailureKind::Cancelled {
        catalog.persist_persistent_journal_root_failure(&failure, failed_unix_ms, policy)?;
    }
    Ok(failure)
}

#[cfg(test)]
mod tests;
