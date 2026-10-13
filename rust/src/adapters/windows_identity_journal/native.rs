use std::collections::BTreeSet;
use std::fs::File;
use std::io;
use std::mem::size_of;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::ptr::{null, null_mut};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use windows_sys::Win32::Foundation::GENERIC_READ;
use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_INVALID_PARAMETER, ERROR_PATH_NOT_FOUND, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ExtendedFileIdType, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_128, FILE_ID_DESCRIPTOR,
    FILE_ID_DESCRIPTOR_0, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_STANDARD_INFO, FileIdInfo, FileStandardInfo, FindClose,
    FindFirstFileNameW, FindNextFileNameW, GetFileInformationByHandleEx, GetFinalPathNameByHandleW,
    OPEN_EXISTING, OpenFileById, VOLUME_NAME_GUID,
};
use windows_sys::Win32::System::Ioctl::FSCTL_QUERY_USN_JOURNAL;

use crate::adapters::PublicationGuardedFileDiscovery;
use crate::domain::{
    FileIdentityEvidence, JournalFileReference, PersistentJournalVolumeIdentity, ScanError,
};
use crate::journal_broker::RootAuthorization;
use crate::ports::IdentityJournalLocation;

use super::io::{control, native_error};

const METADATA_FLAGS: u32 =
    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT;
const MAX_PATH_UNITS: usize = 32_768;
const MAX_LINKS: usize = 64;

#[cfg(test)]
mod tests;

pub(super) struct NativeRoot {
    pub(super) authorization: RootAuthorization,
    pub(super) volume: PersistentJournalVolumeIdentity,
    pub(super) volume_file: File,
    root_file: File,
    root_guid_path: String,
    configured_path: String,
    identity: FileIdentityEvidence,
}

impl NativeRoot {
    pub(super) fn open(authorization: &RootAuthorization) -> Result<Self, ScanError> {
        let configured_path =
            String::from_utf16(&authorization.canonical_root_utf16).map_err(|_| invalid())?;
        let root_file = open_path(&configured_path, FILE_READ_ATTRIBUTES)?;
        let info = file_info(&root_file)?;
        if info.FileId.Identifier.as_slice() != authorization.root_identity.as_slice() {
            return Err(invalid());
        }
        let root_guid_path = final_path(&root_file)?;
        let volume_path = format!("{}\\", authorization.volume_id.trim_end_matches('\\'));
        if !root_guid_path.starts_with(&volume_path) {
            return Err(invalid());
        }
        let identity = identity(&info);
        let _guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &configured_path,
            &identity,
        )?;
        let volume_file = open_path(&volume_path, GENERIC_READ)?;
        let volume = PersistentJournalVolumeIdentity {
            volume_guid: authorization.volume_id.clone(),
            volume_serial: info.VolumeSerialNumber,
        };
        if file_info(&volume_file)?.VolumeSerialNumber != volume.volume_serial {
            return Err(invalid());
        }
        Ok(Self {
            authorization: authorization.clone(),
            volume,
            volume_file,
            root_file,
            root_guid_path,
            configured_path,
            identity,
        })
    }

    pub(super) fn guard(&self) -> Result<PublicationGuardedFileDiscovery, ScanError> {
        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
            &self.configured_path,
            &self.identity,
        )?;
        if identity(&file_info(&self.root_file)?) != self.identity
            || final_path(&self.root_file)? != self.root_guid_path
        {
            return Err(invalid());
        }
        Ok(guard)
    }

    pub(super) fn query(
        &self,
        cancelled: &AtomicBool,
        deadline: Instant,
    ) -> Result<(u64, i64, i64), ScanError> {
        let _guard = self.guard()?;
        let mut data = [0_u8; 128];
        let count = control(
            &self.volume_file,
            FSCTL_QUERY_USN_JOURNAL,
            &[],
            &mut data,
            cancelled,
            deadline,
        )?;
        if count < 56 {
            return Err(invalid());
        }
        let journal = u64::from_le_bytes(data[0..8].try_into().map_err(|_| invalid())?);
        let first = i64::from_le_bytes(data[8..16].try_into().map_err(|_| invalid())?);
        let next = i64::from_le_bytes(data[16..24].try_into().map_err(|_| invalid())?);
        if journal == 0 || first < 0 || next < first {
            return Err(invalid());
        }
        Ok((journal, first, next))
    }

    pub(super) fn locations(
        &self,
        reference: &JournalFileReference,
    ) -> Result<Option<Vec<IdentityJournalLocation>>, ScanError> {
        let mut identifier = [0_u8; 16];
        identifier[..reference.as_bytes().len()].copy_from_slice(reference.as_bytes());
        let descriptor = FILE_ID_DESCRIPTOR {
            dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: ExtendedFileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                ExtendedFileId: FILE_ID_128 {
                    Identifier: identifier,
                },
            },
        };
        // SAFETY: the descriptor and volume handle stay live; this metadata-only open cannot
        // read media or recall placeholder content. The resulting handle has one owner.
        let raw = unsafe {
            OpenFileById(
                self.volume_file.as_raw_handle() as HANDLE,
                &descriptor,
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                METADATA_FLAGS,
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            let error = io::Error::last_os_error();
            if error.raw_os_error()
                == Some(windows_sys::Win32::Foundation::ERROR_ACCESS_DENIED as i32)
            {
                return Err(native_error(
                    "identity_journal_identity_access_denied",
                    error,
                ));
            }
            if matches!(
                error.raw_os_error().map(|code| code as u32),
                Some(ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_INVALID_PARAMETER)
            ) {
                return Ok(None);
            }
            return Err(native_error("identity_journal_identity_unavailable", error));
        }
        // SAFETY: OpenFileById returned a valid uniquely owned file handle.
        let file = unsafe { File::from_raw_handle(raw) };
        let info = file_info(&file)?;
        if info.FileId.Identifier != identifier
            || info.VolumeSerialNumber != self.volume.volume_serial
        {
            return Err(invalid());
        }
        let standard = standard_info(&file)?;
        if standard.DeletePending || standard.NumberOfLinks == 0 {
            return Ok(None);
        }
        let path = final_path(&file)?;
        let is_directory = attributes(&file)? & FILE_ATTRIBUTE_DIRECTORY != 0;
        let candidates = if is_directory || standard.NumberOfLinks == 1 {
            vec![path]
        } else {
            link_paths(&path, &self.volume.volume_guid)?
        };
        let mut locations = Vec::new();
        for candidate in candidates {
            if candidate != self.root_guid_path
                && !candidate
                    .starts_with(&format!("{}\\", self.root_guid_path.trim_end_matches('\\')))
            {
                continue;
            }
            let alias = open_path(&candidate, FILE_READ_ATTRIBUTES)?;
            if identity(&file_info(&alias)?) != identity(&info) {
                return Err(invalid());
            }
            let resolved = final_path(&alias)?;
            let relative = if resolved == self.root_guid_path {
                Some(String::new())
            } else {
                resolved
                    .strip_prefix(&format!("{}\\", self.root_guid_path.trim_end_matches('\\')))
                    .map(|relative| relative.replace('\\', "/"))
            };
            if let Some(relative_path) = relative {
                locations.push(IdentityJournalLocation {
                    identity: identity(&info),
                    relative_path,
                    is_directory,
                });
            }
        }
        Ok(Some(locations))
    }
}

fn open_path(path: &str, access: u32) -> Result<File, ScanError> {
    let path = terminated(path)?;
    // SAFETY: the bounded terminated path stays live; only read/attribute access is admitted.
    let raw = unsafe {
        CreateFileW(
            path.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            METADATA_FLAGS,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(metadata_error(
            "identity_journal_path_unavailable",
            io::Error::last_os_error(),
        ));
    }
    // SAFETY: CreateFileW succeeded and this value owns its handle.
    Ok(unsafe { File::from_raw_handle(raw) })
}

fn file_info(file: &File) -> Result<FILE_ID_INFO, ScanError> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: the exact SDK structure and its live writable storage match FileIdInfo.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(metadata_error(
            "identity_journal_file_identity",
            io::Error::last_os_error(),
        ));
    }
    Ok(info)
}

fn attributes(file: &File) -> Result<u32, ScanError> {
    use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_TAG_INFO, FileAttributeTagInfo};
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: the exact SDK structure and its writable storage match FileAttributeTagInfo.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    } == 0
    {
        return Err(metadata_error(
            "identity_journal_file_attributes",
            io::Error::last_os_error(),
        ));
    }
    Ok(info.FileAttributes)
}

fn standard_info(file: &File) -> Result<FILE_STANDARD_INFO, ScanError> {
    let mut info = FILE_STANDARD_INFO::default();
    // SAFETY: the exact SDK structure and its writable storage match FileStandardInfo.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileStandardInfo,
            (&mut info as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )
    } == 0
    {
        return Err(metadata_error(
            "identity_journal_file_standard_info",
            io::Error::last_os_error(),
        ));
    }
    Ok(info)
}

fn identity(info: &FILE_ID_INFO) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!(
            "{:016x}:{:032x}",
            info.VolumeSerialNumber,
            u128::from_le_bytes(info.FileId.Identifier)
        ),
    }
}

fn final_path(file: &File) -> Result<String, ScanError> {
    let mut path = vec![0_u16; MAX_PATH_UNITS];
    // SAFETY: the file remains live and the UTF-16 buffer has the declared capacity.
    let count = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle() as HANDLE,
            path.as_mut_ptr(),
            path.len() as u32,
            VOLUME_NAME_GUID,
        )
    } as usize;
    if count == 0 || count >= path.len() {
        return Err(metadata_error(
            "identity_journal_final_path",
            io::Error::last_os_error(),
        ));
    }
    String::from_utf16(&path[..count]).map_err(|_| invalid())
}

struct LinkSearch(HANDLE);
impl Drop for LinkSearch {
    fn drop(&mut self) {
        // SAFETY: this owner retains the successful FindFirstFileNameW search handle.
        let _ = unsafe { FindClose(self.0) };
    }
}

fn link_paths(path: &str, volume: &str) -> Result<Vec<String>, ScanError> {
    let path = terminated(path)?;
    let mut name = vec![0_u16; MAX_PATH_UNITS];
    let mut count = name.len() as u32;
    // SAFETY: all buffers are bounded and live; this enumerates names only, never file data.
    let raw = unsafe { FindFirstFileNameW(path.as_ptr(), 0, &mut count, name.as_mut_ptr()) };
    if raw == INVALID_HANDLE_VALUE {
        let error = io::Error::last_os_error();
        let code = if error.raw_os_error()
            == Some(windows_sys::Win32::Foundation::ERROR_NOT_SUPPORTED as i32)
        {
            "identity_journal_hardlinks_not_supported"
        } else {
            "identity_journal_hardlinks"
        };
        return Err(metadata_error(code, error));
    }
    let _search = LinkSearch(raw);
    let mut paths = BTreeSet::new();
    loop {
        let length = name
            .iter()
            .position(|unit| *unit == 0)
            .ok_or_else(invalid)?;
        let relative = String::from_utf16(&name[..length]).map_err(|_| invalid())?;
        if !relative.starts_with('\\') || paths.len() >= MAX_LINKS {
            return Err(invalid());
        }
        paths.insert(format!("{}{relative}", volume.trim_end_matches('\\')));
        name.fill(0);
        count = name.len() as u32;
        // SAFETY: the search owner and output buffer remain live at their declared capacities.
        if unsafe { FindNextFileNameW(raw, &mut count, name.as_mut_ptr()) } == 0 {
            if io::Error::last_os_error().raw_os_error()
                == Some(windows_sys::Win32::Foundation::ERROR_HANDLE_EOF as i32)
            {
                break;
            }
            return Err(metadata_error(
                "identity_journal_hardlink_next",
                io::Error::last_os_error(),
            ));
        }
    }
    Ok(paths.into_iter().collect())
}

fn metadata_error(code: &str, error: io::Error) -> ScanError {
    let code = match error.raw_os_error() {
        Some(2 | 3) => "identity_journal_identity_missing",
        Some(5) => "identity_journal_identity_access_denied",
        _ => code,
    };
    native_error(code, error)
}

fn terminated(path: &str) -> Result<Vec<u16>, ScanError> {
    let mut units: Vec<u16> = path.encode_utf16().collect();
    if units.is_empty() || units.len() >= MAX_PATH_UNITS || units.contains(&0) {
        return Err(invalid());
    }
    units.push(0);
    Ok(units)
}

fn invalid() -> ScanError {
    ScanError::new(
        "identity_journal_identity_invalid",
        "The journal identity or contained metadata path could not be verified",
    )
}
