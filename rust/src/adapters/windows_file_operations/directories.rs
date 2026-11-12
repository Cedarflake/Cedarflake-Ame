use crate::domain::ScanError;
use crate::domain::file_operations::{FileOperationDirectory, FileOperationIssueKind};

use super::scope;

pub(super) fn create(
    root: &FileOperationDirectory,
    components: &[&str],
) -> Result<FileOperationDirectory, ScanError> {
    if components.is_empty() || components.len() > 2 {
        return Err(ScanError::new(
            "file_organization_name_invalid",
            "Organization destinations require one or two directory names",
        ));
    }
    for name in components {
        scope::validate_name(name)
            .map_err(|issue| ScanError::new("file_organization_name_invalid", issue.message))?;
    }
    let _lane = super::mutation_lane()
        .map_err(|issue| ScanError::new("file_operation_busy", issue.message))?;
    let mut directory = root.clone();
    let mut guard = scope::open_directory(root, FileOperationIssueKind::DestinationChanged)
        .map_err(|issue| ScanError::new("file_operation_destination_unavailable", issue.message))?;
    let mut ancestors = Vec::with_capacity(components.len());
    for name in components {
        match std::fs::create_dir(guard.operation_path(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(directory_error(error.to_string())),
        }
        let visible = std::path::Path::new(&directory.path).join(name);
        directory = scope::inspect_directory(&visible.to_string_lossy())
            .map_err(|issue| directory_error(issue.message))?;
        let child = scope::open_directory(&directory, FileOperationIssueKind::DestinationChanged)
            .map_err(|issue| directory_error(issue.message))?;
        // Retain each admitted namespace while opening the next child.
        ancestors.push(guard);
        guard = child;
    }
    Ok(directory)
}

fn directory_error(message: String) -> ScanError {
    ScanError::new("file_organization_directory_failed", message)
}
