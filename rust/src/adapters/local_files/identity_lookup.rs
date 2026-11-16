use std::fs::File;
use std::io;
use std::mem::size_of;
use std::os::windows::io::{AsRawHandle, FromRawHandle};

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    ExtendedFileIdType, FILE_ID_128, FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0,
    FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OpenFileById,
};

use crate::domain::{FileIdentityEvidence, ScanError};

pub(super) fn parse_identity(
    identity: &FileIdentityEvidence,
) -> Result<Option<(u64, [u8; 16])>, ScanError> {
    if identity.scheme != "windows-file-id-128-v1" {
        return Ok(None);
    }
    let invalid = || {
        ScanError::new(
            "source_file_identity_invalid",
            "Source lookup requires canonical complete Windows identity evidence",
        )
    };
    let bytes = identity.value.as_bytes();
    if bytes.len() != 49
        || bytes[16] != b':'
        || bytes.iter().enumerate().any(|(index, byte)| {
            index != 16 && !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte)
        })
    {
        return Err(invalid());
    }
    let volume = u64::from_str_radix(&identity.value[..16], 16).map_err(|_| invalid())?;
    let file_id = u128::from_str_radix(&identity.value[17..], 16)
        .map_err(|_| invalid())?
        .to_le_bytes();
    Ok(Some((volume, file_id)))
}

pub(super) fn open_by_id(hint: &File, file_id: [u8; 16], flags: u32) -> io::Result<File> {
    let descriptor = FILE_ID_DESCRIPTOR {
        dwSize: u32::try_from(size_of::<FILE_ID_DESCRIPTOR>()).expect("descriptor size fits u32"),
        Type: ExtendedFileIdType,
        Anonymous: FILE_ID_DESCRIPTOR_0 {
            ExtendedFileId: FILE_ID_128 {
                Identifier: file_id,
            },
        },
    };
    // SAFETY: ADR 0024 admits attribute-only identity lookup. Both the live volume hint and the
    // initialized extended descriptor outlive this synchronous call; no pointers are retained.
    let raw = unsafe {
        OpenFileById(
            hint.as_raw_handle(),
            &raw const descriptor,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            flags,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful call transfers one owned handle, closed by File on every exit path.
    Ok(unsafe { File::from_raw_handle(raw) })
}
