use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use blake3::Hasher;
use image::{ImageFormat, ImageReader, Limits};

use crate::domain::{DiscoveredFile, PreviewArtifact, ScanIssue};
use crate::ports::PreviewStore;

const PREVIEW_ALGORITHM: &str = "ame-jpeg-thumbnail-v1";
const MAX_PREVIEW_EDGE: u32 = 1024;
const MAX_SOURCE_DIMENSION: u32 = 100_000;
const MAX_DECODER_ALLOCATION: u64 = 256 * 1024 * 1024;
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct LocalPreviewStore {
    root: PathBuf,
    budget_bytes: u64,
    used_bytes: AtomicU64,
}

impl LocalPreviewStore {
    pub fn new(root: PathBuf, budget_bytes: u64) -> Result<Self, ScanIssue> {
        fs::create_dir_all(&root).map_err(|error| ScanIssue {
            path: Some(root.to_string_lossy().into_owned()),
            code: "preview_cache_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        let used_bytes = cache_usage(&root)?;
        Ok(Self {
            root,
            budget_bytes,
            used_bytes: AtomicU64::new(used_bytes),
        })
    }

    fn artifact_path(&self, file: &DiscoveredFile, preview_edge: u32) -> PathBuf {
        let mut hasher = Hasher::new();
        hasher.update(PREVIEW_ALGORITHM.as_bytes());
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
        self.root
            .join(format!("{}.jpg", hasher.finalize().to_hex()))
    }
}

impl PreviewStore for LocalPreviewStore {
    fn materialize(
        &self,
        file: &DiscoveredFile,
        preview_edge: u32,
    ) -> Result<PreviewArtifact, ScanIssue> {
        let edge = preview_edge.clamp(96, MAX_PREVIEW_EDGE);
        let source_path = Path::new(&file.absolute_path);
        let artifact_path = self.artifact_path(file, edge);

        let mut reader = ImageReader::open(source_path)
            .and_then(|reader| reader.with_guessed_format())
            .map_err(|error| preview_issue(file, "image_open_failed", error))?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
        limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|error| preview_issue(file, "image_decode_failed", error))?;
        let width = image.width();
        let height = image.height();

        if !artifact_path.exists() {
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
            if !self.reserve(preview_size) {
                let _ = fs::remove_file(&temporary_path);
                return Err(ScanIssue {
                    path: Some(file.absolute_path.clone()),
                    code: "preview_cache_budget_exceeded".to_owned(),
                    message: format!(
                        "The preview cache budget of {} bytes is exhausted",
                        self.budget_bytes
                    ),
                });
            }
            if let Err(error) = fs::rename(&temporary_path, &artifact_path) {
                self.used_bytes.fetch_sub(preview_size, Ordering::AcqRel);
                if artifact_path.exists() {
                    let _ = fs::remove_file(&temporary_path);
                } else {
                    let _ = fs::remove_file(&temporary_path);
                    return Err(preview_issue(file, "preview_publish_failed", error));
                }
            }
        }

        Ok(PreviewArtifact {
            path: artifact_path.to_string_lossy().into_owned(),
            width,
            height,
        })
    }
}

impl LocalPreviewStore {
    fn reserve(&self, preview_size: u64) -> bool {
        let mut used_bytes = self.used_bytes.load(Ordering::Acquire);
        loop {
            let next = used_bytes.saturating_add(preview_size);
            if next > self.budget_bytes {
                return false;
            }
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
}

fn cache_usage(root: &Path) -> Result<u64, ScanIssue> {
    let mut used_bytes = 0_u64;
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
        if metadata.is_file() {
            used_bytes = used_bytes.saturating_add(metadata.len());
        }
    }
    Ok(used_bytes)
}

fn preview_issue(file: &DiscoveredFile, code: &str, error: impl std::fmt::Display) -> ScanIssue {
    ScanIssue {
        path: Some(file.absolute_path.clone()),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use tempfile::tempdir;

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
            absolute_path: source_path.to_string_lossy().into_owned(),
            relative_path: "source.png".to_owned(),
            file_size: metadata.len(),
            created_unix_ms: None,
            modified_unix_ms: 0,
            file_identity: None,
            issues: Vec::new(),
        };
        let preview_root = storage.path().join("previews");
        let store = LocalPreviewStore::new(preview_root.clone(), 1).expect("preview store");

        let issue = store.materialize(&file, 256).expect_err("budget issue");

        assert_eq!(issue.code, "preview_cache_budget_exceeded");
        assert_eq!(fs::read(&source_path).expect("source after"), source_before);
        assert_eq!(
            fs::read_dir(preview_root).expect("preview entries").count(),
            0
        );
    }
}
