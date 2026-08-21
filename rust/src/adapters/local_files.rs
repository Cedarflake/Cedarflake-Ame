use std::fs::{self, File, Metadata, ReadDir};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FileIdInfo,
    GetFileInformationByHandleEx,
};

use crate::domain::{
    DiscoveredFile, ExpectedFileState, FileIdentityEvidence, LibraryRootAvailability,
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryPlaceholderState,
    RootAvailabilityEvidence, ScanError, ScanIssue,
};

const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "heic", "heif", "ico", "jpeg", "jpg", "png", "tif", "tiff", "webp",
];

pub enum FileVisitOutcome {
    File(DiscoveredFile),
    Issue(ScanIssue),
    Directory,
    Ignored,
}

pub struct FileVisit {
    pub relative_path: String,
    pub outcome: FileVisitOutcome,
}

pub struct FileDiscovery {
    root: PathBuf,
    canonical_root: PathBuf,
}

pub fn inspect_root_availability(root_path: &str) -> RootAvailabilityEvidence {
    let path = Path::new(root_path);
    match path.symlink_metadata() {
        Ok(metadata) if is_cloud_placeholder(&metadata) => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Offline,
            message: Some("The source root is not locally available".to_owned()),
        },
        Ok(metadata) if metadata.is_dir() => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Available,
            message: None,
        },
        Ok(_) => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Missing,
            message: Some("The stored source path is no longer a directory".to_owned()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Missing,
            message: Some("The source root is missing or its volume is disconnected".to_owned()),
        },
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Inaccessible,
                message: Some(
                    "The source root cannot be accessed with current permissions".to_owned(),
                ),
            }
        }
        Err(error) => RootAvailabilityEvidence {
            availability: LibraryRootAvailability::Inaccessible,
            message: Some(format!("The source root is unavailable: {error}")),
        },
    }
}

pub struct DirectoryEntryPaths {
    root: PathBuf,
    directory_path: PathBuf,
    entries: ReadDir,
    index: u64,
}

pub struct CheckedDirectoryEntryPaths {
    root: PathBuf,
    directory_path: PathBuf,
    entries: ReadDir,
}

impl Iterator for DirectoryEntryPaths {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.next()?;
        let relative_path = match entry {
            Ok(entry) => entry
                .path()
                .strip_prefix(&self.root)
                .map(relative_path_text)
                .unwrap_or_else(|_| path_text(entry.path())),
            Err(_) => {
                let unresolved = self
                    .directory_path
                    .join(format!("<unresolved-entry-{}>", self.index));
                unresolved
                    .strip_prefix(&self.root)
                    .map(relative_path_text)
                    .unwrap_or_else(|_| path_text(unresolved))
            }
        };
        self.index = self.index.saturating_add(1);
        Some(relative_path)
    }
}

impl Iterator for CheckedDirectoryEntryPaths {
    type Item = Result<String, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(|entry| {
            let entry = entry.map_err(|error| ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            entry
                .path()
                .strip_prefix(&self.root)
                .map(relative_path_text)
                .map_err(|_| path_containment_issue(&entry.path()))
        })
    }
}

impl FileDiscovery {
    pub fn new(root_path: &str) -> Result<Self, ScanError> {
        let root = PathBuf::from(root_path);
        if !root.is_absolute() {
            return Err(ScanError::new(
                "root_not_absolute",
                "The library root must be an absolute path",
            ));
        }
        if !root.is_dir() {
            return Err(ScanError::new(
                "root_unavailable",
                "The selected library root is not an available directory",
            ));
        }

        let canonical_root = root.canonicalize().map_err(|error| {
            ScanError::new(
                "root_canonicalization_failed",
                format!("Could not resolve the selected directory: {error}"),
            )
        })?;

        Ok(Self {
            root,
            canonical_root,
        })
    }

    pub fn canonical_root(&self) -> Result<PathBuf, ScanError> {
        Ok(self.canonical_root.clone())
    }

    pub fn entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<DirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
            return Err(path_containment_issue(&directory_path));
        }
        let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_unreadable".to_owned(),
            message: error.to_string(),
        })?;
        Ok(DirectoryEntryPaths {
            root: self.root.clone(),
            directory_path,
            entries,
            index: 0,
        })
    }

    pub fn checked_entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<CheckedDirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
            return Err(path_containment_issue(&directory_path));
        }
        let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_unreadable".to_owned(),
            message: error.to_string(),
        })?;
        Ok(CheckedDirectoryEntryPaths {
            root: self.root.clone(),
            directory_path,
            entries,
        })
    }

    pub fn visit_relative_path(&self, relative_path: &str) -> FileVisit {
        let relative_path = match validated_relative_path(relative_path) {
            Ok(path) => path.to_path_buf(),
            Err(error) => {
                return FileVisit {
                    relative_path: relative_path.to_owned(),
                    outcome: FileVisitOutcome::Issue(error),
                };
            }
        };
        let (path, metadata) = match self.checked_existing_path(&relative_path) {
            Ok(resolved) => resolved,
            Err(issue) => {
                return FileVisit {
                    relative_path: relative_path_text(relative_path),
                    outcome: FileVisitOutcome::Issue(issue),
                };
            }
        };
        let relative_path = relative_path_text(relative_path);
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Ignored,
            };
        }
        if file_type.is_dir() {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Directory,
            };
        }
        if !file_type.is_file() {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Ignored,
            };
        }

        if is_cloud_placeholder(&metadata) {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Issue(ScanIssue {
                    path: Some(path_text(&path)),
                    code: "cloud_placeholder_skipped".to_owned(),
                    message: "The file is not locally available and was not hydrated".to_owned(),
                }),
            };
        }

        if !has_image_extension(&path) {
            match has_supported_magic(&path) {
                Ok(true) => {}
                Ok(false) => {
                    return FileVisit {
                        relative_path,
                        outcome: FileVisitOutcome::Ignored,
                    };
                }
                Err(error) => {
                    return FileVisit {
                        relative_path,
                        outcome: FileVisitOutcome::Issue(ScanIssue {
                            path: Some(path_text(&path)),
                            code: "media_signature_unreadable".to_owned(),
                            message: format!("The file signature could not be read: {error}"),
                        }),
                    };
                }
            }
        }

        let created_unix_ms = created_unix_ms(&metadata);
        let modified_unix_ms = modified_unix_ms(&metadata);
        let (file_identity, issues) = match file_identity(&path) {
            Ok(identity) => (identity, Vec::new()),
            Err(error) => (
                None,
                vec![ScanIssue {
                    path: Some(path_text(&path)),
                    code: "file_identity_unavailable".to_owned(),
                    message: error.to_string(),
                }],
            ),
        };

        FileVisit {
            relative_path: relative_path.clone(),
            outcome: FileVisitOutcome::File(DiscoveredFile {
                absolute_path: path_text(&path),
                relative_path,
                file_size: metadata.len(),
                created_unix_ms,
                modified_unix_ms,
                file_identity,
                issues,
            }),
        }
    }

    pub(crate) fn metadata_inventory_entry(
        &self,
        relative_path: &str,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        let relative_path_value = validated_relative_path(relative_path)?;
        let (path, metadata) = self.checked_existing_path(relative_path_value)?;
        let relative_path = relative_path_text(relative_path_value);
        let is_reparse_point = is_link_or_reparse_point(&metadata);
        let placeholder_state = metadata_placeholder_state(&metadata);
        if is_reparse_point && placeholder_state == MetadataInventoryPlaceholderState::Available {
            match path.metadata() {
                Ok(target_metadata) if target_metadata.is_dir() => {
                    return Err(ScanIssue {
                        path: Some(path_text(path)),
                        code: "metadata_inventory_reparse_directory".to_owned(),
                        message:
                            "The inventory cannot prove descendants through a reparse directory"
                                .to_owned(),
                    });
                }
                Ok(_) => {}
                Err(error) => {
                    return Err(ScanIssue {
                        path: Some(path_text(path)),
                        code: "metadata_inventory_reparse_unverifiable".to_owned(),
                        message: format!(
                            "The inventory could not classify a reparse target safely: {error}"
                        ),
                    });
                }
            }
        }
        let kind = metadata_inventory_entry_kind(
            metadata.is_dir(),
            metadata.is_file(),
            is_reparse_point,
            placeholder_state,
        );
        if kind == MetadataInventoryEntryKind::Directory
            && (is_reparse_point
                || placeholder_state != MetadataInventoryPlaceholderState::Available)
        {
            return Err(ScanIssue {
                path: Some(path_text(path)),
                code: if is_reparse_point {
                    "metadata_inventory_reparse_directory"
                } else {
                    "metadata_inventory_placeholder_directory"
                }
                .to_owned(),
                message: "The inventory cannot prove descendants without traversing this directory"
                    .to_owned(),
            });
        }
        let file_identity = if kind == MetadataInventoryEntryKind::File
            && placeholder_state == MetadataInventoryPlaceholderState::Available
            && !is_reparse_point
        {
            file_identity(&path).ok().flatten()
        } else {
            None
        };
        Ok(MetadataInventoryEntry {
            relative_path,
            kind,
            file_size: (kind == MetadataInventoryEntryKind::File).then_some(metadata.len()),
            modified_unix_ms: modified_unix_ms(&metadata),
            file_identity,
            placeholder_state,
            is_reparse_point,
        })
    }

    pub fn revalidate_relative_file_state(
        &self,
        relative_path: &str,
        expected: &ExpectedFileState,
    ) -> Result<(), ScanIssue> {
        let relative_path = validated_relative_path(relative_path)?;
        let (path, metadata) = self.checked_existing_path(relative_path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(path_containment_issue(&path));
        }
        revalidate_file_state_with_metadata(expected, &path, &metadata)
    }

    fn checked_existing_path(
        &self,
        relative_path: &Path,
    ) -> Result<(PathBuf, Metadata), ScanIssue> {
        let mut current = self.root.clone();
        let mut components = relative_path.components().peekable();
        while let Some(component) = components.next() {
            current.push(component.as_os_str());
            if components.peek().is_none() {
                break;
            }
            match current.symlink_metadata() {
                Ok(metadata) if is_link_or_reparse_point(&metadata) => {
                    return Err(path_containment_issue(&current));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => return Err(path_metadata_issue(&current, error)),
            }
        }
        let metadata = current
            .symlink_metadata()
            .map_err(|error| path_metadata_issue(&current, error))?;
        if !is_link_or_reparse_point(&metadata) {
            let canonical = current
                .canonicalize()
                .map_err(|error| path_metadata_issue(&current, error))?;
            if !canonical.starts_with(&self.canonical_root) {
                return Err(path_containment_issue(&current));
            }
        }
        Ok((current, metadata))
    }
}

fn metadata_inventory_entry_kind(
    is_directory: bool,
    is_regular_file: bool,
    is_reparse_point: bool,
    placeholder_state: MetadataInventoryPlaceholderState,
) -> MetadataInventoryEntryKind {
    if is_directory {
        MetadataInventoryEntryKind::Directory
    } else if (is_regular_file && !is_reparse_point)
        || placeholder_state != MetadataInventoryPlaceholderState::Available
    {
        MetadataInventoryEntryKind::File
    } else {
        MetadataInventoryEntryKind::Other
    }
}

fn path_metadata_issue(path: &Path, error: std::io::Error) -> ScanIssue {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => "file_missing",
        std::io::ErrorKind::PermissionDenied => "file_inaccessible",
        _ => "file_metadata_unreadable",
    };
    ScanIssue {
        path: Some(path_text(path)),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

fn path_containment_issue(path: &Path) -> ScanIssue {
    ScanIssue {
        path: Some(path_text(path)),
        code: "source_path_outside_root".to_owned(),
        message: "The source path crossed a filesystem link outside the selected root".to_owned(),
    }
}

fn is_link_or_reparse_point(metadata: &Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        has_reparse_point_attribute(metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[cfg(windows)]
fn has_reparse_point_attribute(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn validated_relative_path(value: &str) -> Result<&Path, ScanIssue> {
    let path = Path::new(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ScanIssue {
            path: Some(value.to_owned()),
            code: "directory_frontier_invalid".to_owned(),
            message: "The stored directory entry escaped the selected root".to_owned(),
        });
    }
    Ok(path)
}

pub fn revalidate_file_state(expected: &ExpectedFileState) -> Result<(), ScanIssue> {
    let path = Path::new(&expected.absolute_path);
    let metadata = path.symlink_metadata().map_err(|error| ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: "source_revalidation_failed".to_owned(),
        message: error.to_string(),
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(path_containment_issue(path));
    }
    revalidate_file_state_with_metadata(expected, path, &metadata)
}

fn revalidate_file_state_with_metadata(
    expected: &ExpectedFileState,
    path: &Path,
    metadata: &Metadata,
) -> Result<(), ScanIssue> {
    if is_cloud_placeholder(metadata) {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_became_unavailable".to_owned(),
            message: "The file is no longer locally available".to_owned(),
        });
    }
    if metadata.len() != expected.file_size
        || modified_unix_ms(metadata) != expected.modified_unix_ms
    {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_changed_during_scan".to_owned(),
            message: "The file size or modification time changed during the scan".to_owned(),
        });
    }
    if let Some(expected_identity) = &expected.file_identity {
        let actual_identity = file_identity(path).map_err(|error| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if actual_identity.as_ref() != Some(expected_identity) {
            return Err(ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "source_replaced_during_scan".to_owned(),
                message: "The file identity changed during the scan".to_owned(),
            });
        }
    }
    Ok(())
}

#[cfg(windows)]
fn file_identity(path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    let file = OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let mut info = FILE_ID_INFO::default();
    // SAFETY: ADR 0007 fixes the complete contract: the live file owns the handle for this call,
    // `info` is aligned and writable, and the buffer length exactly matches `FILE_ID_INFO`.
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_ID_INFO>()).expect("FILE_ID_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file_id = u128::from_le_bytes(info.FileId.Identifier);
    Ok(Some(FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("{:016x}:{file_id:032x}", info.VolumeSerialNumber),
    }))
}

#[cfg(not(windows))]
fn file_identity(_path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    Ok(None)
}

fn modified_unix_ms(metadata: &Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn created_unix_ms(metadata: &Metadata) -> Option<i64> {
    metadata
        .created()
        .ok()
        .and_then(|created| created.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
}

fn path_text(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

fn relative_path_text(path: impl AsRef<Path>) -> String {
    path_text(path).replace('\\', "/")
}

pub(crate) fn user_visible_path(path: &str) -> String {
    const DEVICE_PREFIX: &str = "\\\\?\\";
    const UNC_DEVICE_PREFIX: &str = "\\\\?\\UNC\\";
    if path
        .get(..UNC_DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(UNC_DEVICE_PREFIX))
    {
        return format!("\\\\{}", &path[UNC_DEVICE_PREFIX.len()..]);
    }
    if path
        .get(..DEVICE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(DEVICE_PREFIX))
    {
        return path[DEVICE_PREFIX.len()..].to_owned();
    }
    path.to_owned()
}

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(false)
}

fn has_supported_magic(path: &Path) -> std::io::Result<bool> {
    let mut header = [0_u8; 16];
    let mut file = File::open(path)?;
    let read_count = file.read(&mut header)?;
    let header = &header[..read_count];

    Ok(header.starts_with(b"\x89PNG\r\n\x1a\n")
        || header.starts_with(b"\xff\xd8\xff")
        || header.starts_with(b"GIF87a")
        || header.starts_with(b"GIF89a")
        || header.starts_with(b"BM")
        || header.starts_with(b"II*\0")
        || header.starts_with(b"MM\0*")
        || header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP")
        || header.get(4..8) == Some(b"ftyp"))
}

#[cfg(windows)]
fn is_cloud_placeholder(metadata: &Metadata) -> bool {
    metadata_placeholder_state(metadata) != MetadataInventoryPlaceholderState::Available
}

#[cfg(all(windows, test))]
fn has_cloud_placeholder_attribute(attributes: u32) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };

    attributes & FILE_ATTRIBUTE_OFFLINE != 0
        || attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0
        || attributes & FILE_ATTRIBUTE_RECALL_ON_OPEN != 0
}

#[cfg(windows)]
fn metadata_placeholder_state(metadata: &Metadata) -> MetadataInventoryPlaceholderState {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };

    let attributes = metadata.file_attributes();
    if attributes & FILE_ATTRIBUTE_OFFLINE != 0 {
        MetadataInventoryPlaceholderState::Offline
    } else if attributes & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0 {
        MetadataInventoryPlaceholderState::RecallOnDataAccess
    } else if attributes & FILE_ATTRIBUTE_RECALL_ON_OPEN != 0 {
        MetadataInventoryPlaceholderState::RecallOnOpen
    } else {
        MetadataInventoryPlaceholderState::Available
    }
}

#[cfg(not(windows))]
fn is_cloud_placeholder(_metadata: &Metadata) -> bool {
    false
}

#[cfg(not(windows))]
fn metadata_placeholder_state(_metadata: &Metadata) -> MetadataInventoryPlaceholderState {
    MetadataInventoryPlaceholderState::Available
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn user_visible_path_hides_windows_device_prefixes() {
        assert_eq!(
            user_visible_path(r"\\?\G:\Pictures\sample.png"),
            r"G:\Pictures\sample.png"
        );
        assert_eq!(
            user_visible_path(r"\\?\UNC\server\share\sample.png"),
            r"\\server\share\sample.png"
        );
        assert_eq!(
            user_visible_path(r"C:\Pictures\sample.png"),
            r"C:\Pictures\sample.png"
        );
    }

    #[test]
    fn discovery_emits_platform_independent_relative_paths() {
        let root = tempdir().expect("source root");
        let nested = root.path().join("album");
        fs::create_dir(&nested).expect("nested directory");
        fs::write(nested.join("photo.png"), b"\x89PNG\r\n\x1a\nfixture").expect("fixture image");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("discovery");
        let relative_path = discovery
            .entry_paths_in_directory("album")
            .expect("directory entries")
            .next()
            .expect("nested image");
        let visit = discovery.visit_relative_path(&relative_path);

        assert_eq!(relative_path, "album/photo.png");
        assert_eq!(visit.relative_path, "album/photo.png");
    }

    #[cfg(windows)]
    #[test]
    fn recognizes_every_windows_cloud_recall_attribute() {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        };

        assert!(has_cloud_placeholder_attribute(FILE_ATTRIBUTE_OFFLINE));
        assert!(has_cloud_placeholder_attribute(
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS,
        ));
        assert!(has_cloud_placeholder_attribute(
            FILE_ATTRIBUTE_RECALL_ON_OPEN,
        ));
        assert!(!has_cloud_placeholder_attribute(0));
    }

    #[cfg(windows)]
    #[test]
    fn recognizes_windows_reparse_points() {
        assert!(has_reparse_point_attribute(FILE_ATTRIBUTE_REPARSE_POINT));
        assert!(!has_reparse_point_attribute(0));
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn intermediate_filesystem_link_cannot_escape_the_discovery_root() {
        let root = tempdir().expect("source root");
        let outside = tempdir().expect("outside directory");
        fs::write(outside.path().join("outside.png"), b"outside bytes").expect("outside fixture");
        let link = root.path().join("linked");
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_dir(outside.path(), &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("create directory link: {error}");
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), &link).expect("create directory link");
        let discovery = FileDiscovery::new(&root.path().to_string_lossy()).expect("file discovery");

        let visit = discovery.visit_relative_path("linked/outside.png");

        let FileVisitOutcome::Issue(issue) = visit.outcome else {
            panic!("an intermediate link must be rejected");
        };
        assert_eq!(issue.code, "source_path_outside_root");

        let explicitly_selected =
            FileDiscovery::new(&link.to_string_lossy()).expect("explicit linked root");
        assert!(matches!(
            explicitly_selected
                .visit_relative_path("outside.png")
                .outcome,
            FileVisitOutcome::File(_)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn offline_file_is_reported_before_image_content_is_opened() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let file_path = directory.path().join("offline.png");
        let original = b"this content must not be decoded";
        fs::write(&file_path, original).expect("fixture write");
        let set_offline = Command::new("attrib.exe")
            .arg("+O")
            .arg(&file_path)
            .status()
            .expect("attrib executable");
        assert!(set_offline.success());

        let discovery =
            FileDiscovery::new(&directory.path().to_string_lossy()).expect("valid discovery root");
        let issues = discovery
            .entry_paths_in_directory("")
            .expect("directory entries")
            .map(|relative_path| discovery.visit_relative_path(&relative_path))
            .filter_map(|visit| match visit.outcome {
                FileVisitOutcome::Issue(issue) => Some(issue),
                _ => None,
            })
            .collect::<Vec<_>>();
        let inventory_entry = discovery
            .metadata_inventory_entry("offline.png")
            .expect("offline metadata inventory entry");

        let clear_offline = Command::new("attrib.exe")
            .arg("-O")
            .arg(&file_path)
            .status()
            .expect("attrib executable");
        assert!(clear_offline.success());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "cloud_placeholder_skipped");
        assert_eq!(
            inventory_entry.placeholder_state,
            MetadataInventoryPlaceholderState::Offline
        );
        assert_eq!(inventory_entry.kind, MetadataInventoryEntryKind::File);
        assert!(inventory_entry.file_identity.is_none());
        assert_eq!(fs::read(file_path).expect("fixture bytes"), original);
    }

    #[test]
    fn reparse_cloud_placeholder_remains_a_present_file_entry() {
        assert_eq!(
            metadata_inventory_entry_kind(
                false,
                false,
                true,
                MetadataInventoryPlaceholderState::RecallOnDataAccess,
            ),
            MetadataInventoryEntryKind::File
        );
        assert_eq!(
            metadata_inventory_entry_kind(
                false,
                false,
                true,
                MetadataInventoryPlaceholderState::Available,
            ),
            MetadataInventoryEntryKind::Other
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_identity_survives_rename_and_distinguishes_a_replacement() {
        let directory = tempdir().expect("temporary directory");
        let original_path = directory.path().join("original.bin");
        let moved_path = directory.path().join("moved.bin");
        fs::write(&original_path, b"same-size").expect("original fixture");
        let original_identity = file_identity(&original_path)
            .expect("original identity")
            .expect("Windows identity");

        fs::rename(&original_path, &moved_path).expect("rename fixture");
        let moved_identity = file_identity(&moved_path)
            .expect("moved identity")
            .expect("Windows identity");
        fs::write(&original_path, b"same-size").expect("replacement fixture");
        let replacement_identity = file_identity(&original_path)
            .expect("replacement identity")
            .expect("Windows identity");

        assert_eq!(moved_identity, original_identity);
        assert_ne!(replacement_identity, original_identity);

        let replacement_metadata = fs::metadata(&original_path).expect("replacement metadata");
        let error = revalidate_file_state(&ExpectedFileState {
            absolute_path: path_text(&original_path),
            file_size: replacement_metadata.len(),
            modified_unix_ms: modified_unix_ms(&replacement_metadata),
            file_identity: Some(original_identity),
        })
        .expect_err("replacement identity must be rejected");
        assert_eq!(error.code, "source_replaced_during_scan");
    }

    #[test]
    fn root_availability_distinguishes_available_and_missing_paths() {
        let directory = tempdir().expect("temporary directory");
        let available = inspect_root_availability(&directory.path().to_string_lossy());
        let missing =
            inspect_root_availability(&directory.path().join("missing").to_string_lossy());

        assert!(matches!(
            available.availability,
            LibraryRootAvailability::Available
        ));
        assert!(available.message.is_none());
        assert!(matches!(
            missing.availability,
            LibraryRootAvailability::Missing
        ));
        assert!(missing.message.is_some());
    }
}
