use std::path::Path;

use crate::domain::ScanError;
use crate::ports::catalog_capacity::CatalogCapacity;

pub(crate) struct LocalCatalogCapacity;

impl CatalogCapacity for LocalCatalogCapacity {
    #[cfg(windows)]
    fn available_for(&self, path: &Path) -> Result<u64, ScanError> {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        let directory = path.parent().ok_or_else(|| {
            ScanError::new(
                "catalog_reclamation_directory_invalid",
                "The catalog does not have a parent directory",
            )
        })?;
        let wide = directory
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let mut available = 0_u64;
        // SAFETY: `wide` is a live NUL-terminated UTF-16 buffer for the duration of the call, and the
        // only non-null output points to a valid initialized `u64` owned by this stack frame.
        let succeeded = unsafe {
            GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if succeeded == 0 {
            return Err(ScanError::new(
                "catalog_reclamation_capacity_unavailable",
                format!(
                    "Could not inspect free space for catalog compaction: {}",
                    std::io::Error::last_os_error()
                ),
            ));
        }
        Ok(available)
    }

    #[cfg(all(not(windows), test))]
    fn available_for(&self, _path: &Path) -> Result<u64, ScanError> {
        Ok(u64::MAX)
    }

    #[cfg(all(not(windows), not(test)))]
    fn available_for(&self, _path: &Path) -> Result<u64, ScanError> {
        Err(ScanError::new(
            "catalog_reclamation_platform_unsupported",
            "Catalog compaction is currently supported only on Windows",
        ))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn capacity_probe_accepts_an_existing_parent_without_creating_the_catalog() {
        let directory = tempfile::tempdir().unwrap();
        let catalog = directory.path().join("catalog.sqlite3");
        LocalCatalogCapacity.available_for(&catalog).unwrap();
        assert!(!catalog.exists());
    }

    #[test]
    fn capacity_probe_rejects_a_path_without_a_parent() {
        let error = LocalCatalogCapacity
            .available_for(Path::new(""))
            .unwrap_err();
        assert_eq!(error.code, "catalog_reclamation_directory_invalid");
    }
}
