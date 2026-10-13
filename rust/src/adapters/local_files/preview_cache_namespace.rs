use std::path::{Path, PathBuf};

use crate::domain::{FileIdentityEvidence, ScanError};

#[cfg(windows)]
use super::{
    WindowsDirectoryRootProof, publication_namespace_path,
    validate_publication_namespace_directory, windows_path_uses_device_namespace,
};

pub(crate) struct PreviewCacheNamespace {
    requested_root: PathBuf,
    root: Option<PathBuf>,
    #[cfg(windows)]
    _root_proof: Option<WindowsDirectoryRootProof>,
    #[cfg(windows)]
    _ancestors: Vec<std::fs::File>,
}

impl PreviewCacheNamespace {
    pub(crate) fn open(path: &Path) -> Result<Self, ScanError> {
        #[cfg(windows)]
        {
            if !path.is_absolute() || windows_path_uses_device_namespace(path) {
                return Err(namespace_error(std::io::Error::other(
                    "Preview cleanup requires an absolute filesystem directory path",
                )));
            }
            publication_namespace_path(path).map_err(namespace_error)?;
            match std::fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // This execution has no deletion authority, even if the path appears later.
                    return Ok(Self {
                        requested_root: path.to_path_buf(),
                        root: None,
                        _root_proof: None,
                        _ancestors: Vec::new(),
                    });
                }
                Err(error) => return Err(namespace_error(error)),
                Ok(_) => {}
            }
            let (root_proof, root, ancestors) =
                WindowsDirectoryRootProof::open_publication_namespace(path, None)
                    .map_err(namespace_error)?;
            Ok(Self {
                requested_root: path.to_path_buf(),
                root: Some(root),
                _root_proof: Some(root_proof),
                _ancestors: ancestors,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Err(ScanError::new(
                "preview_cleanup_namespace_unsupported",
                "Preview cleanup requires the Windows directory namespace capability",
            ))
        }
    }

    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub(crate) fn admits(&self, path: &Path) -> bool {
        self.root.is_some() && self.requested_root == path
    }

    pub(crate) fn identity(&self) -> Option<&FileIdentityEvidence> {
        #[cfg(windows)]
        {
            self._root_proof.as_ref().map(|proof| &proof.identity)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    pub(crate) fn for_operation(path: &Path) -> Result<Self, ScanError> {
        #[cfg(windows)]
        {
            if !path.is_absolute() || windows_path_uses_device_namespace(path) {
                return Err(namespace_error(std::io::Error::other(
                    "The cache requires an absolute filesystem directory path",
                )));
            }
            let (_, components) = publication_namespace_path(path).map_err(namespace_error)?;
            match std::fs::symlink_metadata(path) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let parent =
                        path.parent()
                            .filter(|parent| *parent != path)
                            .ok_or_else(|| {
                                namespace_error(std::io::Error::other(
                                    "The cache has no existing parent",
                                ))
                            })?;
                    // Keep each existing parent pinned while creating and binding its child.
                    let (_parent_namespace, _volume_anchor) = if components.len() == 1 {
                        let (anchor, _) = WindowsDirectoryRootProof::open_configured(parent, false)
                            .map_err(namespace_error)?;
                        validate_publication_namespace_directory(anchor.identity_handle(), None)
                            .map_err(namespace_error)?;
                        (None, Some(anchor))
                    } else {
                        (Some(Self::for_operation(parent)?), None)
                    };
                    match std::fs::create_dir(path) {
                        Ok(()) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(namespace_error(error)),
                    }
                    return Self::require_present(path);
                }
                Err(error) => return Err(namespace_error(error)),
            }
            Self::require_present(path)
        }
        #[cfg(not(windows))]
        {
            std::fs::create_dir_all(path).map_err(namespace_error)?;
            Ok(Self {
                requested_root: path.to_path_buf(),
                root: Some(path.to_path_buf()),
            })
        }
    }

    pub(crate) fn observe_existing(path: &Path) -> Result<Self, ScanError> {
        #[cfg(windows)]
        {
            Self::open(path)
        }
        #[cfg(not(windows))]
        {
            let root = match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() => Some(path.to_path_buf()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Ok(_) => {
                    return Err(namespace_error(std::io::Error::other(
                        "The cache is not a directory",
                    )));
                }
                Err(error) => return Err(namespace_error(error)),
            };
            Ok(Self {
                requested_root: path.to_path_buf(),
                root,
            })
        }
    }

    #[cfg(windows)]
    fn require_present(path: &Path) -> Result<Self, ScanError> {
        let namespace = Self::open(path)?;
        if namespace.root.is_none() {
            return Err(namespace_error(std::io::Error::other(
                "The cache disappeared during admission",
            )));
        }
        Ok(namespace)
    }
}

fn namespace_error(error: std::io::Error) -> ScanError {
    ScanError::new(
        "preview_cleanup_namespace_unavailable",
        format!("Could not pin the preview cleanup directory namespace: {error}"),
    )
}

#[cfg(all(test, windows))]
mod tests;
