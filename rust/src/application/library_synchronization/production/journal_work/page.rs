use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::application::catch_up_persistent_journal_window;
use crate::domain::{
    LibraryChangeLane, LibraryChangeQueuePolicy, PersistentJournalCheckpoint,
    PersistentJournalEnrollmentReport, PersistentJournalReadFailure, PersistentJournalRootFailure,
    PersistentJournalRootFailureKind, ScanError,
};
use crate::ports::PersistentJournalRepository;
use crate::ports::catalog_session::JournalCatalogSession;
use crate::ports::journal_volume::PersistentJournalVolumeSource;

use super::fallback::{FallbackReader, JournalReadFailures};
use super::{JournalTaskOutcome, JournalVolumeWork};

pub(super) struct JournalPageControl {
    pub(super) failures: Arc<JournalReadFailures>,
    pub(super) cancelled: Arc<AtomicBool>,
}

pub(super) fn run<Session>(
    work: JournalVolumeWork,
    volume_source: impl PersistentJournalVolumeSource,
    observed_unix_ms: i64,
    catalog_session: Arc<Session>,
    policy: LibraryChangeQueuePolicy,
    control: JournalPageControl,
) -> Result<JournalTaskOutcome, ScanError>
where
    Session: JournalCatalogSession,
    Session::Catalog: PersistentJournalRepository,
{
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    let mut report = PersistentJournalEnrollmentReport::default();
    let mut registrations = Vec::new();
    let mut checkpoints = Vec::new();
    for root in work.roots {
        let checkpoint = root.checkpoint;
        let registration = match volume_source.open_root(
            &checkpoint.root_id,
            checkpoint.root_generation,
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
        registrations.push(registration);
        checkpoints.push(checkpoint);
    }
    if checkpoints.is_empty() {
        return Ok(JournalTaskOutcome::Page(report.into()));
    }
    let reader = volume_source.into_reader(
        &registrations,
        Arc::clone(&catalog_session).namespace_reader(),
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
    catalog: &mut impl PersistentJournalRepository,
    checkpoint: &PersistentJournalCheckpoint,
    failures: &JournalReadFailures,
    mut classified: PersistentJournalReadFailure,
    failed_unix_ms: i64,
    policy: crate::domain::LibraryChangeQueuePolicy,
) -> Result<PersistentJournalRootFailure, ScanError> {
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
