use std::io;

use windows_sys::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_PARAMETER, ERROR_PATH_NOT_FOUND,
    ERROR_SHARING_VIOLATION,
};

use crate::domain::ScanError;

use super::super::io::native_error;

pub(super) fn identity_open_error(error: io::Error) -> Option<ScanError> {
    if matches!(
        error.raw_os_error().map(|code| code as u32),
        Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_INVALID_PARAMETER)
    ) {
        return None;
    }
    Some(metadata_error(
        "identity_journal_identity_unavailable",
        error,
    ))
}

pub(super) fn metadata_error(code: &str, error: io::Error) -> ScanError {
    #[cfg(test)]
    eprintln!(
        "[Ame identity journal] stage={code} os_error={:?}",
        error.raw_os_error()
    );
    let code = match error.raw_os_error().map(|value| value as u32) {
        Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND) => "identity_journal_identity_missing",
        Some(ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION) => {
            "identity_journal_identity_access_denied"
        }
        _ => code,
    };
    native_error(code, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharing_denial_is_unavailable_evidence_and_not_absence() {
        let failure = identity_open_error(io::Error::from_raw_os_error(32))
            .expect("sharing denial is retained");
        assert_eq!(failure.code, "identity_journal_identity_access_denied");
        assert_eq!(
            metadata_error("alias_open", io::Error::from_raw_os_error(32)).code,
            failure.code
        );
    }

    #[test]
    fn missing_identity_codes_do_not_suppress_other_metadata_failures() {
        for code in [2, 3, 87] {
            assert!(identity_open_error(io::Error::from_raw_os_error(code)).is_none());
        }
        assert_eq!(
            metadata_error("alias_open", io::Error::from_raw_os_error(87)).code,
            "alias_open"
        );
        assert_eq!(
            identity_open_error(io::Error::from_raw_os_error(21))
                .expect("unexpected failure is retained")
                .code,
            "identity_journal_identity_unavailable"
        );
    }
}
