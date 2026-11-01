use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::domain::ScanError;

use super::super::manifest::Artifact;
use super::{check_cancelled, io_error};

const TRANSFER_DEADLINE: Duration = Duration::from_secs(60 * 60);

pub(super) fn download_artifact(
    root: &Path,
    artifact: &Artifact,
    reader: &mut dyn Read,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), ScanError> {
    let mut temporary = tempfile::Builder::new()
        .prefix(".ame-model-")
        .suffix(".part")
        .rand_bytes(12)
        .tempfile_in(root)
        .map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut received = 0;
    let started = Instant::now();
    loop {
        check_cancelled(cancelled)?;
        if started.elapsed() >= TRANSFER_DEADLINE {
            return Err(ScanError::new(
                "classification_download_timed_out",
                "The model download timed out; retry to continue installation",
            ));
        }
        let limit = buffer.len().min((artifact.length - received + 1) as usize);
        let result = reader.read(&mut buffer[..limit]);
        check_cancelled(cancelled)?;
        let count = result.map_err(|_| {
            ScanError::new(
                "classification_download_interrupted",
                "The model download was interrupted; retry to continue installation",
            )
        })?;
        if count == 0 {
            break;
        }
        received += count as u64;
        if received > artifact.length {
            return Err(mismatch());
        }
        temporary.write_all(&buffer[..count]).map_err(io_error)?;
        hasher.update(&buffer[..count]);
        progress(received);
    }
    if received != artifact.length || format!("{:x}", hasher.finalize()) != artifact.digest {
        return Err(mismatch());
    }
    temporary.as_file().sync_all().map_err(io_error)?;
    check_cancelled(cancelled)?;
    // No replacement: another publisher's destination must pass the same held-file checks.
    match temporary.persist_noclobber(root.join(artifact.name)) {
        Ok(file) => drop(file),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io_error(error.error)),
    }
    Ok(())
}

fn mismatch() -> ScanError {
    ScanError::new(
        "classification_download_mismatch",
        "The downloaded model failed verification; retry to download this file again",
    )
}
