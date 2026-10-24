use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::domain::ScanError;

use super::super::{error, io_error};

pub(super) fn open(path: &Path) -> Result<File, ScanError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_NO_RECALL, FILE_SHARE_READ};
        options
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_NO_RECALL);
    }
    let file = options.open(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err(error("input_invalid", "Choose a regular backup file"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        require_resident(metadata.file_attributes())?;
    }
    Ok(file)
}

#[cfg(windows)]
fn require_resident(attributes: u32) -> Result<(), ScanError> {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };
    if attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
            | FILE_ATTRIBUTE_RECALL_ON_OPEN)
        != 0
    {
        Err(error(
            "input_not_local",
            "Make the backup available locally before restoring it",
        ))
    } else {
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        FILE_ATTRIBUTE_RECALL_ON_OPEN, FILE_ATTRIBUTE_REPARSE_POINT,
    };

    #[test]
    fn organization_archive_input_rejects_recall_but_accepts_resident_reparse_files() {
        for flag in [
            FILE_ATTRIBUTE_OFFLINE,
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        ] {
            assert_eq!(
                require_resident(flag | FILE_ATTRIBUTE_REPARSE_POINT)
                    .unwrap_err()
                    .code,
                "organization_archive_input_not_local"
            );
        }
        require_resident(FILE_ATTRIBUTE_NORMAL).unwrap();
        require_resident(FILE_ATTRIBUTE_REPARSE_POINT).unwrap();
    }
}
