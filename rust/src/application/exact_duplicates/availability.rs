use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::domain::{
    DuplicateLocationAvailability, ExpectedFileState, LibraryRootAvailability, ScanIssue,
};
use crate::ports::source_state::SourceStateProbe;

pub(super) fn inspect(
    probe: &impl SourceStateProbe,
    candidate: &FingerprintCandidate,
) -> DuplicateLocationAvailability {
    let root = probe.inspect_root(&candidate.root.root_path);
    match root.availability {
        LibraryRootAvailability::Offline => DuplicateLocationAvailability::Offline,
        LibraryRootAvailability::Missing | LibraryRootAvailability::Inaccessible => {
            DuplicateLocationAvailability::Unavailable
        }
        _ => {
            let location = &candidate.location;
            let expected = ExpectedFileState {
                absolute_path: location.absolute_path.clone(),
                file_size: location.file_size,
                modified_unix_ms: location.modified_unix_ms,
                file_identity: location.file_identity.clone(),
                source_revision: location.source_revision.clone(),
            };
            file_availability(probe.revalidate_file(&expected))
        }
    }
}

fn file_availability(result: Result<(), ScanIssue>) -> DuplicateLocationAvailability {
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

#[cfg(test)]
mod tests;
