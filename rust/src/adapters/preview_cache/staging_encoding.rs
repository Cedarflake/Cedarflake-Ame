use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use image::{DynamicImage, ImageFormat};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

use crate::domain::{DiscoveredFile, ScanIssue};

use super::{TEMPORARY_SEQUENCE, preview_issue};

pub(super) struct EncodedPreview {
    pub(super) path: PathBuf,
    pub(super) byte_size: u64,
}

pub(super) fn encode(
    image: &DynamicImage,
    artifact_path: &Path,
    source: &DiscoveredFile,
) -> Result<EncodedPreview, ScanIssue> {
    let (path, file) = claim(artifact_path)
        .map_err(|error| preview_issue(source, "preview_write_failed", error))?;
    let mut writer = BufWriter::new(file);
    let encoded = (|| {
        image
            .write_to(&mut writer, ImageFormat::Jpeg)
            .map_err(|error| preview_issue(source, "preview_write_failed", error))?;
        writer
            .flush()
            .map_err(|error| preview_issue(source, "preview_write_failed", error))?;
        writer
            .get_ref()
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|error| preview_issue(source, "preview_size_unavailable", error))
    })();
    drop(writer);
    match encoded {
        Ok(byte_size) => Ok(EncodedPreview { path, byte_size }),
        Err(error) => {
            let _ = fs::remove_file(path);
            Err(error)
        }
    }
}

fn claim(artifact_path: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..16 {
        let path = artifact_path.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        #[cfg(all(test, windows))]
        super::staging_safety_tests::before_write(artifact_path, &path);
        let mut options = File::options();
        options.write(true).create_new(true);
        #[cfg(windows)]
        options.share_mode(FILE_SHARE_READ);
        // Encoding uses this exclusive new handle, never reopens a guessed pathname.
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "No exclusive preview staging name was available",
    ))
}
