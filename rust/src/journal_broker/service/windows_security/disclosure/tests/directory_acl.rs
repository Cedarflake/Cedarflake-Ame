use std::path::Path;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetKernelObjectSecurity, SetKernelObjectSecurity,
};
use windows_sys::Win32::Storage::FileSystem::{READ_CONTROL, WRITE_DAC};

use super::super::{OwnedFileHandle, open_directory_handle, reopen_directory_access};

pub(super) struct DirectoryAcl {
    handle: OwnedFileHandle,
    original: Vec<u8>,
}

impl DirectoryAcl {
    pub(super) fn deny_listing(path: &Path) -> Self {
        let path = path.to_str().unwrap().encode_utf16().collect::<Vec<_>>();
        let metadata = open_directory_handle(&path).expect("open generated directory");
        let handle = reopen_directory_access(&metadata, READ_CONTROL | WRITE_DAC)
            .expect("open generated directory DACL");
        let mut needed = 0;
        // SAFETY: the first call only queries the required descriptor size into a live output.
        unsafe {
            GetKernelObjectSecurity(
                handle.raw(),
                DACL_SECURITY_INFORMATION,
                null_mut(),
                0,
                &mut needed,
            );
        }
        assert!(needed > 0 && needed <= 64 * 1024);
        let mut original = vec![0; needed as usize];
        // SAFETY: the bounded buffer owns the complete self-relative descriptor returned here.
        assert_ne!(
            unsafe {
                GetKernelObjectSecurity(
                    handle.raw(),
                    DACL_SECURITY_INFORMATION,
                    original.as_mut_ptr().cast(),
                    needed,
                    &mut needed,
                )
            },
            0
        );
        let guard = Self { handle, original };
        let sddl = "D:(D;;0x1;;;WD)(A;;FA;;;WD)\0"
            .encode_utf16()
            .collect::<Vec<_>>();
        let mut descriptor = null_mut();
        // SAFETY: the fixed terminated SDDL denies only directory listing on this test object.
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    null_mut(),
                )
            },
            0
        );
        // SAFETY: the descriptor is live and valid, and only this held generated object's DACL changes.
        let applied = unsafe {
            SetKernelObjectSecurity(guard.handle.raw(), DACL_SECURITY_INFORMATION, descriptor)
        };
        // SAFETY: conversion returned this unique LocalAlloc allocation, no longer used after this call.
        unsafe { LocalFree(descriptor) };
        assert_ne!(applied, 0);
        guard
    }
}

impl Drop for DirectoryAcl {
    fn drop(&mut self) {
        // SAFETY: the original self-relative descriptor and WRITE_DAC handle remain live. This
        // restores the exact DACL before TempDir cleanup, including during assertion unwinding.
        assert_ne!(
            unsafe {
                SetKernelObjectSecurity(
                    self.handle.raw(),
                    DACL_SECURITY_INFORMATION,
                    self.original.as_mut_ptr().cast(),
                )
            },
            0
        );
    }
}
