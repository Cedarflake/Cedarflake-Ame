use std::fs::{File, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use crate::domain::ScanError;

use super::{check_cancelled, io_error};

pub(super) struct StagingLease {
    root: PathBuf,
    _file: File,
}

impl StagingLease {
    pub fn acquire(root: &Path) -> Result<Self, ScanError> {
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(root.join(".ame-installation.lock"))
            .map_err(io_error)?;
        let metadata = lease.metadata().map_err(io_error)?;
        if !metadata.is_file()
            || metadata.len() != 0
            || !local_attributes(metadata.file_attributes())
        {
            return Err(io_error("Model installation ownership is unavailable"));
        }
        lease.try_lock().map_err(|_| {
            ScanError::new(
                "classification_installation_busy",
                "Another model installation owns this directory",
            )
        })?;
        Ok(Self {
            root: root.to_path_buf(),
            _file: lease,
        })
    }

    pub fn retire_interrupted_files(&self, cancelled: &AtomicBool) -> Result<(), ScanError> {
        let entries: Vec<_> = std::fs::read_dir(&self.root)
            .map_err(io_error)?
            .take(513)
            .collect::<Result<_, _>>()
            .map_err(io_error)?;
        if entries.len() > 512 {
            return Err(io_error(
                "The model package directory contains too many entries",
            ));
        }
        for entry in entries {
            check_cancelled(cancelled)?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(random) = name
                .strip_prefix(".ame-model-")
                .and_then(|name| name.strip_suffix(".part"))
            else {
                continue;
            };
            if random.len() != 12
                || !random
                    .bytes()
                    .all(|character| character.is_ascii_alphanumeric())
            {
                continue;
            }
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(io_error)?;
            if metadata.is_file() && local_attributes(metadata.file_attributes()) {
                std::fs::remove_file(entry.path()).map_err(io_error)?;
            }
        }
        Ok(())
    }
}

pub(super) fn local_attributes(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
            | FILE_ATTRIBUTE_REPARSE_POINT)
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_retired_installer_partials_are_removed_under_exclusive_ownership() {
        let directory = tempfile::tempdir().unwrap();
        let interrupted = directory.path().join(".ame-model-a1234567890B.part");
        let unrelated = directory.path().join(".ame-model-user.part");
        let destination = directory.path().join("vision_model_fp16.onnx");
        std::fs::write(&interrupted, b"incomplete").unwrap();
        std::fs::write(&unrelated, b"retain").unwrap();
        std::fs::write(&destination, b"retain").unwrap();
        let lease = StagingLease::acquire(directory.path()).unwrap();
        assert!(StagingLease::acquire(directory.path()).is_err());
        lease
            .retire_interrupted_files(&AtomicBool::new(false))
            .unwrap();
        assert!(!interrupted.exists());
        assert!(unrelated.is_file());
        assert!(destination.is_file());
        drop(lease);
        drop(StagingLease::acquire(directory.path()).unwrap());
    }
}
