use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT, NtCreateFile,
};
use windows_sys::Win32::Foundation::{HANDLE, OBJ_DONT_REPARSE, UNICODE_STRING};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_LIST_DIRECTORY, FILE_NAME_NORMALIZED, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

use super::{
    OwnedFileHandle, WindowsSecurityError, final_path, open_directory_handle, open_metadata_handle,
    reject_reparse_handle, strip_extended_dos_prefix,
};
use crate::journal_broker::wire::{
    NameComparisonSemantics, NormalizedWindowsPath, RootRelativeScope, normalize_backend_path,
    scope_under_root,
};

pub(super) fn probe_disclosable_scope(
    root_path: &str,
    normalized_root: &NormalizedWindowsPath,
    root_comparisons: &[NameComparisonSemantics],
    scope: &RootRelativeScope,
) -> Result<bool, WindowsSecurityError> {
    probe_disclosable_scope_with(
        root_path,
        scope,
        |path| {
            let handle = open_listable_directory(path)?;
            verify_handle_under_root(handle.raw(), normalized_root, root_comparisons)
        },
        |path| match open_metadata_handle(&path.encode_utf16().collect::<Vec<_>>()) {
            Ok(handle) => {
                verify_handle_under_root(handle.raw(), normalized_root, root_comparisons)?;
                Ok(CandidateProbe::Accessible)
            }
            Err(WindowsSecurityError::PathMissing) => Ok(CandidateProbe::Missing),
            Err(error) => Err(error),
        },
    )
}

fn open_listable_directory(path: &str) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let metadata = open_directory_handle(&path.encode_utf16().collect::<Vec<_>>())?;
    listable_directory(&metadata)
}

pub(super) fn listable_directory(
    directory: &OwnedFileHandle,
) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let handle = reopen_directory_access(directory, FILE_READ_ATTRIBUTES | FILE_LIST_DIRECTORY)?;
    reject_reparse_handle(handle.raw())?;
    Ok(handle)
}

fn reopen_directory_access(
    directory: &OwnedFileHandle,
    access: u32,
) -> Result<OwnedFileHandle, WindowsSecurityError> {
    let mut empty = [0_u16];
    let name = UNICODE_STRING {
        Length: 0,
        MaximumLength: 0,
        Buffer: empty.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>())
            .map_err(|_| WindowsSecurityError::RootRejected)?,
        RootDirectory: directory.raw(),
        ObjectName: &name,
        Attributes: OBJ_DONT_REPARSE,
        SecurityDescriptor: null(),
        SecurityQualityOfService: null(),
    };
    let mut raw = null_mut();
    let mut status = IO_STATUS_BLOCK::default();
    // SAFETY: the empty relative name reopens the held object under the impersonated caller.
    // DIRECTORY_FILE rejects regular files before list rights could become file-data access.
    // All native buffers stay live; no EA, backup intent, enumeration or source write is requested.
    let result = unsafe {
        NtCreateFile(
            &mut raw,
            access,
            &attributes,
            &mut status,
            null(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT,
            null(),
            0,
        )
    };
    if result < 0 {
        return Err(WindowsSecurityError::RootRejected);
    }
    OwnedFileHandle::new(raw)
}

fn verify_handle_under_root(
    handle: HANDLE,
    normalized_root: &NormalizedWindowsPath,
    root_comparisons: &[NameComparisonSemantics],
) -> Result<(), WindowsSecurityError> {
    let final_path = strip_extended_dos_prefix(&final_path(handle, FILE_NAME_NORMALIZED)?)?;
    let normalized =
        normalize_backend_path(&final_path).map_err(|_| WindowsSecurityError::RootRejected)?;
    if scope_under_root(&normalized, normalized_root, root_comparisons)
        .map_err(|_| WindowsSecurityError::RootRejected)?
        .is_none()
    {
        return Err(WindowsSecurityError::RootRejected);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateProbe {
    Accessible,
    Missing,
}

fn probe_disclosable_scope_with(
    root_path: &str,
    scope: &RootRelativeScope,
    mut probe_ancestor: impl FnMut(&str) -> Result<(), WindowsSecurityError>,
    mut probe_candidate: impl FnMut(&str) -> Result<CandidateProbe, WindowsSecurityError>,
) -> Result<bool, WindowsSecurityError> {
    probe_ancestor(root_path)?;
    let RootRelativeScope::Relative(relative) = scope else {
        return Ok(true);
    };
    let components: Vec<&str> = relative.split('/').collect();
    if components.is_empty() || components.iter().any(|component| component.is_empty()) {
        return Err(WindowsSecurityError::RootRejected);
    }
    let mut current = root_path.to_owned();
    for component in &components[..components.len().saturating_sub(1)] {
        current.push('\\');
        current.push_str(component);
        probe_ancestor(&current)?;
    }
    current.push('\\');
    current.push_str(
        components
            .last()
            .ok_or(WindowsSecurityError::RootRejected)?,
    );
    Ok(matches!(
        probe_candidate(&current)?,
        CandidateProbe::Accessible | CandidateProbe::Missing
    ))
}

#[cfg(test)]
mod tests;
