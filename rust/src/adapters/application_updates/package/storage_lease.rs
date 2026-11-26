use std::fs::{File, OpenOptions};
use std::mem::MaybeUninit;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, GetFileInformationByHandle,
};

use super::{ApplicationUpdateError, storage};

pub(super) fn open_partial(path: &Path) -> Result<(File, Vec<File>), ApplicationUpdateError> {
    let parent = path
        .parent()
        .ok_or_else(|| storage("Missing update directory"))?;
    let mut namespace = Vec::new();
    let ancestors: Vec<_> = parent.ancestors().collect();
    for directory in ancestors.into_iter().rev() {
        let handle = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(directory)
            .map_err(storage)?;
        let metadata = handle.metadata().map_err(storage)?;
        if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
            return Err(storage("Update directory is redirected"));
        }
        namespace.push(handle);
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(storage)?;
    let metadata = file.metadata().map_err(storage)?;
    if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
        return Err(storage("Partial update is redirected"));
    }
    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // The owned File retains a valid handle; Windows initializes the entire record on success.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) } == 0 {
        return Err(storage(std::io::Error::last_os_error()));
    }
    // The successful call above initialized this plain Win32 information structure.
    if unsafe { information.assume_init() }.nNumberOfLinks != 1 {
        return Err(storage("Partial update has another file entry"));
    }
    Ok((file, namespace))
}
