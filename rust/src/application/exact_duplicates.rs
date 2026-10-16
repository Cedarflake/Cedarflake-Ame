use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::exact_duplicates::{EXACT_DUPLICATE_PAGE_SIZE, ExactAnalysisRun};
use crate::domain::{
    DuplicateLocationAvailability, ExactAnalysisProgress, ExactDuplicateGroupCursor,
    ExactDuplicateGroupPage, ExactDuplicateLocation, ExactDuplicateLocationPage, LibraryChangeLane,
    LibraryRootAvailability, ScanError,
};
use crate::ports::exact_duplicates::ExactDuplicateRepository;

mod execution;

static ACTIVE_ANALYSIS: Mutex<Option<AnalysisFlight>> = Mutex::new(None);

struct AnalysisFlight {
    run_id: String,
    cancelled: Arc<AtomicBool>,
}

struct AnalysisAdmission(String);

impl Drop for AnalysisAdmission {
    fn drop(&mut self) {
        if let Ok(mut active) = ACTIVE_ANALYSIS.lock()
            && active
                .as_ref()
                .is_some_and(|flight| flight.run_id == self.0)
        {
            *active = None;
        }
    }
}

pub(crate) fn run_exact_analysis(
    run_id: String,
    mut emit: impl FnMut(ExactAnalysisProgress) -> bool,
) -> Result<(), ScanError> {
    validate_identifier(&run_id)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let admission = {
        let mut active = ACTIVE_ANALYSIS.lock().map_err(|_| unavailable())?;
        if active.is_some() {
            return Err(ScanError::new(
                "exact_analysis_busy",
                "A content analysis is already running",
            ));
        }
        *active = Some(AnalysisFlight {
            run_id: run_id.clone(),
            cancelled: Arc::clone(&cancelled),
        });
        AnalysisAdmission(run_id.clone())
    };
    let storage = super::storage_paths()?;
    let mut catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let run = ExactAnalysisRun {
        run_id,
        started_unix_ms: current_time_ms()?,
    };
    let result = execution::analyze(
        &mut catalog,
        &crate::adapters::GuardedExactContentReader,
        &run,
        &cancelled,
        &mut emit,
    );
    drop(admission);
    result
}

pub(crate) fn cancel_exact_analysis(run_id: &str) -> Result<(), ScanError> {
    validate_identifier(run_id)?;
    let active = ACTIVE_ANALYSIS.lock().map_err(|_| unavailable())?;
    if let Some(flight) = active.as_ref().filter(|flight| flight.run_id == run_id) {
        flight.cancelled.store(true, Ordering::Release);
    }
    Ok(())
}

pub(crate) fn load_exact_groups(
    after: Option<ExactDuplicateGroupCursor>,
) -> Result<ExactDuplicateGroupPage, ScanError> {
    if let Some(cursor) = &after {
        validate_digest(&cursor.digest)?;
    }
    let storage = super::storage_paths()?;
    let catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    catalog.load_duplicate_groups(after.as_ref())
}

pub(crate) fn load_exact_locations(
    digest: String,
    byte_length: u64,
    after_location_id: Option<String>,
) -> Result<ExactDuplicateLocationPage, ScanError> {
    validate_digest(&digest)?;
    if let Some(id) = &after_location_id {
        validate_identifier(id)?;
    }
    let storage = super::storage_paths()?;
    let catalog =
        super::catalog_session::open_catalog(&storage.catalog_path, LibraryChangeLane::Recovery)?;
    let mut candidates = catalog.load_duplicate_locations(
        &digest,
        byte_length,
        after_location_id.as_deref().unwrap_or(""),
    )?;
    drop(catalog);
    let next_location_id = if candidates.len() > EXACT_DUPLICATE_PAGE_SIZE {
        candidates.truncate(EXACT_DUPLICATE_PAGE_SIZE);
        candidates
            .last()
            .map(|candidate| candidate.location.location_id.clone())
    } else {
        None
    };
    let locations = candidates
        .into_iter()
        .map(|candidate| {
            let location = candidate.location;
            let root = crate::adapters::inspect_root_availability(&candidate.root.root_path);
            let availability = match root.availability {
                LibraryRootAvailability::Offline => DuplicateLocationAvailability::Offline,
                LibraryRootAvailability::Missing | LibraryRootAvailability::Inaccessible => {
                    DuplicateLocationAvailability::Unavailable
                }
                _ => {
                    let expected = crate::domain::ExpectedFileState {
                        absolute_path: location.absolute_path.clone(),
                        file_size: location.file_size,
                        modified_unix_ms: location.modified_unix_ms,
                        file_identity: location.file_identity.clone(),
                        source_revision: location.source_revision.clone(),
                    };
                    file_availability(crate::adapters::revalidate_file_state(&expected))
                }
            };
            ExactDuplicateLocation {
                asset_id: location.asset_id,
                location_id: location.location_id,
                root_id: location.root_id,
                display_path: location.display_path,
                source_generation: location.source_generation,
                availability,
            }
        })
        .collect();
    Ok(ExactDuplicateLocationPage {
        locations,
        next_location_id,
    })
}

fn file_availability(
    result: Result<(), crate::domain::ScanIssue>,
) -> DuplicateLocationAvailability {
    match result {
        Ok(()) => DuplicateLocationAvailability::Local,
        // Revalidation reports placeholder attributes before opening any content handle.
        Err(issue)
            if matches!(
                issue.code.as_str(),
                "cloud_placeholder_skipped" | "source_became_unavailable"
            ) =>
        {
            DuplicateLocationAvailability::CloudOnly
        }
        Err(_) => DuplicateLocationAvailability::Unavailable,
    }
}

fn validate_identifier(value: &str) -> Result<(), ScanError> {
    if value.is_empty() || value.len() > 512 || value.contains('\0') {
        return Err(ScanError::new(
            "exact_analysis_identifier_invalid",
            "Content analysis requires a bounded identifier",
        ));
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ScanError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ScanError::new(
            "exact_digest_invalid",
            "The content digest is not canonical",
        ));
    }
    Ok(())
}

fn current_time_ms() -> Result<i64, ScanError> {
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| unavailable())?;
    i64::try_from(value.as_millis()).map_err(|_| unavailable())
}

fn unavailable() -> ScanError {
    ScanError::new(
        "exact_analysis_unavailable",
        "Content analysis is temporarily unavailable",
    )
}

#[cfg(test)]
mod tests;
