use crate::domain::ScanError;

mod directories;

pub(crate) use directories::directories;

fn storage_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new(
        "application_data_unavailable",
        format!("Application data: {error}"),
    )
}
