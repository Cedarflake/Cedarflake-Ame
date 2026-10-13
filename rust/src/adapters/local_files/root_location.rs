use std::fs::{File, OpenOptions};
use std::io;
use std::mem::size_of;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::{Component, Path, PathBuf, Prefix};

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    ExtendedFileIdType, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_128, FILE_ID_DESCRIPTOR,
    FILE_ID_DESCRIPTOR_0, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OpenFileById,
};

use crate::domain::{FileIdentityEvidence, MetadataInventoryPlaceholderState, ScanError};

use super::{
    PublicationGuardedFileDiscovery, file_attribute_tag_info_from_handle,
    file_identity_from_handle, final_path_from_handle, metadata_placeholder_state_from_attributes,
    raw_file_id_info,
};

pub(crate) struct LocatedLibraryRoot {
    path: String,
    guard: PublicationGuardedFileDiscovery,
}

impl LocatedLibraryRoot {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn guard(&self) -> &PublicationGuardedFileDiscovery {
        &self.guard
    }
}

pub(crate) fn locate_library_root(
    registered_path: &str,
    identity: &FileIdentityEvidence,
) -> Result<Option<LocatedLibraryRoot>, ScanError> {
    let Some(hint_path) = local_volume_hint(Path::new(registered_path)) else {
        return Ok(None);
    };
    let Some((volume_serial, file_id)) = parse_identity(identity)? else {
        return Ok(None);
    };
    let candidate = match lookup_directory(&hint_path, volume_serial, file_id, identity) {
        Ok(candidate) => candidate,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
            ) =>
        {
            return Ok(None);
        }
        Err(error) => {
            return Err(ScanError::new(
                "root_location_lookup_failed",
                format!("Could not locate the retained directory identity: {error}"),
            ));
        }
    };
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
        &candidate.to_string_lossy(),
        identity,
    )?;
    let path = guard
        .discovery
        .canonical_root
        .to_string_lossy()
        .into_owned();
    Ok(Some(LocatedLibraryRoot { path, guard }))
}

fn local_volume_hint(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    let Component::Prefix(prefix) = components.next()? else {
        return None;
    };
    let (Prefix::Disk(letter) | Prefix::VerbatimDisk(letter)) = prefix.kind() else {
        return None;
    };
    if components.next()? != Component::RootDir {
        return None;
    }
    Some(PathBuf::from(format!("{}:\\", char::from(letter))))
}

fn parse_identity(identity: &FileIdentityEvidence) -> Result<Option<(u64, [u8; 16])>, ScanError> {
    if identity.scheme != "windows-file-id-128-v1" {
        return Ok(None);
    }
    let bytes = identity.value.as_bytes();
    if bytes.len() != 49
        || bytes[16] != b':'
        || bytes.iter().enumerate().any(|(index, byte)| {
            index != 16 && !byte.is_ascii_digit() && !(b'a'..=b'f').contains(byte)
        })
    {
        return Err(invalid_identity());
    }
    let volume = u64::from_str_radix(&identity.value[..16], 16).map_err(|_| invalid_identity())?;
    let file_id = u128::from_str_radix(&identity.value[17..], 16)
        .map_err(|_| invalid_identity())?
        .to_le_bytes();
    Ok(Some((volume, file_id)))
}

fn invalid_identity() -> ScanError {
    ScanError::new(
        "root_location_identity_invalid",
        "Directory recovery requires canonical complete Windows identity evidence",
    )
}

fn lookup_directory(
    hint_path: &Path,
    volume_serial: u64,
    file_id: [u8; 16],
    expected: &FileIdentityEvidence,
) -> io::Result<Option<PathBuf>> {
    let hint = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(hint_path)?;
    if !is_available_directory(&hint)?
        || raw_file_id_info(&hint)?.VolumeSerialNumber != volume_serial
    {
        return Ok(None);
    }
    let descriptor = FILE_ID_DESCRIPTOR {
        dwSize: u32::try_from(size_of::<FILE_ID_DESCRIPTOR>()).expect("descriptor size fits u32"),
        Type: ExtendedFileIdType,
        Anonymous: FILE_ID_DESCRIPTOR_0 {
            ExtendedFileId: FILE_ID_128 {
                Identifier: file_id,
            },
        },
    };
    // SAFETY: ADR 0024 admits this attribute-only lookup. The live hint and initialized extended
    // descriptor are borrowed only for this call; OpenFileById retains neither pointer.
    let raw = unsafe {
        OpenFileById(
            hint.as_raw_handle(),
            &raw const descriptor,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the successful call transferred exactly one owned handle. File closes it on every
    // return path, and no other owner is created from it.
    let directory = unsafe { File::from_raw_handle(raw) };
    if !is_available_directory(&directory)?
        || file_identity_from_handle(&directory)?.as_ref() != Some(expected)
    {
        return Ok(None);
    }
    final_path_from_handle(&directory).map(Some)
}

fn is_available_directory(file: &File) -> io::Result<bool> {
    let attributes = file_attribute_tag_info_from_handle(file)?.FileAttributes;
    Ok(attributes & FILE_ATTRIBUTE_DIRECTORY != 0
        && attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0
        && metadata_placeholder_state_from_attributes(attributes)
            == MetadataInventoryPlaceholderState::Available)
}

#[cfg(test)]
mod tests;
