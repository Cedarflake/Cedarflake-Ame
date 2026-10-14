use crate::adapters::SqliteCatalog;
use crate::domain::{ScanError, ScanResumeCoverage};

#[cfg(windows)]
pub(super) fn load_resume_coverage(
    catalog: &SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
) -> Result<Option<ScanResumeCoverage>, ScanError> {
    use std::path::Path;

    use crate::application::library_synchronization::{
        JournalBoundaryProbe, probe_journal_boundary,
    };
    use crate::journal_broker::{
        PersistentChangeJournalConnection, describe_production_persistent_journal_root,
        production_persistent_journal_caller_claim,
    };

    let Some(baseline) = catalog.load_first_import_resume_baseline(scan_id, root_id)? else {
        return Ok(None);
    };
    let registration = describe_production_persistent_journal_root(
        root_id,
        baseline.root_generation.value(),
        Path::new(root_path),
    )
    .map_err(probe_error)?;
    let caller = production_persistent_journal_caller_claim().map_err(probe_error)?;
    let connection = crate::adapters::production_identity_journal_factory().connect();
    let result = match &connection {
        PersistentChangeJournalConnection::Connected(session) => {
            match probe_journal_boundary(session.as_ref(), &caller, &registration) {
                Ok(JournalBoundaryProbe::Supported(boundary)) => ScanResumeCoverage::verify(
                    baseline,
                    &boundary.volume,
                    &boundary.root_file_reference,
                    boundary.journal_id,
                    boundary.first_usn,
                    boundary.next_usn,
                ),
                Ok(JournalBoundaryProbe::LiveOnly) => Ok(None),
                Err(error) => Err(error),
            }
        }
        PersistentChangeJournalConnection::LiveOnly(_) => Ok(None),
    };
    connection.close().map_err(probe_error)?;
    result
}

#[cfg(windows)]
fn probe_error(error: crate::journal_broker::PersistentChangeJournalOperationError) -> ScanError {
    ScanError::new(
        "scan_resume_journal_unavailable",
        format!("The paused import's change coverage could not be checked: {error}"),
    )
}

#[cfg(not(windows))]
pub(super) fn load_resume_coverage(
    _catalog: &SqliteCatalog,
    _scan_id: &str,
    _root_id: &str,
    _root_path: &str,
) -> Result<Option<ScanResumeCoverage>, ScanError> {
    Ok(None)
}
