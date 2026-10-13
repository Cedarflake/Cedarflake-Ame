use std::sync::atomic::AtomicBool;

use crate::adapters::{SqliteCatalog, SqliteCatalogSession};
use crate::application::scan_library::FirstImportCaptureLease;
use crate::domain::{
    LibraryChangeLane, LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaselineStartRequest, ScanError,
};
use crate::journal_broker::{
    CallerClaim, PersistentChangeJournalSession, describe_production_persistent_journal_root,
};
use crate::ports::PersistentJournalRepository;

use super::{
    JournalBaselineOpeningAuthority, JournalBaselineOpeningWork, JournalBoundaryProbe,
    SupportedJournalBoundary, ensure_boundary_not_cancelled, live_only_capability,
    map_journal_operation_error, probe_journal_boundary,
};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::application::library_synchronization) enum OpeningCaptureOutcome {
    Recorded,
    Retired,
}

pub(in crate::application::library_synchronization) fn persist_unavailable_session(
    catalog: &mut SqliteCatalog,
    work: &JournalBaselineOpeningWork,
    observed_unix_ms: i64,
) -> Result<OpeningCaptureOutcome, ScanError> {
    if !work.is_current() {
        return Ok(OpeningCaptureOutcome::Retired);
    }
    let capability = live_only_capability(
        &work.root_id,
        work.root_generation,
        work.authorized_unix_ms(observed_unix_ms),
    );
    let result = match &work.authority {
        JournalBaselineOpeningAuthority::ExistingRoot => {
            catalog.save_persistent_journal_capability(&capability)
        }
        JournalBaselineOpeningAuthority::FirstImport { capture, .. } => catalog
            .save_first_import_journal_capability(&capability, capture.scan_id(), || {
                capture.acquire_publication()
            }),
    };
    work.complete_publication(result)
}

pub(in crate::application::library_synchronization) fn capture_opening_boundary(
    work: JournalBaselineOpeningWork,
    session: &dyn PersistentChangeJournalSession,
    caller: &CallerClaim,
    observed_unix_ms: i64,
    catalog_session: &SqliteCatalogSession,
    queue_policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<OpeningCaptureOutcome, ScanError> {
    ensure_boundary_not_cancelled(cancelled, "opening")?;
    if !work.is_current() {
        return Ok(OpeningCaptureOutcome::Retired);
    }
    let registration = describe_production_persistent_journal_root(
        &work.root_id,
        work.root_generation.value(),
        &work.root_path,
    )
    .map_err(map_journal_operation_error)?;
    let boundary = probe_journal_boundary(session, caller, &registration)?;
    ensure_boundary_not_cancelled(cancelled, "opening")?;
    if !work.is_current() {
        return Ok(OpeningCaptureOutcome::Retired);
    }
    let mut catalog = catalog_session.open_in_lane(LibraryChangeLane::Journal)?;
    let result = match boundary {
        JournalBoundaryProbe::Supported(supported) => {
            let capture = match &work.authority {
                JournalBaselineOpeningAuthority::ExistingRoot => None,
                JournalBaselineOpeningAuthority::FirstImport { capture, .. } => Some(capture),
            };
            let publication = catalog.begin_journal_baseline_with_admission(
                &opening_request(&work, supported, observed_unix_ms),
                queue_policy,
                || {
                    capture
                        .map(FirstImportCaptureLease::acquire_publication)
                        .transpose()
                },
            );
            work.complete_publication(publication.map(|_| ()))
        }
        JournalBoundaryProbe::LiveOnly => {
            persist_unavailable_session(&mut catalog, &work, observed_unix_ms)
        }
    };
    drop(registration);
    result
}

impl JournalBaselineOpeningWork {
    fn is_current(&self) -> bool {
        match &self.authority {
            JournalBaselineOpeningAuthority::ExistingRoot => true,
            JournalBaselineOpeningAuthority::FirstImport { capture, .. } => capture.is_current(),
        }
    }

    fn complete_publication(
        &self,
        result: Result<(), ScanError>,
    ) -> Result<OpeningCaptureOutcome, ScanError> {
        match result {
            // Commit admission can precede a later control request. Keep that receipt authoritative.
            Ok(()) => Ok(OpeningCaptureOutcome::Recorded),
            Err(error)
                if error.code == "persistent_journal_first_import_inactive"
                    && !self.is_current() =>
            {
                Ok(OpeningCaptureOutcome::Retired)
            }
            Err(error) => Err(error),
        }
    }

    fn authorized_unix_ms(&self, observed_unix_ms: i64) -> i64 {
        match &self.authority {
            JournalBaselineOpeningAuthority::ExistingRoot => observed_unix_ms,
            JournalBaselineOpeningAuthority::FirstImport {
                started_unix_ms, ..
            } => observed_unix_ms.max(*started_unix_ms),
        }
    }
}

pub(super) fn opening_request(
    work: &JournalBaselineOpeningWork,
    boundary: SupportedJournalBoundary,
    observed_unix_ms: i64,
) -> PersistentJournalBaselineStartRequest {
    let SupportedJournalBoundary {
        volume,
        root_file_reference,
        journal_id,
        next_usn,
    } = boundary;
    let authorized_unix_ms = work.authorized_unix_ms(observed_unix_ms);
    let (run_id, authority_reason) = match &work.authority {
        JournalBaselineOpeningAuthority::ExistingRoot => (
            crate::application::scan_library::stable_id(
                "persistent-journal-baseline-v1",
                &format!(
                    "{}\0{}\0{}\0{}\0{}\0{}",
                    work.root_id,
                    work.root_generation.value(),
                    volume.volume_guid,
                    volume.volume_serial,
                    journal_id.value(),
                    next_usn.value(),
                ),
            ),
            LibraryRecoveryAuthorityReason::ExistingRootBaseline,
        ),
        JournalBaselineOpeningAuthority::FirstImport { capture, .. } => (
            capture.scan_id().to_owned(),
            LibraryRecoveryAuthorityReason::FirstImportBoundary,
        ),
    };
    PersistentJournalBaselineStartRequest {
        run_id,
        root_id: work.root_id.clone(),
        root_generation: work.root_generation,
        authority_reason,
        volume,
        root_file_reference,
        journal_id,
        opening_next_usn: next_usn,
        protocol_version: crate::journal_broker::PROTOCOL_VERSION,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        authorized_unix_ms,
    }
}
