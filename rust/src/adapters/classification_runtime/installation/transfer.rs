use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT,
};

use crate::domain::ScanError;

use super::super::manifest::Artifact;
use super::download::ModelDownload;
use super::{check_cancelled, io_error};

const TRANSFER_DEADLINE: Duration = Duration::from_secs(60 * 60);

struct PartialArtifact {
    file: File,
    path: PathBuf,
    received: u64,
    hasher: Sha256,
}

impl PartialArtifact {
    fn open(root: &Path, artifact: &Artifact, cancelled: &AtomicBool) -> Result<Self, ScanError> {
        let path = partial_path(root, artifact);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&path)
            .map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() || !super::staging::local_attributes(metadata.file_attributes()) {
            return Err(io_error(
                "Partial model storage is not a local regular file",
            ));
        }
        let mut partial = Self {
            file,
            path,
            received: 0,
            hasher: Sha256::new(),
        };
        if metadata.len() > artifact.length {
            partial.restart()?;
        }
        let mut buffer = [0; 64 * 1024];
        loop {
            check_cancelled(cancelled)?;
            let count = partial.file.read(&mut buffer).map_err(io_error)?;
            if count == 0 {
                break;
            }
            partial.hasher.update(&buffer[..count]);
            partial.received += count as u64;
        }
        if partial.received == artifact.length && !partial.matches(artifact) {
            partial.restart()?;
        }
        Ok(partial)
    }

    fn restart(&mut self) -> Result<(), ScanError> {
        self.file.set_len(0).map_err(io_error)?;
        self.file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        self.received = 0;
        self.hasher = Sha256::new();
        Ok(())
    }

    fn matches(&self, artifact: &Artifact) -> bool {
        self.received == artifact.length
            && format!("{:x}", self.hasher.clone().finalize()) == artifact.digest
    }

    fn publish(
        self,
        root: &Path,
        artifact: &Artifact,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError> {
        self.file.sync_all().map_err(io_error)?;
        drop(self.file);
        check_cancelled(cancelled)?;
        let destination = root.join(artifact.name);
        let temporary = tempfile::TempPath::try_from_path(self.path).map_err(io_error)?;
        match temporary.persist_noclobber(&destination) {
            Ok(()) => Ok(()),
            Err(error) => {
                let retained = error.path.keep().map_err(io_error)?;
                if error.error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(io_error(error.error));
                }
                // A destination created independently must pass the same pinned-artifact admission.
                drop(
                    super::super::artifacts::verify_file_cancellable(
                        &destination,
                        artifact,
                        cancelled,
                    )
                    .map_err(super::runtime_error)?,
                );
                std::fs::remove_file(retained).map_err(io_error)
            }
        }
    }
}

fn partial_path(root: &Path, artifact: &Artifact) -> PathBuf {
    root.join(format!(
        ".ame-model-{}.{}.partial",
        artifact.name, artifact.digest
    ))
}

pub(super) fn download_artifact(
    root: &Path,
    artifact: &Artifact,
    download: &mut dyn ModelDownload,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), ScanError> {
    let mut partial = PartialArtifact::open(root, artifact, cancelled)?;
    progress(partial.received);
    if partial.received == artifact.length {
        check_cancelled(cancelled)?;
        return partial.publish(root, artifact, cancelled);
    }
    let mut response = download.open(artifact, partial.received)?;
    check_cancelled(cancelled)?;
    if response.offset == 0 && partial.received > 0 {
        partial.restart()?;
    }
    if response.offset != partial.received {
        return Err(mismatch());
    }
    progress(partial.received);
    let mut buffer = [0; 64 * 1024];
    let started = Instant::now();
    loop {
        check_cancelled(cancelled)?;
        if started.elapsed() >= TRANSFER_DEADLINE {
            return Err(ScanError::new(
                "classification_download_timed_out",
                "The model download timed out; retry to continue installation",
            ));
        }
        let limit = buffer
            .len()
            .min((artifact.length - partial.received + 1) as usize);
        let result = response.reader.read(&mut buffer[..limit]);
        check_cancelled(cancelled)?;
        let count = result.map_err(|_| interrupted())?;
        if count == 0 {
            break;
        }
        if partial.received + count as u64 > artifact.length {
            partial.restart()?;
            return Err(mismatch());
        }
        partial.file.write_all(&buffer[..count]).map_err(io_error)?;
        partial.hasher.update(&buffer[..count]);
        partial.received += count as u64;
        progress(partial.received);
    }
    if partial.received != artifact.length {
        return Err(interrupted());
    }
    if !partial.matches(artifact) {
        partial.restart()?;
        return Err(mismatch());
    }
    check_cancelled(cancelled)?;
    partial.publish(root, artifact, cancelled)
}

fn interrupted() -> ScanError {
    ScanError::new(
        "classification_download_interrupted",
        "The model download was interrupted; retry to continue installation",
    )
}

fn mismatch() -> ScanError {
    ScanError::new(
        "classification_download_mismatch",
        "The downloaded model failed verification; retry to download this file again",
    )
}

#[cfg(test)]
mod tests;
