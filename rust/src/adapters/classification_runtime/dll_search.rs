use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::System::LibraryLoader::{
    AddDllDirectory, LOAD_LIBRARY_SEARCH_SYSTEM32, LOAD_LIBRARY_SEARCH_USER_DIRS,
    RemoveDllDirectory, SetDefaultDllDirectories,
};

use super::RuntimeError;

pub(super) struct RuntimeDllDirectory(*mut c_void);

impl RuntimeDllDirectory {
    pub fn install(directory: &Path) -> Result<Self, RuntimeError> {
        let wide: Vec<_> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_USER_DIRS)
        } == 0
        {
            return Err(RuntimeError::RuntimeUnavailable);
        }
        let cookie = unsafe { AddDllDirectory(wide.as_ptr()) };
        if cookie.is_null() {
            return Err(RuntimeError::RuntimeUnavailable);
        }
        Ok(Self(cookie))
    }
}

impl Drop for RuntimeDllDirectory {
    fn drop(&mut self) {
        unsafe { RemoveDllDirectory(self.0) };
    }
}
