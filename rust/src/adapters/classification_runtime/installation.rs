mod download;
mod staging;
mod transfer;

use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::adapters::local_files::PreviewCacheNamespace;
use crate::domain::ScanError;
use crate::domain::classification::installation::{
    ModelInstallationPhase, ModelInstallationProgress,
};

use super::RuntimeError;
use super::artifacts::{pin_directory, verify_file_cancellable};
use super::factory::runtime_error;
use super::manifest::{self, Artifact};
use download::{HttpsModelDownload, ModelDownload};

const PACKAGE_DIRECTORY: &str = "siglip2-b16-224-ba1f3b0843f2";
const MODEL_FILES: [&Artifact; 3] = [&manifest::IMAGE, &manifest::TEXT, &manifest::TOKENIZER];
const NOTICES: [(&str, &[u8]); 2] = [
    (
        "LICENSE-2.0.txt",
        include_bytes!("installation/LICENSE-2.0.txt"),
    ),
    ("NOTICE.txt", include_bytes!("installation/NOTICE.txt")),
];

pub(crate) struct VerifiedModelInstallation {
    pub models: PathBuf,
    pub runtime: PathBuf,
    _files: Vec<File>,
    _namespace: PreviewCacheNamespace,
    _lease: staging::StagingLease,
}

pub(crate) fn prepare(
    model_storage: &Path,
    runtime: &Path,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(ModelInstallationProgress),
) -> Result<VerifiedModelInstallation, ScanError> {
    prepare_with(
        &model_storage.join(PACKAGE_DIRECTORY),
        runtime,
        &MODEL_FILES,
        &[&manifest::RUNTIME, &manifest::PROVIDER],
        &mut HttpsModelDownload::new(),
        cancelled,
        progress,
    )
}

fn prepare_with(
    models: &Path,
    runtime: &Path,
    model_files: &[&Artifact],
    runtime_files: &[&Artifact],
    download: &mut dyn ModelDownload,
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(ModelInstallationProgress),
) -> Result<VerifiedModelInstallation, ScanError> {
    check_cancelled(cancelled)?;
    let total_bytes = model_files.iter().map(|artifact| artifact.length).sum();
    progress(ModelInstallationProgress {
        phase: ModelInstallationPhase::Verifying,
        completed_bytes: 0,
        total_bytes,
    });
    let mut files = pin_directory(runtime).map_err(runtime_error)?;
    for artifact in runtime_files {
        files.push(
            verify_file_cancellable(&runtime.join(artifact.name), artifact, cancelled)
                .map_err(runtime_error)?,
        );
    }
    let namespace = PreviewCacheNamespace::for_operation(models)?;
    let root = namespace
        .root()
        .ok_or_else(|| io_error("Model storage is unavailable"))?;
    let lease = staging::StagingLease::acquire(root)?;
    lease.retire_interrupted_files(cancelled)?;
    let mut completed_bytes = 0;
    for artifact in model_files {
        check_cancelled(cancelled)?;
        progress(ModelInstallationProgress {
            phase: ModelInstallationPhase::Verifying,
            completed_bytes,
            total_bytes,
        });
        let path = root.join(artifact.name);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                progress(ModelInstallationProgress {
                    phase: ModelInstallationPhase::Downloading,
                    completed_bytes,
                    total_bytes,
                });
                let response = download.open(artifact);
                check_cancelled(cancelled)?;
                let mut response = response?;
                transfer::download_artifact(
                    root,
                    artifact,
                    response.as_mut(),
                    cancelled,
                    &mut |received| {
                        progress(ModelInstallationProgress {
                            phase: ModelInstallationPhase::Downloading,
                            completed_bytes: completed_bytes + received,
                            total_bytes,
                        });
                    },
                )?;
            }
            Err(error) => return Err(io_error(error)),
        }
        files.push(verify_file_cancellable(&path, artifact, cancelled).map_err(runtime_error)?);
        completed_bytes += artifact.length;
    }
    for (name, contents) in NOTICES {
        install_notice(root, name, contents, cancelled, &mut files)?;
    }
    check_cancelled(cancelled)?;
    progress(ModelInstallationProgress {
        phase: ModelInstallationPhase::Prepared,
        completed_bytes,
        total_bytes,
    });
    Ok(VerifiedModelInstallation {
        models: models.to_path_buf(),
        runtime: runtime.to_path_buf(),
        _files: files,
        _namespace: namespace,
        _lease: lease,
    })
}

fn install_notice(
    root: &Path,
    name: &'static str,
    contents: &[u8],
    cancelled: &AtomicBool,
    files: &mut Vec<File>,
) -> Result<(), ScanError> {
    // Notice bytes are part of the application, not another network resource.
    let mut temporary = tempfile::Builder::new()
        .prefix(".ame-model-")
        .suffix(".part")
        .rand_bytes(12)
        .tempfile_in(root)
        .map_err(io_error)?;
    std::io::copy(&mut Cursor::new(contents), &mut temporary).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    check_cancelled(cancelled)?;
    match temporary.persist_noclobber(root.join(name)) {
        Ok(file) => drop(file),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io_error(error.error)),
    }
    let mut file =
        super::artifacts::open_local_artifact(&root.join(name)).map_err(runtime_error)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    (&mut file)
        .take(contents.len() as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes != contents {
        return Err(runtime_error(RuntimeError::ArtifactMismatch));
    }
    files.push(file);
    Ok(())
}

pub(crate) fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        Err(runtime_error(RuntimeError::Cancelled))
    } else {
        Ok(())
    }
}

fn io_error(error: impl std::fmt::Display) -> ScanError {
    ScanError::new(
        "classification_model_storage_unavailable",
        error.to_string(),
    )
}

#[cfg(test)]
mod tests;
