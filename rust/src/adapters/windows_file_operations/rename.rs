use std::fs::File;
use std::mem::{offset_of, size_of};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_RENAME_INFO, FileRenameInfo, SetFileInformationByHandle,
};

pub(super) fn rename(source: &File, destination: &Path) -> std::io::Result<()> {
    let name: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let bytes = name
        .len()
        .checked_mul(size_of::<u16>())
        .and_then(|size| size.checked_add(offset_of!(FILE_RENAME_INFO, FileName)))
        .filter(|size| *size <= u32::MAX as usize)
        .ok_or_else(|| std::io::Error::other("The destination name is too long"))?;
    let words = bytes
        .max(size_of::<FILE_RENAME_INFO>())
        .div_ceil(size_of::<usize>());
    let mut buffer = vec![0usize; words];
    // SAFETY: ADR 0042; usize storage has the structure's pointer alignment and sufficient
    // capacity, including its flexible UTF-16 tail. Zero keeps ReplaceIfExists false.
    // The file handle and buffer remain live for the synchronous call.
    unsafe {
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).FileNameLength = ((name.len() - 1) * size_of::<u16>()) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!((*info).FileName).cast(),
            name.len(),
        );
        if SetFileInformationByHandle(
            source.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            bytes as u32,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
