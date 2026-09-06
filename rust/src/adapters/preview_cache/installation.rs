use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, ReplaceFileW};

pub(super) struct InstalledPreview {
    pub released_previous: bool,
}

#[derive(Debug)]
pub(super) struct InstallationFailure {
    pub code: &'static str,
    pub message: String,
}

impl From<io::Error> for InstallationFailure {
    fn from(error: io::Error) -> Self {
        Self {
            code: "preview_publish_failed",
            message: error.to_string(),
        }
    }
}

pub(super) fn install_new(
    staged: &Path,
    target: &Path,
) -> Result<InstalledPreview, InstallationFailure> {
    move_if_absent(staged, target)?;
    Ok(InstalledPreview {
        released_previous: false,
    })
}

pub(super) fn replace(
    staged: &Path,
    target: &Path,
) -> Result<InstalledPreview, InstallationFailure> {
    let backup = claim_backup(target)?;
    replace_with(
        staged,
        target,
        &backup,
        &NativeInstallation::for_target(target),
    )
}

fn claim_backup(target: &Path) -> io::Result<PathBuf> {
    for _ in 0..16 {
        let backup = target.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            super::TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        match claim_backup_path(&backup) {
            Ok(()) => return Ok(backup),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "No exclusive preview replacement backup name was available",
    ))
}

fn claim_backup_path(backup: &Path) -> io::Result<()> {
    File::options().write(true).create_new(true).open(backup)?;
    Ok(())
}

struct NativeInstallation {
    #[cfg(test)]
    fault: Option<ReplacementFault>,
}

impl NativeInstallation {
    fn for_target(_target: &Path) -> Self {
        Self {
            #[cfg(test)]
            fault: REPLACEMENT_FAULTS
                .lock()
                .expect("replacement faults")
                .remove(_target),
        }
    }
}

impl NativeInstallation {
    fn replace(&self, staged: &Path, target: &Path, backup: &Path) -> io::Result<()> {
        #[cfg(test)]
        if let Some(fault) = &self.fault
            && fault.error != 0
        {
            if fault.error == 1177 {
                fs::rename(target, backup)?;
            }
            return Err(io::Error::from_raw_os_error(fault.error));
        }
        #[cfg(test)]
        if super::should_fail_atomic_replace(target) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected preview replacement failure",
            ));
        }
        replace_file(staged, target, Some(backup))
    }

    fn restore(&self, backup: &Path, target: &Path) -> io::Result<()> {
        #[cfg(test)]
        if self.fault.as_ref().is_some_and(|fault| fault.fail_restore) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected restoration failure",
            ));
        }
        move_if_absent(backup, target)
    }

    fn remove_backup(&self, backup: &Path) -> io::Result<()> {
        #[cfg(test)]
        if self
            .fault
            .as_ref()
            .is_some_and(|fault| fault.fail_backup_cleanup)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "injected backup cleanup failure",
            ));
        }
        fs::remove_file(backup)
    }
}

fn replace_with(
    staged: &Path,
    target: &Path,
    backup: &Path,
    operations: &NativeInstallation,
) -> Result<InstalledPreview, InstallationFailure> {
    match operations.replace(staged, target, backup) {
        Ok(()) => Ok(InstalledPreview {
            // A failed deletion retains a managed temporary and its already reserved old bytes.
            // Startup recovery and idle cleanup recognize the same cache temporary grammar.
            released_previous: match operations.remove_backup(backup) {
                Ok(()) => true,
                Err(error) => error.kind() == io::ErrorKind::NotFound,
            },
        }),
        Err(error) if error.raw_os_error() == Some(1177) => {
            if let Err(restore_error) = operations.restore(backup, target) {
                return Err(InstallationFailure {
                    code: "preview_replace_restore_failed",
                    message: format!(
                        "Preview replacement failed ({error}); restoring its managed backup also failed ({restore_error}). The previous preview is retained at {}",
                        backup.display()
                    ),
                });
            }
            Err(error.into())
        }
        Err(error) => {
            // With an explicit backup, 1176 and all other documented failures retain the target.
            // The exclusively claimed zero-byte marker is the only remaining owned backup.
            let _ = operations.remove_backup(backup);
            Err(error.into())
        }
    }
}

#[cfg(windows)]
fn replace_file(staged: &Path, target: &Path, backup: Option<&Path>) -> io::Result<()> {
    let staged = wide_path(staged);
    let target = wide_path(target);
    let backup = backup.map(wide_path);
    // SAFETY: all supplied UTF-16 paths are NUL terminated and live through the synchronous call.
    // Production paths are in one managed cache directory; the explicit backup is exclusively
    // owned. Flags are zero because ReplaceFileW does not support WRITE_THROUGH.
    let replaced = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            staged.as_ptr(),
            backup
                .as_ref()
                .map_or(std::ptr::null(), |path| path.as_ptr()),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn move_if_absent(source: &Path, target: &Path) -> io::Result<()> {
    let source = wide_path(source);
    let target = wide_path(target);
    // SAFETY: both UTF-16 buffers live through this synchronous call. No replace/copy flag is
    // supplied: installation and rollback cannot overwrite a target that appeared concurrently.
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn replace_file(staged: &Path, target: &Path, backup: Option<&Path>) -> io::Result<()> {
    if let Some(backup) = backup {
        fs::copy(target, backup)?;
    }
    fs::rename(staged, target)
}

#[cfg(not(windows))]
fn move_if_absent(source: &Path, target: &Path) -> io::Result<()> {
    fs::hard_link(source, target)?;
    fs::remove_file(source)
}

#[cfg(test)]
pub(super) fn replace_file_for_fixture(staged: &Path, target: &Path) -> io::Result<()> {
    replace_file(staged, target, None)
}

#[cfg(test)]
#[derive(Default)]
struct ReplacementFault {
    error: i32,
    fail_restore: bool,
    fail_backup_cleanup: bool,
}

#[cfg(test)]
static REPLACEMENT_FAULTS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<PathBuf, ReplacementFault>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::adapters::preview_cache::{
        LocalPreviewStore, PREVIEW_ALGORITHM, PREVIEW_ALGORITHM_ID, PREVIEW_ALGORITHM_VERSION,
        PREVIEW_ORIENTATION_CONTRACT, cache_inventory, is_managed_preview_cleanup_entry,
    };
    use crate::domain::{PreviewArtifact, PreviewMaterialization};

    const OLD: &[u8] = b"previous-preview-bytes";
    const NEW: &[u8] = b"new-preview";

    fn staged_fixture(
        root: &Path,
        fault: ReplacementFault,
    ) -> (LocalPreviewStore, PreviewMaterialization, PathBuf) {
        let key = "a".repeat(64);
        let target = root.join(format!("{PREVIEW_ALGORITHM}-{key}.jpg"));
        let staged = target.with_extension("1-999.tmp");
        fs::write(&target, OLD).expect("old preview");
        fs::write(&staged, NEW).expect("staged preview");
        let store = LocalPreviewStore::new(root.to_path_buf(), 1024).expect("preview store");
        REPLACEMENT_FAULTS
            .lock()
            .expect("replacement faults")
            .insert(target.clone(), fault);
        let materialization = PreviewMaterialization {
            artifact: PreviewArtifact {
                artifact_key: key,
                algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
                algorithm_version: PREVIEW_ALGORITHM_VERSION,
                orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
                size_bucket: 128,
                path: target.to_string_lossy().into_owned(),
                byte_size: NEW.len() as u64,
                encoded_width: 1,
                encoded_height: 1,
                width: 1,
                height: 1,
            },
            staged_path: Some(staged.to_string_lossy().into_owned()),
            reserved_bytes: NEW.len() as u64,
            replace_existing: true,
        };
        (store, materialization, target)
    }

    #[test]
    fn backup_1176_failure_retains_target_and_exact_cache_accounting() {
        let directory = tempdir().expect("cache");
        let (store, staged, target) = staged_fixture(
            directory.path(),
            ReplacementFault {
                error: 1176,
                ..ReplacementFault::default()
            },
        );

        let issue = store.commit(staged).expect_err("partial replacement");

        assert_eq!(issue.code, "preview_publish_failed");
        assert_eq!(fs::read(target).expect("retained target"), OLD);
        assert_eq!(store.used_bytes(), OLD.len() as u64);
        assert_eq!(
            cache_inventory(directory.path()).expect("inventory").0,
            OLD.len() as u64
        );
    }

    #[test]
    fn backup_1177_failure_restores_target_without_installing_staged_bytes() {
        let directory = tempdir().expect("cache");
        let (store, staged, target) = staged_fixture(
            directory.path(),
            ReplacementFault {
                error: 1177,
                ..ReplacementFault::default()
            },
        );

        let issue = store.commit(staged).expect_err("partial replacement");

        assert_eq!(issue.code, "preview_publish_failed");
        assert_eq!(fs::read(target).expect("restored target"), OLD);
        assert_eq!(store.used_bytes(), OLD.len() as u64);
        assert_eq!(
            cache_inventory(directory.path()).expect("inventory").0,
            OLD.len() as u64
        );
    }

    #[test]
    fn backup_1177_restore_failure_retains_recoverable_managed_bytes_and_accounting() {
        let directory = tempdir().expect("cache");
        let (store, staged, target) = staged_fixture(
            directory.path(),
            ReplacementFault {
                error: 1177,
                fail_restore: true,
                ..ReplacementFault::default()
            },
        );

        let issue = store.commit(staged).expect_err("restoration failure");

        assert_eq!(issue.code, "preview_replace_restore_failed");
        assert!(!target.exists());
        let entries = fs::read_dir(directory.path())
            .expect("cache entries")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert!(is_managed_preview_cleanup_entry(&entries[0]));
        assert_eq!(fs::read(&entries[0]).expect("recoverable old preview"), OLD);
        assert!(
            issue
                .message
                .contains(&entries[0].to_string_lossy().into_owned())
        );
        assert_eq!(store.used_bytes(), OLD.len() as u64);
        assert_eq!(
            cache_inventory(directory.path()).expect("inventory").0,
            OLD.len() as u64
        );
    }

    #[test]
    fn installed_preview_counts_a_backup_until_physical_cleanup_succeeds() {
        let directory = tempdir().expect("cache");
        let (store, staged, target) = staged_fixture(
            directory.path(),
            ReplacementFault {
                fail_backup_cleanup: true,
                ..ReplacementFault::default()
            },
        );

        store.commit(staged).expect("installed preview");

        assert_eq!(fs::read(target).expect("new target"), NEW);
        assert_eq!(store.used_bytes(), (OLD.len() + NEW.len()) as u64);
        assert_eq!(
            cache_inventory(directory.path()).expect("inventory").0,
            (OLD.len() + NEW.len()) as u64
        );
        assert_eq!(
            LocalPreviewStore::new(directory.path().to_path_buf(), 1024)
                .expect("reopened cache")
                .used_bytes(),
            store.used_bytes()
        );
    }

    #[test]
    fn backup_claim_and_restore_never_overwrite_an_existing_entry() {
        let directory = tempdir().expect("cache");
        let backup = directory.path().join("owned.tmp");
        let target = directory.path().join("target.jpg");
        fs::write(&backup, OLD).expect("existing backup name");
        fs::write(&target, NEW).expect("concurrent target");

        assert_eq!(
            claim_backup_path(&backup)
                .expect_err("exclusive claim")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(move_if_absent(&backup, &target).is_err());
        assert_eq!(fs::read(&backup).expect("retained backup"), OLD);
        assert_eq!(fs::read(&target).expect("retained concurrent target"), NEW);
    }
}
