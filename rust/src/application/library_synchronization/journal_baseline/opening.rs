use std::sync::atomic::AtomicBool;

use crate::application::scan_library::FirstImportCaptureLease;
use crate::domain::{
    LibraryChangeLane, LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason,
    PERSISTENT_JOURNAL_CONTRACT_VERSION, PersistentJournalBaselineStartRequest, ScanError,
};
use crate::ports::catalog_session::CatalogLaneSession;
use crate::ports::journal_baseline::JournalBaselineAdmission;
use crate::ports::journal_boundary::{JournalBoundaryReader, JournalBoundarySource};

use super::{
    JournalBaselineOpeningAuthority, JournalBaselineOpeningWork, JournalBoundaryProbe,
    SupportedJournalBoundary, ensure_boundary_not_cancelled, live_only_capability,
};

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::application::library_synchronization) enum OpeningCaptureOutcome {
    Recorded,
    Retired,
}

pub(in crate::application::library_synchronization) fn persist_unavailable_session(
    catalog: &mut impl JournalBaselineAdmission,
    work: &JournalBaselineOpeningWork,
    observed_unix_ms: i64,
    protocol_version: u16,
) -> Result<OpeningCaptureOutcome, ScanError> {
    if !work.is_current() {
        return Ok(OpeningCaptureOutcome::Retired);
    }
    let capability = live_only_capability(
        &work.root_id,
        work.root_generation,
        work.authorized_unix_ms(observed_unix_ms),
        protocol_version,
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

pub(in crate::application::library_synchronization) fn capture_opening_boundary<Session>(
    work: JournalBaselineOpeningWork,
    boundary_source: &impl JournalBoundarySource,
    observed_unix_ms: i64,
    catalog_session: &Session,
    queue_policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<OpeningCaptureOutcome, ScanError>
where
    Session: CatalogLaneSession,
    Session::Catalog: JournalBaselineAdmission,
{
    ensure_boundary_not_cancelled(cancelled, "opening")?;
    if !work.is_current() {
        return Ok(OpeningCaptureOutcome::Retired);
    }
    let registration =
        boundary_source.open_root(&work.root_id, work.root_generation, &work.root_path)?;
    let boundary = registration.read_boundary()?;
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
        JournalBoundaryProbe::LiveOnly { protocol_version } => {
            persist_unavailable_session(&mut catalog, &work, observed_unix_ms, protocol_version)
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
        protocol_version,
        volume,
        root_file_reference,
        journal_id,
        next_usn,
        ..
    } = boundary;
    let authorized_unix_ms = work.authorized_unix_ms(observed_unix_ms);
    let (run_id, authority_reason) = match &work.authority {
        JournalBaselineOpeningAuthority::ExistingRoot => (
            crate::domain::stable_id(
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
        protocol_version,
        contract_version: PERSISTENT_JOURNAL_CONTRACT_VERSION,
        authorized_unix_ms,
    }
}
