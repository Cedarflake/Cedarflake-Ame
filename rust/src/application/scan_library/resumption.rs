use crate::domain::{ScanError, ScanResumeCoverage};
use crate::ports::scan_admission::ScanAdmissionRepository;

#[cfg(windows)]
pub(super) fn load_resume_coverage(
    catalog: &impl ScanAdmissionRepository,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
) -> Result<Option<ScanResumeCoverage>, ScanError> {
    use std::path::Path;

    use crate::ports::journal_boundary::{
        JournalBoundaryProbe, JournalBoundaryReader, JournalBoundarySession, JournalBoundarySource,
    };

    let Some(baseline) = catalog.load_first_import_resume_baseline(scan_id, root_id)? else {
        return Ok(None);
    };
    let session = new_production_resume_boundary_source().open_root(
        root_id,
        baseline.root_generation,
        Path::new(root_path),
    )?;
    let result = match session.read_boundary() {
        Ok(JournalBoundaryProbe::Supported(boundary)) => ScanResumeCoverage::verify(
            baseline,
            &boundary.volume,
            &boundary.root_file_reference,
            boundary.journal_id,
            boundary.first_usn,
            boundary.next_usn,
        ),
        Ok(JournalBoundaryProbe::LiveOnly { .. }) => Ok(None),
        Err(error) => Err(error),
    };
    session.close()?;
    result
}

#[cfg(windows)]
fn new_production_resume_boundary_source()
-> impl crate::ports::journal_boundary::JournalBoundarySource<
    Root: crate::ports::journal_boundary::JournalBoundarySession,
> {
    crate::adapters::resume_boundary_source()
}

#[cfg(not(windows))]
pub(super) fn load_resume_coverage(
    _catalog: &impl ScanAdmissionRepository,
    _scan_id: &str,
    _root_id: &str,
    _root_path: &str,
) -> Result<Option<ScanResumeCoverage>, ScanError> {
    Ok(None)
}
