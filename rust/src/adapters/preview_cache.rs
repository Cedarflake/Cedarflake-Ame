use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};

use blake3::Hasher;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

use crate::domain::{
    DiscoveredFile, ImageOrientation, PreviewArtifact, PreviewMaterialization, ScanIssue,
};
use crate::ports::PreviewStore;

use super::image_orientation::{apply_image_orientation, from_image_orientation};
use super::jpeg_preview::decode_scaled_jpeg;

mod installation;

struct PreviewDimensions {
    source_width: u32,
    source_height: u32,
    encoded: Option<(u32, u32)>,
}

pub(crate) const PREVIEW_CACHE_VERSION: &str = "ame-jpeg-thumbnail-v3-source-revision";
const PREVIEW_ALGORITHM: &str = PREVIEW_CACHE_VERSION;
const OBSOLETE_PREVIEW_ALGORITHM_V2: &str = "ame-jpeg-thumbnail-v2-orientation";
pub(crate) const PREVIEW_ALGORITHM_ID: &str = "ame-jpeg-thumbnail";
pub(crate) const PREVIEW_ALGORITHM_VERSION: u32 = 3;
pub(crate) const PREVIEW_ORIENTATION_CONTRACT: &str = "exif-display-v1";
const MAX_PREVIEW_EDGE: u32 = 1024;
const PREVIEW_SIZE_BUCKETS: [u32; 4] = [128, 256, 512, MAX_PREVIEW_EDGE];
const MAX_SOURCE_DIMENSION: u32 = 100_000;
const MAX_DECODER_ALLOCATION: u64 = 256 * 1024 * 1024;
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
static FAIL_ATOMIC_REPLACE_TARGETS: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(test)]
fn should_fail_atomic_replace(artifact_path: &Path) -> bool {
    FAIL_ATOMIC_REPLACE_TARGETS
        .lock()
        .expect("preview replacement failure targets")
        .remove(artifact_path)
}

#[cfg(test)]
pub(crate) fn fail_next_atomic_replace_for_test(artifact_path: &Path) {
    FAIL_ATOMIC_REPLACE_TARGETS
        .lock()
        .expect("preview replacement failure targets")
        .insert(artifact_path.to_path_buf());
}

pub struct LocalPreviewStore {
    root: PathBuf,
    budget_bytes: u64,
    used_bytes: AtomicU64,
    rejected_reservation_bytes: AtomicU64,
    install_lock: Mutex<()>,
    staging_targets: Mutex<HashSet<PathBuf>>,
    staging_changed: Condvar,
}

impl LocalPreviewStore {
    pub fn new(root: PathBuf, budget_bytes: u64) -> Result<Self, ScanIssue> {
        fs::create_dir_all(&root).map_err(|error| ScanIssue {
            path: Some(root.to_string_lossy().into_owned()),
            code: "preview_cache_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let (used_bytes, _) = cache_inventory(&root)?;
        Ok(Self {
            root,
            budget_bytes,
            used_bytes: AtomicU64::new(used_bytes),
            rejected_reservation_bytes: AtomicU64::new(0),
            install_lock: Mutex::new(()),
            staging_targets: Mutex::new(HashSet::new()),
            staging_changed: Condvar::new(),
        })
    }

    fn artifact_path(&self, file: &DiscoveredFile, preview_edge: u32) -> PathBuf {
        let artifact_key = self.artifact_key(file, preview_edge);
        self.root
            .join(format!("{PREVIEW_ALGORITHM}-{artifact_key}.jpg"))
    }

    fn artifact_key(&self, file: &DiscoveredFile, preview_edge: u32) -> String {
        preview_cache_key(PREVIEW_ALGORITHM, file, preview_edge)
            .to_hex()
            .to_string()
    }

    pub(crate) fn commit(
        &self,
        mut materialization: PreviewMaterialization,
    ) -> Result<PreviewArtifact, ScanIssue> {
        let Some(staged_path) = materialization.staged_path.take() else {
            return Ok(materialization.artifact);
        };
        let staged_path = PathBuf::from(staged_path);
        let artifact_path = PathBuf::from(&materialization.artifact.path);
        let _staging_completion = PreviewStagingCompletion {
            store: self,
            artifact_path: artifact_path.clone(),
        };
        if staged_path.parent() != Some(self.root.as_path())
            || artifact_path.parent() != Some(self.root.as_path())
            || !is_current_preview_temporary(&staged_path)
            || !is_current_preview_artifact(&artifact_path.to_string_lossy())
        {
            return Err(ScanIssue {
                path: Some(staged_path.to_string_lossy().into_owned()),
                code: "preview_publish_path_invalid".to_owned(),
                message: "The staged preview is outside the active preview cache".to_owned(),
            });
        }

        if !materialization.replace_existing
            && let Some((encoded_width, encoded_height)) =
                cached_artifact_dimensions(&artifact_path, materialization.artifact.size_bucket)
        {
            self.remove_staged_file(&staged_path, materialization.reserved_bytes)?;
            materialization.artifact.byte_size = artifact_path
                .metadata()
                .map_err(|metadata_error| ScanIssue {
                    path: Some(artifact_path.to_string_lossy().into_owned()),
                    code: "preview_size_unavailable".to_owned(),
                    message: metadata_error.to_string(),
                })?
                .len();
            materialization.artifact.encoded_width = encoded_width;
            materialization.artifact.encoded_height = encoded_height;
            return Ok(materialization.artifact);
        }

        let _install = self
            .install_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut replaced_size = match artifact_path.metadata() {
            Ok(metadata) => Some(metadata.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                let _ = self.remove_staged_file(&staged_path, materialization.reserved_bytes);
                return Err(ScanIssue {
                    path: Some(artifact_path.to_string_lossy().into_owned()),
                    code: "preview_size_unavailable".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        if self
            .used_bytes
            .load(Ordering::Acquire)
            .saturating_sub(replaced_size.unwrap_or(0))
            > self.budget_bytes
        {
            let _ = self.remove_staged_file(&staged_path, materialization.reserved_bytes);
            return Err(ScanIssue {
                path: Some(artifact_path.to_string_lossy().into_owned()),
                code: "preview_cache_budget_exceeded".to_owned(),
                message: format!(
                    "The preview cache budget of {} bytes is exhausted",
                    self.budget_bytes
                ),
            });
        }
        let install_result = if replaced_size.is_some() {
            installation::replace(&staged_path, &artifact_path)
        } else {
            installation::install_new(&staged_path, &artifact_path)
        };
        let install_result = match (install_result, replaced_size) {
            (Err(_), None) => match artifact_path.metadata() {
                Ok(metadata) => {
                    replaced_size = Some(metadata.len());
                    installation::replace(&staged_path, &artifact_path)
                }
                Err(error) => Err(installation::InstallationFailure::from(error)),
            },
            (result, _) => result,
        };
        let installed = match install_result {
            Ok(installed) => installed,
            Err(error) => {
                if !materialization.replace_existing
                    && let Some((encoded_width, encoded_height)) = cached_artifact_dimensions(
                        &artifact_path,
                        materialization.artifact.size_bucket,
                    )
                {
                    self.remove_staged_file(&staged_path, materialization.reserved_bytes)?;
                    materialization.artifact.byte_size = artifact_path
                        .metadata()
                        .map_err(|metadata_error| ScanIssue {
                            path: Some(artifact_path.to_string_lossy().into_owned()),
                            code: "preview_size_unavailable".to_owned(),
                            message: metadata_error.to_string(),
                        })?
                        .len();
                    materialization.artifact.encoded_width = encoded_width;
                    materialization.artifact.encoded_height = encoded_height;
                    return Ok(materialization.artifact);
                }
                let _ = self.remove_staged_file(&staged_path, materialization.reserved_bytes);
                return Err(ScanIssue {
                    path: Some(artifact_path.to_string_lossy().into_owned()),
                    code: error.code.to_owned(),
                    message: error.message,
                });
            }
        };
        if installed.released_previous
            && let Some(replaced_size) = replaced_size
        {
            self.release(replaced_size);
        }
        Ok(materialization.artifact)
    }

    pub(crate) fn discard_staged(
        &self,
        materialization: &PreviewMaterialization,
    ) -> Result<(), ScanIssue> {
        let Some(staged_path) = materialization.staged_path.as_deref() else {
            return Ok(());
        };
        let _staging_completion = PreviewStagingCompletion {
            store: self,
            artifact_path: PathBuf::from(&materialization.artifact.path),
        };
        self.remove_staged_file(Path::new(staged_path), materialization.reserved_bytes)
    }

    pub(crate) fn has_usable_artifact(&self, artifact_path: &str) -> bool {
        let artifact_path = Path::new(artifact_path);
        artifact_path.parent() == Some(self.root.as_path())
            && is_current_preview_artifact(&artifact_path.to_string_lossy())
            && cached_artifact_dimensions(artifact_path, MAX_PREVIEW_EDGE).is_some()
    }

    fn acquire_staging_target(&self, artifact_path: &Path) -> PreviewStagingTarget<'_> {
        let mut staging_targets = self
            .staging_targets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while !staging_targets.insert(artifact_path.to_path_buf()) {
            staging_targets = self
                .staging_changed
                .wait(staging_targets)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        PreviewStagingTarget {
            store: self,
            artifact_path: artifact_path.to_path_buf(),
            transfers_to_materialization: false,
        }
    }

    fn release_staging_target(&self, artifact_path: &Path) {
        let mut staging_targets = self
            .staging_targets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if staging_targets.remove(artifact_path) {
            self.staging_changed.notify_all();
        }
    }

    fn remove_staged_file(&self, staged_path: &Path, reserved_bytes: u64) -> Result<(), ScanIssue> {
        if staged_path.parent() != Some(self.root.as_path())
            || !is_current_preview_temporary(staged_path)
        {
            return Err(ScanIssue {
                path: Some(staged_path.to_string_lossy().into_owned()),
                code: "preview_discard_path_invalid".to_owned(),
                message: "The staged preview is outside the active preview cache".to_owned(),
            });
        }
        match fs::remove_file(staged_path) {
            Ok(()) => self.release(reserved_bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.release(reserved_bytes);
            }
            Err(error) => {
                return Err(ScanIssue {
                    path: Some(staged_path.to_string_lossy().into_owned()),
                    code: "preview_discard_failed".to_owned(),
                    message: error.to_string(),
                });
            }
        }
        Ok(())
    }
}

struct PreviewStagingTarget<'a> {
    store: &'a LocalPreviewStore,
    artifact_path: PathBuf,
    transfers_to_materialization: bool,
}

impl PreviewStagingTarget<'_> {
    fn transfer_to_materialization(mut self) {
        self.transfers_to_materialization = true;
    }
}

impl Drop for PreviewStagingTarget<'_> {
    fn drop(&mut self) {
        if !self.transfers_to_materialization {
            self.store.release_staging_target(&self.artifact_path);
        }
    }
}

struct PreviewStagingCompletion<'a> {
    store: &'a LocalPreviewStore,
    artifact_path: PathBuf,
}

impl Drop for PreviewStagingCompletion<'_> {
    fn drop(&mut self) {
        self.store.release_staging_target(&self.artifact_path);
    }
}

#[cfg(test)]
pub(crate) fn replace_file_atomically_for_test(
    replacement_path: &Path,
    target_path: &Path,
) -> std::io::Result<()> {
    installation::replace_file_for_fixture(replacement_path, target_path)
}

fn preview_cache_key(algorithm: &str, file: &DiscoveredFile, preview_edge: u32) -> blake3::Hash {
    let mut hasher = Hasher::new();
    hasher.update(algorithm.as_bytes());
    hasher.update(&preview_edge.to_le_bytes());
    if let Some(identity) = &file.file_identity {
        hasher.update(identity.scheme.as_bytes());
        hasher.update(&[0]);
        hasher.update(identity.value.as_bytes());
    } else {
        hasher.update(file.absolute_path.as_bytes());
    }
    hasher.update(&file.file_size.to_le_bytes());
    hasher.update(&file.modified_unix_ms.to_le_bytes());
    hasher.update(&file.source_generation.to_le_bytes());
    match &file.source_revision {
        Some(revision) => {
            hasher.update(&[1]);
            hasher.update(&(revision.scheme.len() as u64).to_le_bytes());
            hasher.update(revision.scheme.as_bytes());
            hasher.update(&(revision.value.len() as u64).to_le_bytes());
            hasher.update(revision.value.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.finalize()
}

impl PreviewStore for LocalPreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        source: &File,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
        force_regenerate: bool,
    ) -> Result<PreviewMaterialization, ScanIssue> {
        let edge = preview_size_bucket(preview_edge);
        let artifact_key = self.artifact_key(file, edge);
        let artifact_path = self.artifact_path(file, edge);

        let cached_dimensions = cached_artifact_dimensions(&artifact_path, edge);
        if !force_regenerate && let Some(encoded_dimensions) = cached_dimensions {
            let (resolved_source_width, resolved_source_height) =
                if source_width > 0 && source_height > 0 {
                    (source_width, source_height)
                } else {
                    inspect_source_display_dimensions(file, source)?
                };
            return preview_artifact(
                file,
                artifact_key,
                artifact_path.clone(),
                &artifact_path,
                edge,
                PreviewDimensions {
                    source_width: resolved_source_width,
                    source_height: resolved_source_height,
                    encoded: Some(encoded_dimensions),
                },
            )
            .map(existing_materialization);
        }
        let mut decode_source = source
            .try_clone()
            .map_err(|error| preview_issue(file, "image_open_failed", error))?;
        decode_source
            .seek(SeekFrom::Start(0))
            .map_err(|error| preview_issue(file, "image_open_failed", error))?;
        let mut reader = ImageReader::new(BufReader::new(decode_source))
            .with_guessed_format()
            .map_err(|error| preview_issue(file, "image_open_failed", error))?;
        if reader.format() == Some(ImageFormat::Jpeg) {
            let mut jpeg_source = source
                .try_clone()
                .map_err(|error| preview_issue(file, "image_open_failed", error))?;
            jpeg_source
                .seek(SeekFrom::Start(0))
                .map_err(|error| preview_issue(file, "image_open_failed", error))?;
            if let Some(decoded) = decode_scaled_jpeg(jpeg_source, edge, MAX_DECODER_ALLOCATION) {
                return publish_preview(
                    self,
                    file,
                    PreviewPublication {
                        artifact_key,
                        artifact_path,
                        edge,
                        image: decoded.image,
                        source_dimensions: (decoded.source_width, decoded.source_height),
                        force_regenerate,
                    },
                );
            }
            let mut fallback_source = reader.into_inner();
            fallback_source
                .seek(SeekFrom::Start(0))
                .map_err(|error| preview_issue(file, "image_open_failed", error))?;
            reader = ImageReader::with_format(fallback_source, ImageFormat::Jpeg);
        }
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
        limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
        reader.limits(limits);
        let mut decoder = reader
            .into_decoder()
            .map_err(|error| preview_issue(file, "image_decode_failed", error))?;
        let orientation = decoder
            .orientation()
            .map(from_image_orientation)
            .unwrap_or_else(|_| ImageOrientation::default());
        let mut image = DynamicImage::from_decoder(decoder)
            .map_err(|error| preview_issue(file, "image_decode_failed", error))?;
        apply_image_orientation(&mut image, orientation);
        let width = image.width();
        let height = image.height();

        publish_preview(
            self,
            file,
            PreviewPublication {
                artifact_key,
                artifact_path,
                edge,
                image,
                source_dimensions: (width, height),
                force_regenerate,
            },
        )
    }
}

struct PreviewPublication {
    artifact_key: String,
    artifact_path: PathBuf,
    edge: u32,
    image: DynamicImage,
    source_dimensions: (u32, u32),
    force_regenerate: bool,
}

fn publish_preview(
    store: &LocalPreviewStore,
    file: &DiscoveredFile,
    publication: PreviewPublication,
) -> Result<PreviewMaterialization, ScanIssue> {
    let PreviewPublication {
        artifact_key,
        artifact_path,
        edge,
        image,
        source_dimensions,
        force_regenerate,
    } = publication;
    if !force_regenerate
        && let Some(encoded_dimensions) = cached_artifact_dimensions(&artifact_path, edge)
    {
        return preview_artifact(
            file,
            artifact_key,
            artifact_path.clone(),
            &artifact_path,
            edge,
            PreviewDimensions {
                source_width: source_dimensions.0,
                source_height: source_dimensions.1,
                encoded: Some(encoded_dimensions),
            },
        )
        .map(existing_materialization);
    }
    let staging_target = store.acquire_staging_target(&artifact_path);
    if !force_regenerate
        && let Some(encoded_dimensions) = cached_artifact_dimensions(&artifact_path, edge)
    {
        return preview_artifact(
            file,
            artifact_key,
            artifact_path.clone(),
            &artifact_path,
            edge,
            PreviewDimensions {
                source_width: source_dimensions.0,
                source_height: source_dimensions.1,
                encoded: Some(encoded_dimensions),
            },
        )
        .map(existing_materialization);
    }

    let thumbnail = image.thumbnail(edge, edge);
    let temporary_path = artifact_path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(error) = thumbnail.save_with_format(&temporary_path, ImageFormat::Jpeg) {
        let _ = fs::remove_file(&temporary_path);
        return Err(preview_issue(file, "preview_write_failed", error));
    }
    let preview_size = match temporary_path.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            return Err(preview_issue(file, "preview_size_unavailable", error));
        }
    };
    let replaced_size = artifact_path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if !store.reserve_staged(preview_size, replaced_size) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ScanIssue {
            path: Some(file.absolute_path.clone()),
            code: "preview_cache_budget_exceeded".to_owned(),
            message: format!(
                "The preview cache budget of {} bytes is exhausted",
                store.budget_bytes
            ),
        });
    }
    let encoded_dimensions = cached_artifact_dimensions(&temporary_path, edge);
    let artifact = match preview_artifact(
        file,
        artifact_key,
        artifact_path.clone(),
        &temporary_path,
        edge,
        PreviewDimensions {
            source_width: source_dimensions.0,
            source_height: source_dimensions.1,
            encoded: encoded_dimensions,
        },
    ) {
        Ok(artifact) => artifact,
        Err(issue) => {
            let _ = store.remove_staged_file(&temporary_path, preview_size);
            return Err(issue);
        }
    };
    let replace_existing = force_regenerate || artifact_path.exists();
    staging_target.transfer_to_materialization();
    Ok(PreviewMaterialization {
        artifact,
        staged_path: Some(temporary_path.to_string_lossy().into_owned()),
        reserved_bytes: preview_size,
        replace_existing,
    })
}

fn existing_materialization(artifact: PreviewArtifact) -> PreviewMaterialization {
    PreviewMaterialization {
        artifact,
        staged_path: None,
        reserved_bytes: 0,
        replace_existing: false,
    }
}

fn preview_size_bucket(requested_edge: u32) -> u32 {
    let requested_edge = requested_edge.clamp(96, MAX_PREVIEW_EDGE);
    PREVIEW_SIZE_BUCKETS
        .into_iter()
        .find(|bucket| *bucket >= requested_edge)
        .unwrap_or(MAX_PREVIEW_EDGE)
}

impl LocalPreviewStore {
    fn reserve_staged(&self, preview_size: u64, replaced_size: u64) -> bool {
        let mut used_bytes = self.used_bytes.load(Ordering::Acquire);
        loop {
            let settled_bytes = used_bytes
                .saturating_sub(replaced_size)
                .saturating_add(preview_size);
            if settled_bytes > self.budget_bytes {
                self.rejected_reservation_bytes
                    .fetch_max(preview_size.saturating_sub(replaced_size), Ordering::AcqRel);
                return false;
            }
            // Atomic replacement temporarily keeps both files. Account the staged bytes in full,
            // but admit them against the settled post-replacement size so an exact-full cache can
            // refresh its current artifact without first deleting the last trustworthy preview.
            let next = used_bytes.saturating_add(preview_size);
            match self.used_bytes.compare_exchange_weak(
                used_bytes,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(observed) => used_bytes = observed,
            }
        }
    }

    pub(crate) fn release(&self, preview_size: u64) {
        let mut used_bytes = self.used_bytes.load(Ordering::Acquire);
        loop {
            let next = used_bytes.saturating_sub(preview_size);
            match self.used_bytes.compare_exchange_weak(
                used_bytes,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return,
                Err(observed) => used_bytes = observed,
            }
        }
    }

    pub(crate) fn used_bytes(&self) -> u64 {
        self.used_bytes.load(Ordering::Acquire)
    }

    pub(crate) fn take_rejected_reservation_bytes(&self) -> u64 {
        self.rejected_reservation_bytes.swap(0, Ordering::AcqRel)
    }
}

pub(crate) fn is_current_preview_artifact(path: &str) -> bool {
    current_preview_artifact_key(Path::new(path)).is_some()
}

pub(crate) fn current_preview_artifact_key(path: &Path) -> Option<&str> {
    let file_name = path.file_name().and_then(|name| name.to_str())?;
    let hash = file_name
        .strip_prefix(PREVIEW_ALGORITHM)
        .and_then(|suffix| suffix.strip_prefix('-'))
        .and_then(|suffix| suffix.strip_suffix(".jpg"))?;
    is_artifact_hash(hash).then_some(hash)
}

pub(crate) fn is_managed_preview_cleanup_entry(path: &Path) -> bool {
    if is_current_preview_artifact(&path.to_string_lossy()) || is_obsolete_preview_artifact(path) {
        return true;
    }
    is_current_preview_temporary(path) || is_obsolete_preview_temporary(path)
}

fn is_current_preview_temporary(path: &Path) -> bool {
    is_preview_temporary_for_algorithm(path, PREVIEW_ALGORITHM)
}

fn is_obsolete_preview_temporary(path: &Path) -> bool {
    if is_preview_temporary_for_algorithm(path, OBSOLETE_PREVIEW_ALGORITHM_V2) {
        return true;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|file_name| file_name.strip_suffix(".tmp"))
        .and_then(preview_temporary_hash)
        .is_some_and(is_legacy_artifact_hash)
}

fn is_preview_temporary_for_algorithm(path: &Path, algorithm: &str) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(body) = file_name
        .strip_prefix(algorithm)
        .and_then(|suffix| suffix.strip_prefix('-'))
        .and_then(|suffix| suffix.strip_suffix(".tmp"))
    else {
        return false;
    };
    preview_temporary_hash(body).is_some_and(is_artifact_hash)
}

fn preview_temporary_hash(body: &str) -> Option<&str> {
    let (hash, temporary_id) = body.split_once('.')?;
    let (process_id, sequence) = temporary_id.split_once('-')?;
    if process_id.is_empty()
        || !process_id.bytes().all(|byte| byte.is_ascii_digit())
        || sequence.is_empty()
        || !sequence.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(hash)
}

fn is_legacy_artifact_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_artifact_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_obsolete_preview_artifact(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let v1_hash = file_name.strip_suffix(".jpg");
    let v2_hash = file_name
        .strip_prefix(OBSOLETE_PREVIEW_ALGORITHM_V2)
        .and_then(|suffix| suffix.strip_prefix('-'))
        .and_then(|suffix| suffix.strip_suffix(".jpg"));
    v1_hash.is_some_and(is_legacy_artifact_hash) || v2_hash.is_some_and(is_legacy_artifact_hash)
}

pub(crate) fn is_ame_preview_cache_entry(path: &Path) -> bool {
    is_managed_preview_cleanup_entry(path)
}

fn cached_artifact_dimensions(path: &Path, edge: u32) -> Option<(u32, u32)> {
    let Ok(mut reader) = ImageReader::open(path).and_then(|reader| reader.with_guessed_format())
    else {
        return None;
    };
    if reader.format() != Some(ImageFormat::Jpeg) {
        return None;
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(edge);
    limits.max_image_height = Some(edge);
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    reader.limits(limits);
    let Ok(image) = reader.decode() else {
        return None;
    };
    let (width, height) = (image.width(), image.height());
    (width > 0 && height > 0 && width <= edge && height <= edge).then_some((width, height))
}

fn preview_artifact(
    file: &DiscoveredFile,
    artifact_key: String,
    artifact_path: PathBuf,
    artifact_file_path: &Path,
    size_bucket: u32,
    dimensions: PreviewDimensions,
) -> Result<PreviewArtifact, ScanIssue> {
    let (encoded_width, encoded_height) = dimensions.encoded.ok_or_else(|| ScanIssue {
        path: Some(artifact_file_path.to_string_lossy().into_owned()),
        code: "preview_cache_publish_invalid".to_owned(),
        message: "The published preview artifact could not be validated".to_owned(),
    })?;
    let byte_size = artifact_file_path
        .metadata()
        .map_err(|error| preview_issue(file, "preview_size_unavailable", error))?
        .len();
    Ok(PreviewArtifact {
        artifact_key,
        algorithm_id: PREVIEW_ALGORITHM_ID.to_owned(),
        algorithm_version: PREVIEW_ALGORITHM_VERSION,
        orientation_contract: PREVIEW_ORIENTATION_CONTRACT.to_owned(),
        size_bucket,
        path: artifact_path.to_string_lossy().into_owned(),
        byte_size,
        encoded_width,
        encoded_height,
        width: dimensions.source_width,
        height: dimensions.source_height,
    })
}

fn cache_inventory(root: &Path) -> Result<(u64, bool), ScanIssue> {
    let mut used_bytes = 0_u64;
    let mut has_legacy_artifacts = false;
    let entries = fs::read_dir(root).map_err(|error| ScanIssue {
        path: Some(root.to_string_lossy().into_owned()),
        code: "preview_cache_usage_unavailable".to_owned(),
        message: error.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| ScanIssue {
            path: Some(root.to_string_lossy().into_owned()),
            code: "preview_cache_usage_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let metadata = entry.metadata().map_err(|error| ScanIssue {
            path: Some(entry.path().to_string_lossy().into_owned()),
            code: "preview_cache_usage_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if metadata.is_file() && is_ame_preview_cache_entry(&entry.path()) {
            used_bytes = used_bytes.saturating_add(metadata.len());
            has_legacy_artifacts |= is_obsolete_preview_artifact(&entry.path());
        }
    }
    Ok((used_bytes, has_legacy_artifacts))
}

fn preview_issue(file: &DiscoveredFile, code: &str, error: impl std::fmt::Display) -> ScanIssue {
    ScanIssue {
        path: Some(file.absolute_path.clone()),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

fn inspect_source_display_dimensions(
    file: &DiscoveredFile,
    source: &File,
) -> Result<(u32, u32), ScanIssue> {
    let source = source
        .try_clone()
        .map_err(|error| preview_issue(file, "image_open_failed", error))?;
    let mut reader = ImageReader::new(BufReader::new(source))
        .with_guessed_format()
        .map_err(|error| preview_issue(file, "image_open_failed", error))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| preview_issue(file, "image_dimensions_failed", error))?;
    let (pixel_width, pixel_height) = decoder.dimensions();
    if pixel_width > MAX_SOURCE_DIMENSION || pixel_height > MAX_SOURCE_DIMENSION {
        return Err(ScanIssue {
            path: Some(file.absolute_path.clone()),
            code: "image_dimensions_exceeded".to_owned(),
            message: format!(
                "Image dimensions {pixel_width}x{pixel_height} exceed the supported limit"
            ),
        });
    }
    let orientation = decoder
        .orientation()
        .map(from_image_orientation)
        .unwrap_or_default();
    Ok(orientation.display_dimensions(pixel_width, pixel_height))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    use exif::experimental::Writer;
    use exif::{Field, In, Tag, Value};
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, GenericImageView, ImageEncoder, Rgb, RgbImage, Rgba};
    use tempfile::tempdir;

    use super::super::local_files::canonical_source_root_path;
    use super::*;

    fn fixture_source_root(source_path: &Path) -> String {
        canonical_source_root_path(source_path.parent().expect("source root"))
            .expect("canonical source root")
            .to_string_lossy()
            .into_owned()
    }

    fn materialize_and_commit(
        store: &LocalPreviewStore,
        file: &DiscoveredFile,
        preview_edge: u32,
        source_width: u32,
        source_height: u32,
    ) -> PreviewArtifact {
        let source = File::open(&file.absolute_path).expect("open preview source");
        let materialization = store
            .materialize(
                file,
                &source,
                preview_edge,
                source_width,
                source_height,
                false,
            )
            .expect("materialize preview");
        store.commit(materialization).expect("commit preview")
    }

    #[test]
    fn exhausted_budget_does_not_publish_or_modify_media() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 32, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let source_before = fs::read(&source_path).expect("source before");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1).expect("preview store");

        let source = File::open(&source_path).expect("open source");
        let issue = store
            .materialize(&file, &source, 256, 32, 32, false)
            .expect_err("budget issue");

        assert_eq!(issue.code, "preview_cache_budget_exceeded");
        assert_eq!(fs::read(&source_path).expect("source after"), source_before);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            0
        );
    }

    #[test]
    fn existing_artifact_uses_catalog_dimensions_without_decoding_source() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.jpg");
        let source_bytes = b"not a decodable image";
        fs::write(&source_path, source_bytes).expect("write source");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.jpg".to_owned(),
            file_size: u64::try_from(source_bytes.len()).expect("source size"),
            created_unix_ms: None,
            modified_unix_ms: 7,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1024).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        RgbImage::from_pixel(16, 16, Rgb([12, 34, 56]))
            .save(&artifact_path)
            .expect("write preview artifact");
        drop(store);
        let store = LocalPreviewStore::new(preview_root, 1024).expect("reopen preview store");

        let preview = materialize_and_commit(&store, &file, 256, 4032, 3024);

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert_eq!((preview.width, preview.height), (4032, 3024));
        assert_eq!(fs::read(source_path).expect("source after"), source_bytes);
    }

    #[test]
    fn existing_artifact_recovers_oriented_source_dimensions_from_headers() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("oriented.jpg");
        let source_bytes = orientation_jpeg(6);
        fs::write(&source_path, &source_bytes).expect("write source");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "oriented.jpg".to_owned(),
            file_size: u64::try_from(source_bytes.len()).expect("source size"),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1024).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        RgbImage::from_pixel(16, 16, Rgb([12, 34, 56]))
            .save(&artifact_path)
            .expect("write preview artifact");
        let artifact_before = fs::read(&artifact_path).expect("artifact before");
        drop(store);
        let store = LocalPreviewStore::new(preview_root, 1024).expect("reopen preview store");

        let preview = materialize_and_commit(&store, &file, 256, 0, 0);

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert_eq!((preview.width, preview.height), (60, 80));
        assert_eq!(
            fs::read(&artifact_path).expect("artifact after"),
            artifact_before
        );
        assert_eq!(fs::read(source_path).expect("source after"), source_bytes);
    }

    #[test]
    fn generated_preview_is_staged_until_revalidation_and_commit() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store =
            LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);

        let source = File::open(&source_path).expect("open source");
        let staged = store
            .materialize(&file, &source, 256, 32, 24, false)
            .expect("staged preview");
        let staged_path = PathBuf::from(staged.staged_path.as_ref().expect("staged path"));

        assert!(!artifact_path.exists());
        assert!(staged_path.exists());
        assert!(store.used_bytes() > 0);

        store
            .discard_staged(&staged)
            .expect("discard staged preview");
        assert!(!staged_path.exists());
        assert!(!artifact_path.exists());
        assert_eq!(store.used_bytes(), 0);

        let staged = store
            .materialize(&file, &source, 256, 32, 24, false)
            .expect("replacement staged preview");
        let committed = store.commit(staged).expect("commit preview");
        assert_eq!(PathBuf::from(&committed.path), artifact_path);
        assert!(artifact_path.exists());

        let existing = store
            .materialize(&file, &source, 256, 32, 24, false)
            .expect("existing preview");
        assert!(existing.staged_path.is_none());
    }

    #[test]
    fn v3_cache_key_includes_source_generation_and_revision() {
        let file = DiscoveredFile {
            source_root_path: "C:\\source".to_owned(),
            absolute_path: "C:\\source\\one.png".to_owned(),
            relative_path: "one.png".to_owned(),
            file_size: 10,
            created_unix_ms: None,
            modified_unix_ms: 20,
            file_identity: None,
            source_revision: Some(crate::domain::SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 1,
            issues: Vec::new(),
        };
        let mut newer_generation = file.clone();
        newer_generation.source_generation = 2;
        let mut newer_revision = file.clone();
        newer_revision.source_revision = Some(crate::domain::SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0000000000000002".to_owned(),
        });

        let original = preview_cache_key(PREVIEW_ALGORITHM, &file, 256);
        assert_ne!(
            original,
            preview_cache_key(PREVIEW_ALGORITHM, &newer_generation, 256)
        );
        assert_ne!(
            original,
            preview_cache_key(PREVIEW_ALGORITHM, &newer_revision, 256)
        );
    }

    #[test]
    fn force_regenerate_atomically_replaces_a_valid_but_wrong_target() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root, 1024 * 1024).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        RgbImage::from_pixel(16, 16, Rgb([192, 24, 96]))
            .save(&artifact_path)
            .expect("wrong existing preview");
        let wrong_bytes = fs::read(&artifact_path).expect("wrong preview bytes");
        let source = File::open(&source_path).expect("open source");

        let materialization = store
            .materialize(&file, &source, 256, 32, 24, true)
            .expect("forced materialization");
        assert!(materialization.staged_path.is_some());
        let preview = store.commit(materialization).expect("forced commit");

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert_ne!(
            fs::read(&artifact_path).expect("new preview bytes"),
            wrong_bytes
        );
    }

    #[test]
    fn atomic_replace_failure_retains_old_target_and_exact_accounting() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let artifact_path = preview_root.join(format!(
            "{PREVIEW_ALGORITHM}-{}.jpg",
            preview_cache_key(PREVIEW_ALGORITHM, &file, 256).to_hex()
        ));
        RgbImage::from_pixel(16, 16, Rgb([192, 24, 96]))
            .save(&artifact_path)
            .expect("old target");
        let old_bytes = fs::read(&artifact_path).expect("old target bytes");
        let old_size = u64::try_from(old_bytes.len()).expect("old target size");
        let store = LocalPreviewStore::new(preview_root, 1024 * 1024).expect("preview store");
        let source = File::open(&source_path).expect("open source");
        let materialization = store
            .materialize(&file, &source, 256, 32, 24, true)
            .expect("forced materialization");
        fail_next_atomic_replace_for_test(&artifact_path);

        let issue = store.commit(materialization).expect_err("replace failure");

        assert_eq!(issue.code, "preview_publish_failed");
        assert_eq!(
            fs::read(&artifact_path).expect("retained target"),
            old_bytes
        );
        assert_eq!(store.used_bytes(), old_size);
    }

    #[test]
    fn exact_full_budget_allows_atomic_replacement_by_settled_size() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let initial_store =
            LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("preview store");
        let initial = materialize_and_commit(&initial_store, &file, 256, 32, 24);
        let artifact_path = PathBuf::from(&initial.path);
        let settled_size = artifact_path.metadata().expect("preview metadata").len();
        drop(initial_store);

        let exact_store =
            LocalPreviewStore::new(preview_root, settled_size).expect("exact budget store");
        let source = File::open(&source_path).expect("open source");
        let replacement = exact_store
            .materialize(&file, &source, 256, 32, 24, true)
            .expect("stage exact-budget replacement");

        assert_eq!(replacement.reserved_bytes, settled_size);
        assert_eq!(exact_store.used_bytes(), settled_size.saturating_mul(2));
        exact_store
            .commit(replacement)
            .expect("commit exact-budget replacement");
        assert_eq!(exact_store.used_bytes(), settled_size);
        assert_eq!(
            artifact_path.metadata().expect("preview metadata").len(),
            settled_size
        );
    }

    #[test]
    fn concurrent_forced_replacements_singleflight_staging_with_two_artifact_budget() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let source_bytes = fs::read(&source_path).expect("source bytes");
        let metadata = source_path.metadata().expect("source metadata");
        let file = Arc::new(DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        });
        let preview_root = storage.path().join("previews");
        let initial_store =
            LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("preview store");
        let initial = materialize_and_commit(&initial_store, &file, 256, 32, 24);
        let settled_size = Path::new(&initial.path)
            .metadata()
            .expect("preview metadata")
            .len();
        drop(initial_store);
        let store = Arc::new(
            LocalPreviewStore::new(preview_root.clone(), settled_size.saturating_mul(2))
                .expect("two-artifact budget store"),
        );

        let (first_staged_tx, first_staged_rx) = mpsc::channel();
        let (allow_first_commit_tx, allow_first_commit_rx) = mpsc::channel();
        let first_store = Arc::clone(&store);
        let first_file = Arc::clone(&file);
        let first = std::thread::spawn(move || {
            let source = File::open(&first_file.absolute_path).expect("open first source");
            let materialization = first_store
                .materialize(&first_file, &source, 256, 32, 24, true)
                .expect("first forced materialization");
            first_staged_tx.send(()).expect("first staged signal");
            allow_first_commit_rx.recv().expect("allow first commit");
            first_store
                .commit(materialization)
                .expect("first forced commit")
        });
        first_staged_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("first preview staged");

        let (second_started_tx, second_started_rx) = mpsc::channel();
        let (second_staged_tx, second_staged_rx) = mpsc::channel();
        let (allow_second_commit_tx, allow_second_commit_rx) = mpsc::channel();
        let second_store = Arc::clone(&store);
        let second_file = Arc::clone(&file);
        let second = std::thread::spawn(move || {
            let source = File::open(&second_file.absolute_path).expect("open second source");
            second_started_tx.send(()).expect("second started signal");
            let materialization = second_store
                .materialize(&second_file, &source, 256, 32, 24, true)
                .expect("second forced materialization");
            second_staged_tx.send(()).expect("second staged signal");
            allow_second_commit_rx.recv().expect("allow second commit");
            second_store
                .commit(materialization)
                .expect("second forced commit")
        });
        second_started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("second preview started");
        let staged_before_first_commit = second_staged_rx
            .recv_timeout(Duration::from_millis(250))
            .is_ok();
        let used_before_first_commit = store.used_bytes();

        allow_first_commit_tx.send(()).expect("allow first commit");
        first.join().expect("first preview worker");
        if !staged_before_first_commit {
            second_staged_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("second preview staged after first commit");
        }
        allow_second_commit_tx
            .send(())
            .expect("allow second commit");
        second.join().expect("second preview worker");

        assert!(!staged_before_first_commit);
        assert_eq!(used_before_first_commit, settled_size.saturating_mul(2));
        assert_eq!(store.used_bytes(), settled_size);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            1,
        );
        assert_eq!(fs::read(source_path).expect("source after"), source_bytes);
    }

    #[test]
    fn concurrent_materializations_publish_one_v3_artifact() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = Arc::new(DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        });
        let preview_root = storage.path().join("previews");
        let store = Arc::new(
            LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("preview store"),
        );
        let barrier = Arc::new(Barrier::new(2));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let store = Arc::clone(&store);
            let file = Arc::clone(&file);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                let source = File::open(&file.absolute_path).expect("open source");
                barrier.wait();
                let materialization = store
                    .materialize(&file, &source, 256, 32, 24, false)
                    .expect("materialize preview");
                store.commit(materialization).expect("commit preview")
            }));
        }
        let previews = workers
            .into_iter()
            .map(|worker| worker.join().expect("preview worker"))
            .collect::<Vec<_>>();

        assert_eq!(previews[0].artifact_key, previews[1].artifact_key);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            1
        );
    }

    #[test]
    fn obsolete_v1_and_v2_artifacts_are_managed_but_never_reused_or_promoted() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.jpg");
        RgbImage::from_pixel(32, 24, Rgb([24, 96, 192]))
            .save(&source_path)
            .expect("source image");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.jpg".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 8,
            file_identity: None,
            source_revision: Some(crate::domain::SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 7,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        fs::create_dir_all(&preview_root).expect("preview root");
        let hash = "a".repeat(64);
        let v1_path = preview_root.join(format!("{hash}.jpg"));
        let v2_path = preview_root.join(format!("{OBSOLETE_PREVIEW_ALGORITHM_V2}-{hash}.jpg"));
        RgbImage::from_pixel(16, 12, Rgb([12, 34, 56]))
            .save(&v1_path)
            .expect("v1 preview");
        RgbImage::from_pixel(16, 12, Rgb([56, 34, 12]))
            .save(&v2_path)
            .expect("v2 preview");
        let obsolete_bytes = v1_path.metadata().expect("v1 metadata").len()
            + v2_path.metadata().expect("v2 metadata").len();
        let store = LocalPreviewStore::new(preview_root, 1024 * 1024).expect("preview store");
        let current_path = store.artifact_path(&file, 256);

        let preview = materialize_and_commit(&store, &file, 256, 32, 24);

        assert_eq!(PathBuf::from(preview.path), current_path);
        assert!(v1_path.exists());
        assert!(v2_path.exists());
        assert!(is_ame_preview_cache_entry(&v1_path));
        assert!(is_ame_preview_cache_entry(&v2_path));
        assert!(store.used_bytes() > obsolete_bytes);
    }

    #[test]
    fn corrupt_cached_artifact_is_rebuilt_from_source() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(32, 24, Rgb([48, 96, 144]))
            .save(&source_path)
            .expect("source image");
        let source_before = fs::read(&source_path).expect("source before");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 11,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 4096).expect("preview store");
        let artifact_path = store.artifact_path(&file, 256);
        write_truncated_jpeg(&artifact_path);
        drop(store);
        let store = LocalPreviewStore::new(preview_root, 4096).expect("reopen preview store");

        let preview = materialize_and_commit(&store, &file, 256, 32, 24);

        assert_eq!(PathBuf::from(preview.path), artifact_path);
        assert!(cached_artifact_dimensions(&artifact_path, 256).is_some());
        assert_eq!(fs::read(&source_path).expect("source after"), source_before);
    }

    #[test]
    fn malformed_jpeg_fast_path_falls_back_to_a_structured_decode_failure() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("malformed.data");
        let source_bytes = b"\xFF\xD8\xFF\xE0not-a-complete-jpeg";
        fs::write(&source_path, source_bytes).expect("malformed jpeg fixture");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "malformed.data".to_owned(),
            file_size: u64::try_from(source_bytes.len()).expect("source size"),
            created_unix_ms: None,
            modified_unix_ms: 12,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 4096).expect("preview store");

        let source = File::open(&source_path).expect("open source");
        let issue = store
            .materialize(&file, &source, 256, 0, 0, false)
            .expect_err("malformed jpeg issue");

        assert_eq!(issue.code, "image_decode_failed");
        assert_eq!(fs::read(&source_path).expect("source after"), source_bytes);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            0
        );
    }

    #[test]
    fn requested_edges_share_a_finite_physical_size_bucket() {
        let storage = tempdir().expect("storage");
        let source_path = storage.path().join("source.png");
        RgbImage::from_pixel(640, 480, Rgb([48, 96, 144]))
            .save(&source_path)
            .expect("source image");
        let source_before = fs::read(&source_path).expect("source before");
        let metadata = source_path.metadata().expect("source metadata");
        let file = DiscoveredFile {
            source_root_path: fixture_source_root(&source_path),
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 13,
            file_identity: None,
            source_revision: None,
            source_generation: 1,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store =
            LocalPreviewStore::new(preview_root.clone(), 1024 * 1024).expect("preview store");

        let first = materialize_and_commit(&store, &file, 129, 640, 480);
        let second = materialize_and_commit(&store, &file, 255, 640, 480);

        assert_eq!(first.size_bucket, 256);
        assert_eq!(second.size_bucket, 256);
        assert_eq!(first.artifact_key, second.artifact_key);
        assert_eq!(first.path, second.path);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            1
        );
        assert_eq!(fs::read(source_path).expect("source after"), source_before);
    }

    #[test]
    fn cleanup_entry_matching_rejects_foreign_files() {
        let hash = "a".repeat(64);

        assert!(is_managed_preview_cleanup_entry(Path::new(&format!(
            "{PREVIEW_ALGORITHM}-{hash}.jpg"
        ))));
        assert!(is_managed_preview_cleanup_entry(Path::new(&format!(
            "{PREVIEW_ALGORITHM}-{hash}.123-4.tmp"
        ))));
        assert!(!is_managed_preview_cleanup_entry(Path::new(&format!(
            "{PREVIEW_ALGORITHM}-{hash}.notes.tmp"
        ))));
        assert!(!is_managed_preview_cleanup_entry(Path::new(&format!(
            "{PREVIEW_ALGORITHM}-{hash}.123-4-extra.tmp"
        ))));
        assert!(!is_managed_preview_cleanup_entry(Path::new(
            "unrelated.tmp",
        )));
        assert!(is_managed_preview_cleanup_entry(Path::new(&format!(
            "{hash}.jpg"
        ))));
        assert!(is_managed_preview_cleanup_entry(Path::new(&format!(
            "{hash}.123-4.tmp"
        ))));
        assert!(!is_managed_preview_cleanup_entry(Path::new(&format!(
            "{hash}.notes.tmp"
        ))));
        assert!(!is_managed_preview_cleanup_entry(Path::new(&format!(
            "{}.123-4.tmp",
            "A".repeat(64)
        ))));
        assert!(is_ame_preview_cache_entry(Path::new(&format!(
            "{hash}.jpg"
        ))));
        assert!(!is_ame_preview_cache_entry(Path::new(&format!(
            "{}.jpg",
            "A".repeat(64)
        ))));
        assert!(!is_ame_preview_cache_entry(Path::new("keep.jpg")));
    }

    #[test]
    fn cache_inventory_counts_current_and_legacy_artifacts() {
        let directory = tempdir().expect("preview directory");
        let hash = "b".repeat(64);
        let artifact = directory
            .path()
            .join(format!("{PREVIEW_ALGORITHM}-{hash}.jpg"));
        let temporary = directory
            .path()
            .join(format!("{PREVIEW_ALGORITHM}-{hash}.123-4.tmp"));
        let legacy = directory.path().join(format!("{hash}.jpg"));
        let legacy_temporary = directory.path().join(format!("{hash}.456-7.tmp"));
        let foreign = directory.path().join("keep.bin");
        fs::write(&artifact, vec![1_u8; 7]).expect("artifact");
        fs::write(&temporary, vec![2_u8; 5]).expect("temporary");
        fs::write(&legacy, vec![3_u8; 13]).expect("legacy");
        fs::write(&legacy_temporary, vec![4_u8; 17]).expect("legacy temporary");
        fs::write(&foreign, vec![3_u8; 11]).expect("foreign");

        assert_eq!(
            cache_inventory(directory.path()).expect("cache inventory"),
            (42, true)
        );
    }

    #[test]
    fn exif_orientation_transforms_preview_pixels_dimensions_and_cache_identity() {
        let cases = [
            (1, (80, 60), (256, 192), [RED, GREEN, BLUE, YELLOW]),
            (2, (80, 60), (256, 192), [GREEN, RED, YELLOW, BLUE]),
            (3, (80, 60), (256, 192), [YELLOW, BLUE, GREEN, RED]),
            (4, (80, 60), (256, 192), [BLUE, YELLOW, RED, GREEN]),
            (5, (60, 80), (192, 256), [RED, BLUE, GREEN, YELLOW]),
            (6, (60, 80), (192, 256), [BLUE, RED, YELLOW, GREEN]),
            (7, (60, 80), (192, 256), [YELLOW, GREEN, BLUE, RED]),
            (8, (60, 80), (192, 256), [GREEN, YELLOW, RED, BLUE]),
            (9, (80, 60), (256, 192), [RED, GREEN, BLUE, YELLOW]),
        ];

        for (orientation, expected_dimensions, expected_preview_dimensions, expected_corners) in
            cases
        {
            let storage = tempdir().expect("storage");
            let source_path = storage
                .path()
                .join(format!("orientation-{orientation}.jpg"));
            let source_bytes = orientation_jpeg(orientation);
            fs::write(&source_path, &source_bytes).expect("write orientation fixture");
            let file = DiscoveredFile {
                source_root_path: fixture_source_root(&source_path),
                absolute_path: source_path.to_string_lossy().into_owned(),
                relative_path: format!("orientation-{orientation}.jpg"),
                file_size: u64::try_from(source_bytes.len()).expect("source size"),
                created_unix_ms: None,
                modified_unix_ms: i64::from(orientation),
                file_identity: None,
                source_revision: None,
                source_generation: 1,
                issues: Vec::new(),
            };
            let preview_root = storage.path().join("previews");
            let store = LocalPreviewStore::new(preview_root, 1024 * 1024)
                .expect("orientation preview store");

            let preview = materialize_and_commit(&store, &file, 256, 80, 60);
            let preview_path = PathBuf::from(&preview.path);
            let rendered = image::open(&preview_path).expect("decode oriented preview");

            assert_eq!((preview.width, preview.height), expected_dimensions);
            assert_eq!(rendered.dimensions(), expected_preview_dimensions);
            assert_corners_near(&rendered, expected_corners);
            assert!(is_current_preview_artifact(&preview.path));
            assert_eq!(
                fs::read(&source_path).expect("source after preview"),
                source_bytes,
            );

            let legacy_key = preview_cache_key("ame-jpeg-thumbnail-v1", &file, 256);
            let current_key = preview_cache_key(PREVIEW_ALGORITHM, &file, 256);
            assert_ne!(legacy_key, current_key);
            let legacy_path = storage.path().join(format!("{}.jpg", legacy_key.to_hex()));
            assert!(!is_current_preview_artifact(&legacy_path.to_string_lossy()));
        }
    }

    const RED: [u8; 3] = [240, 24, 24];
    const GREEN: [u8; 3] = [24, 220, 24];
    const BLUE: [u8; 3] = [24, 24, 240];
    const YELLOW: [u8; 3] = [240, 220, 24];

    fn orientation_jpeg(orientation: u16) -> Vec<u8> {
        let image = RgbImage::from_fn(80, 60, |x, y| match (x < 40, y < 30) {
            (true, true) => Rgb(RED),
            (false, true) => Rgb(GREEN),
            (true, false) => Rgb(BLUE),
            (false, false) => Rgb(YELLOW),
        });
        let mut exif_writer = Writer::new();
        let orientation_field = Field {
            tag: Tag::Orientation,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![orientation]),
        };
        exif_writer.push_field(&orientation_field);
        let mut exif = Cursor::new(Vec::new());
        exif_writer.write(&mut exif, false).expect("encode EXIF");

        let mut jpeg = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 100);
        encoder
            .set_exif_metadata(exif.into_inner())
            .expect("set orientation EXIF");
        encoder
            .encode(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .expect("encode orientation JPEG");
        jpeg
    }

    fn assert_corners_near(image: &DynamicImage, expected: [[u8; 3]; 4]) {
        let (width, height) = image.dimensions();
        let points = [
            (width / 4, height / 4),
            (width * 3 / 4, height / 4),
            (width / 4, height * 3 / 4),
            (width * 3 / 4, height * 3 / 4),
        ];
        for ((x, y), expected) in points.into_iter().zip(expected) {
            let actual = image.get_pixel(x, y);
            assert_color_near(actual, expected);
        }
    }

    fn assert_color_near(actual: Rgba<u8>, expected: [u8; 3]) {
        for (actual, expected) in actual.0[..3].iter().zip(expected) {
            assert!(
                actual.abs_diff(expected) <= 32,
                "actual color {:?} differs from expected {expected:?}",
                actual
            );
        }
    }

    fn write_truncated_jpeg(path: &Path) {
        let image = RgbImage::from_fn(32, 24, |x, y| {
            Rgb([
                (x * 7 + y * 3) as u8,
                (x * 5 + y * 11) as u8,
                (x * 13 + y * 17) as u8,
            ])
        });
        image.save(path).expect("complete jpeg");
        let jpeg = fs::read(path).expect("jpeg bytes");
        fs::write(path, &jpeg[..jpeg.len() / 2]).expect("truncated jpeg");
        assert!(cached_artifact_dimensions(path, 256).is_none());
    }
}
