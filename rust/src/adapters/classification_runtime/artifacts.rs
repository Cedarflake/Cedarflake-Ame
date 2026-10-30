use std::fs::{File, OpenOptions};
use std::io::{Read, Seek};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf, Prefix};

use sha2::{Digest, Sha256};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_NO_RECALL,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::manifest::{self, Artifact};
use super::{EncoderKind, RuntimeError};

pub(super) struct ArtifactSet {
    pub runtime: PathBuf,
    pub graph: PathBuf,
    pub tokenizer: Option<PathBuf>,
    _guards: Vec<File>,
}

impl ArtifactSet {
    pub fn open(models: &Path, runtime: &Path, kind: EncoderKind) -> Result<Self, RuntimeError> {
        let mut guards = pin_directory(models)?;
        guards.extend(pin_directory(runtime)?);
        for artifact in [&manifest::RUNTIME, &manifest::PROVIDER] {
            guards.push(verify_file(&runtime.join(artifact.name), artifact)?);
        }
        let graph = match kind {
            EncoderKind::Image => &manifest::IMAGE,
            EncoderKind::Text => &manifest::TEXT,
        };
        guards.push(verify_file(&models.join(graph.name), graph)?);
        let tokenizer = if kind == EncoderKind::Text {
            let path = models.join(manifest::TOKENIZER.name);
            guards.push(verify_file(&path, &manifest::TOKENIZER)?);
            Some(path)
        } else {
            None
        };
        Ok(Self {
            runtime: runtime.join(manifest::RUNTIME.name),
            graph: models.join(graph.name),
            tokenizer,
            _guards: guards,
        })
    }
}

fn admitted_attributes(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_REPARSE_POINT
            | FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        == 0
}

fn pin_directory(path: &Path) -> Result<Vec<File>, RuntimeError> {
    let mut components = path.components();
    if !path.is_absolute()
        || !matches!(
            components.next(),
            Some(Component::Prefix(prefix))
                if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
        || components.any(|component| matches!(component, Component::ParentDir))
    {
        return Err(RuntimeError::UnsafeArtifact);
    }
    let ancestors: Vec<_> = path.ancestors().collect();
    let mut guards = Vec::with_capacity(ancestors.len());
    for ancestor in ancestors.into_iter().rev() {
        let guard = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(
                FILE_FLAG_BACKUP_SEMANTICS
                    | FILE_FLAG_OPEN_REPARSE_POINT
                    | FILE_FLAG_OPEN_NO_RECALL,
            )
            .open(ancestor)?;
        let metadata = guard.metadata()?;
        if !metadata.is_dir() || !admitted_attributes(metadata.file_attributes()) {
            return Err(RuntimeError::UnsafeArtifact);
        }
        guards.push(guard);
    }
    Ok(guards)
}

fn verify_file(path: &Path, artifact: &Artifact) -> Result<File, RuntimeError> {
    let mut file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || !admitted_attributes(metadata.file_attributes()) {
        return Err(RuntimeError::UnsafeArtifact);
    }
    if metadata.len() != artifact.length {
        return Err(RuntimeError::ArtifactMismatch);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if format!("{:x}", hasher.finalize()) != artifact.digest {
        return Err(RuntimeError::ArtifactMismatch);
    }
    file.rewind()?;
    Ok(file)
}

#[cfg(test)]
mod tests;
