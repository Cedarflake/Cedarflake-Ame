use std::path::{Path, PathBuf};

use crate::domain::ScanError;
use crate::ports::application_data::ApplicationDataDirectories;

use super::storage_error;

fn beneath(root: &Path) -> ApplicationDataDirectories {
    ApplicationDataDirectories {
        data: root.join("data"),
        config: root.join("config"),
        cache: root.join("cache"),
        preferences: root.join("preferences"),
    }
}

pub(crate) fn directories() -> Result<ApplicationDataDirectories, ScanError> {
    #[cfg(debug_assertions)]
    if let Some(root) = std::env::var_os("CEDARFLAKE_AME_TEST_STORAGE_ROOT") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err(storage_error("Test application storage must be absolute"));
        }
        return Ok(beneath(&root));
    }
    native_layout()
}

#[cfg(windows)]
fn native_layout() -> Result<ApplicationDataDirectories, ScanError> {
    use windows_sys::Win32::UI::Shell::FOLDERID_LocalAppData;

    let local = known_folder(&FOLDERID_LocalAppData)?;
    let root = local
        .join("Packages")
        .join(super::super::windows_package_identity::STORE_PACKAGE_FAMILY)
        .join("LocalState")
        .join("Ame");
    Ok(beneath(&root))
}

#[cfg(windows)]
fn known_folder(id: &windows_sys::core::GUID) -> Result<PathBuf, ScanError> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{KF_FLAG_NO_PACKAGE_REDIRECTION, SHGetKnownFolderPath};

    let mut raw = std::ptr::null_mut();
    // SAFETY: ADR 0045. The fixed identifier and output slot are valid for the call; the
    // returned allocation is read within the platform path bound and then freed exactly once.
    let status = unsafe {
        SHGetKnownFolderPath(
            id,
            KF_FLAG_NO_PACKAGE_REDIRECTION as u32,
            std::ptr::null_mut(),
            &mut raw,
        )
    };
    if status < 0 || raw.is_null() {
        if !raw.is_null() {
            // SAFETY: this is the allocation returned by the failed known-folder call.
            unsafe { CoTaskMemFree(raw.cast()) };
        }
        return Err(storage_error(format!(
            "Cannot resolve the Windows data folder ({status})"
        )));
    }
    let result = (|| {
        for length in 0..32768 {
            // SAFETY: Windows returns a terminated path of at most 32767 UTF-16 code units.
            if unsafe { *raw.add(length) } == 0 {
                // SAFETY: the preceding terminator search bounds this initialized slice.
                let value = unsafe { std::slice::from_raw_parts(raw, length) };
                return Ok(PathBuf::from(OsString::from_wide(value)));
            }
        }
        Err(storage_error(
            "The Windows data folder exceeds the path bound",
        ))
    })();
    // SAFETY: the copied path owns its data; raw is not used after this release.
    unsafe { CoTaskMemFree(raw.cast()) };
    result
}

#[cfg(not(windows))]
fn native_layout() -> Result<ApplicationDataDirectories, ScanError> {
    let dirs = directories::ProjectDirs::from("com", "Cedarflake", "Ame")
        .ok_or_else(|| storage_error("The operating system did not provide application storage"))?;
    Ok(ApplicationDataDirectories {
        data: dirs.data_local_dir().to_path_buf(),
        config: dirs.config_dir().to_path_buf(),
        cache: dirs.cache_dir().to_path_buf(),
        preferences: dirs.config_dir().join("preferences"),
    })
}
