use crate::domain::ScanError;
use crate::ports::file_operation_exports::NativeFilePreparation;
use crate::ports::file_operation_selection::FileOperationSelectionRepository;

use super::FileOperationSelectionSession;

pub(super) fn validate(
    selection: &dyn FileOperationSelectionRepository,
    native: &dyn NativeFilePreparation,
    session: &FileOperationSelectionSession,
) -> Result<(), ScanError> {
    let mut after = String::new();
    loop {
        session.check()?;
        let retained = selection.retained_page(&after)?;
        if retained.is_empty() {
            return Ok(());
        }
        for location in retained {
            session.check()?;
            super::candidate::inspect_source(&location, native).map_err(|error| {
                ScanError::new("file_operation_keep_unavailable", error.message)
            })?;
            after = location.location_id;
        }
    }
}
