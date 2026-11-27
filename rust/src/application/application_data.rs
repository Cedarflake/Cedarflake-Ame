use crate::domain::ScanError;

pub(crate) fn preferences_directory() -> Result<String, ScanError> {
    Ok(crate::adapters::application_data::directories()?
        .preferences
        .to_string_lossy()
        .into_owned())
}
