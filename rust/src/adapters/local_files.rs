use std::fs::{self, File, Metadata, ReadDir};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

#[cfg(windows)]
use std::fs::OpenOptions;
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

#[cfg(windows)]
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
#[cfg(windows)]
use windows_sys::Win32::Storage::CloudFilters::{
    CF_PLACEHOLDER_STATE_INVALID, CF_PLACEHOLDER_STATE_PARTIAL,
    CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK, CF_PLACEHOLDER_STATE_PLACEHOLDER,
    CfGetPlaceholderStateFromAttributeTag,
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_INFO, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FileAttributeTagInfo, FileIdInfo, FindClose, FindFirstFileW,
    GetFileInformationByHandleEx, WIN32_FIND_DATAW,
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
        Ok(metadata) => match entry_reparse_evidence(path, &metadata) {
            Ok((_, state)) if state != MetadataInventoryPlaceholderState::Available => {
                RootAvailabilityEvidence {
                    availability: LibraryRootAvailability::Offline,
                    message: Some("The source root is not locally available".to_owned()),
                }
            }
            Ok((_, _)) if metadata.is_dir() => RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Available,
                message: None,
            },
            Ok(_) => RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Missing,
                message: Some("The stored source path is no longer a directory".to_owned()),
            },
            Err(error) => RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Inaccessible,
                message: Some(format!(
                    "The source root cannot be classified safely: {error}"
                )),
            },
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

pub(crate) struct CheckedDirectoryEntryPaths {
    root: PathBuf,
    directory_path: PathBuf,
    entries: ReadDir,
}

pub(crate) struct CheckedDirectoryEntry {
    relative_path: String,
    metadata: Metadata,
    reparse_kind: ReparseKind,
    placeholder_state: MetadataInventoryPlaceholderState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReparseKind {
    None,
    CloudFiles,
    Other,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AttributeTagEvidence {
    attributes: u32,
    reparse_tag: u32,
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
    type Item = Result<CheckedDirectoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next().map(|entry| {
            let entry = entry.map_err(|error| ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            let relative_path = entry
                .path()
                .strip_prefix(&self.root)
                .map(relative_path_text)
                .map_err(|_| path_containment_issue(&entry.path()))?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| ScanIssue {
                path: Some(path_text(&path)),
                code: "directory_entry_metadata_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            checked_directory_entry_from_metadata(relative_path, &path, metadata)
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

    pub(crate) fn checked_entry_paths_in_directory(
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
        match checked_directory_entry_from_metadata(relative_path.clone(), &path, metadata) {
            Ok(entry) => self.visit_directory_entry(entry),
            Err(issue) => FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Issue(issue),
            },
        }
    }

    pub(crate) fn visit_directory_entry(&self, entry: CheckedDirectoryEntry) -> FileVisit {
        let relative_path = entry.relative_path;
        let path = self.root.join(Path::new(&relative_path));
        self.visit_relative_path_with_metadata(
            relative_path,
            path,
            entry.metadata,
            entry.reparse_kind,
            entry.placeholder_state,
        )
    }

    fn visit_relative_path_with_metadata(
        &self,
        relative_path: String,
        path: PathBuf,
        metadata: Metadata,
        reparse_kind: ReparseKind,
        placeholder_state: MetadataInventoryPlaceholderState,
    ) -> FileVisit {
        let file_type = metadata.file_type();
        if placeholder_state != MetadataInventoryPlaceholderState::Available {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Issue(ScanIssue {
                    path: Some(path_text(&path)),
                    code: "cloud_placeholder_skipped".to_owned(),
                    message: "The file is not locally available and was not hydrated".to_owned(),
                }),
            };
        }
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
        if reparse_kind == ReparseKind::Other {
            return FileVisit {
                relative_path,
                outcome: FileVisitOutcome::Ignored,
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
        checked_directory_entry_from_metadata(
            relative_path_text(relative_path_value),
            &path,
            metadata,
        )
        .and_then(|entry| self.metadata_inventory_entry_from_directory_entry(entry))
    }

    pub(crate) fn metadata_inventory_entry_from_directory_entry(
        &self,
        entry: CheckedDirectoryEntry,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        let relative_path_value = validated_relative_path(&entry.relative_path)?;
        let path = self.root.join(relative_path_value);
        self.metadata_inventory_entry_from_metadata(
            &entry.relative_path,
            &path,
            &entry.metadata,
            entry.reparse_kind,
            entry.placeholder_state,
        )
    }

    fn metadata_inventory_entry_from_metadata(
        &self,
        relative_path: &str,
        path: &Path,
        metadata: &Metadata,
        reparse_kind: ReparseKind,
        placeholder_state: MetadataInventoryPlaceholderState,
    ) -> Result<MetadataInventoryEntry, ScanIssue> {
        let is_reparse_point = reparse_kind != ReparseKind::None;
        if reparse_kind == ReparseKind::Other {
            return Err(ScanIssue {
                path: Some(path_text(path)),
                code: "metadata_inventory_reparse_directory".to_owned(),
                message: "The inventory cannot prove descendants through this reparse point"
                    .to_owned(),
            });
        }
        let kind =
            metadata_inventory_entry_kind(metadata.is_dir(), metadata.is_file(), reparse_kind);
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
        {
            file_identity(path).ok().flatten()
        } else {
            None
        };
        Ok(MetadataInventoryEntry {
            relative_path: relative_path.to_owned(),
            kind,
            file_size: (kind == MetadataInventoryEntryKind::File).then_some(metadata.len()),
            modified_unix_ms: modified_unix_ms(metadata),
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
        let evidence = checked_directory_entry_from_metadata(
            relative_path_text(relative_path),
            &path,
            metadata,
        )?;
        if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
            return Err(path_containment_issue(&path));
        }
        revalidate_file_state_with_metadata(expected, &path, &evidence)
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

fn checked_directory_entry_from_metadata(
    relative_path: String,
    path: &Path,
    metadata: Metadata,
) -> Result<CheckedDirectoryEntry, ScanIssue> {
    let (reparse_kind, placeholder_state) =
        entry_reparse_evidence(path, &metadata).map_err(|error| ScanIssue {
            path: Some(path_text(path)),
            code: "file_reparse_evidence_unreadable".to_owned(),
            message: error.to_string(),
        })?;
    Ok(CheckedDirectoryEntry {
        relative_path,
        metadata,
        reparse_kind,
        placeholder_state,
    })
}

#[cfg(windows)]
fn entry_reparse_evidence(
    path: &Path,
    metadata: &Metadata,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    let metadata_state = metadata_placeholder_state(metadata);
    if !has_reparse_point_attribute(metadata.file_attributes()) {
        return Ok((ReparseKind::None, metadata_state));
    }
    let validation = open_validation_handle(path)?;
    let evidence = exact_attribute_tag_evidence(path, &validation, metadata.file_attributes())?;
    reparse_evidence_from_attribute_tag(evidence.attributes, evidence.reparse_tag)
}

#[cfg(windows)]
fn reparse_evidence_from_attribute_tag(
    attributes: u32,
    reparse_tag: u32,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    let placeholder_state = metadata_placeholder_state_from_attributes(attributes);
    let cloud_state = cloud_placeholder_state_from_attribute_tag(attributes, reparse_tag)?;
    if cloud_state & CF_PLACEHOLDER_STATE_PLACEHOLDER == 0 {
        return Ok((ReparseKind::Other, placeholder_state));
    }
    let placeholder_state = if cloud_state
        & (CF_PLACEHOLDER_STATE_PARTIAL | CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK)
        != 0
        && placeholder_state == MetadataInventoryPlaceholderState::Available
    {
        MetadataInventoryPlaceholderState::RecallOnDataAccess
    } else {
        placeholder_state
    };
    Ok((ReparseKind::CloudFiles, placeholder_state))
}

#[cfg(not(windows))]
fn entry_reparse_evidence(
    _path: &Path,
    metadata: &Metadata,
) -> std::io::Result<(ReparseKind, MetadataInventoryPlaceholderState)> {
    Ok((
        if metadata.file_type().is_symlink() {
            ReparseKind::Other
        } else {
            ReparseKind::None
        },
        MetadataInventoryPlaceholderState::Available,
    ))
}

fn metadata_inventory_entry_kind(
    is_directory: bool,
    is_regular_file: bool,
    reparse_kind: ReparseKind,
) -> MetadataInventoryEntryKind {
    if is_directory {
        MetadataInventoryEntryKind::Directory
    } else if is_regular_file && reparse_kind != ReparseKind::Other {
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
    let evidence = checked_directory_entry_from_metadata(String::new(), path, metadata)?;
    if !evidence.metadata.is_file() || evidence.reparse_kind == ReparseKind::Other {
        return Err(path_containment_issue(path));
    }
    revalidate_file_state_with_metadata(expected, path, &evidence)
}

fn revalidate_file_state_with_metadata(
    expected: &ExpectedFileState,
    path: &Path,
    evidence: &CheckedDirectoryEntry,
) -> Result<(), ScanIssue> {
    if evidence.placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "source_became_unavailable".to_owned(),
            message: "The file is no longer locally available".to_owned(),
        });
    }
    let metadata = &evidence.metadata;
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
    let file = open_validation_handle(path)?;
    file_identity_from_handle(&file)
}

#[cfg(windows)]
fn file_identity_from_handle(file: &File) -> std::io::Result<Option<FileIdentityEvidence>> {
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

#[cfg(windows)]
fn open_validation_handle(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .access_mode(0)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT,
        )
        .open(path)
}

#[cfg(windows)]
fn file_attribute_tag_info_from_handle(file: &File) -> std::io::Result<FILE_ATTRIBUTE_TAG_INFO> {
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: ADR 0023 fixes this no-follow/no-recall query contract: the live file owns the
    // handle, `info` is aligned and writable, and the exact structure size is passed to Win32.
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&raw mut info).cast(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .expect("FILE_ATTRIBUTE_TAG_INFO size fits u32"),
        )
    };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info)
}

#[cfg(windows)]
fn exact_attribute_tag_evidence(
    path: &Path,
    file: &File,
    known_attributes: u32,
) -> std::io::Result<AttributeTagEvidence> {
    let handle_info = file_attribute_tag_info_from_handle(file)?;
    let enumeration_info = exact_directory_entry_info(path)?;
    combine_attribute_tag_evidence(known_attributes, &handle_info, &enumeration_info)
}

#[cfg(windows)]
fn combine_attribute_tag_evidence(
    known_attributes: u32,
    handle_info: &FILE_ATTRIBUTE_TAG_INFO,
    enumeration_info: &WIN32_FIND_DATAW,
) -> std::io::Result<AttributeTagEvidence> {
    let attributes =
        known_attributes | handle_info.FileAttributes | enumeration_info.dwFileAttributes;
    let enumeration_tag = if has_reparse_point_attribute(enumeration_info.dwFileAttributes) {
        enumeration_info.dwReserved0
    } else {
        0
    };
    if handle_info.ReparseTag != 0
        && enumeration_tag != 0
        && handle_info.ReparseTag != enumeration_tag
    {
        return Err(std::io::Error::other(
            "The source reparse identity changed during classification",
        ));
    }
    Ok(AttributeTagEvidence {
        attributes,
        reparse_tag: if handle_info.ReparseTag != 0 {
            handle_info.ReparseTag
        } else {
            enumeration_tag
        },
    })
}

#[cfg(windows)]
fn exact_directory_entry_info(path: &Path) -> std::io::Result<WIN32_FIND_DATAW> {
    let wide_path = windows_extended_path(path);
    let mut info = WIN32_FIND_DATAW::default();
    // SAFETY: ADR 0023 fixes this exact-name enumeration contract: `wide_path` is a terminated
    // UTF-16 path, `info` is aligned and writable, and the returned search handle is closed once.
    let handle = unsafe { FindFirstFileW(wide_path.as_ptr(), &raw mut info) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `handle` is the live search handle returned by the successful call above and is not
    // used again after this close.
    let close_result = unsafe { FindClose(handle) };
    if close_result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info)
}

#[cfg(windows)]
fn windows_extended_path(path: &Path) -> Vec<u16> {
    let separator = u16::from(b'\\');
    let mut path_units = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit == u16::from(b'/') {
                separator
            } else {
                unit
            }
        })
        .collect::<Vec<_>>();
    let is_device_path =
        path_units.starts_with(&[separator, separator, u16::from(b'?'), separator])
            || path_units.starts_with(&[separator, separator, u16::from(b'.'), separator]);
    if path.is_absolute() && !is_device_path {
        let mut extended = vec![separator, separator, u16::from(b'?'), separator];
        if path_units.starts_with(&[separator, separator]) {
            extended.extend("UNC\\".encode_utf16());
            extended.extend_from_slice(&path_units[2..]);
        } else {
            extended.append(&mut path_units);
        }
        path_units = extended;
    }
    path_units.push(0);
    path_units
}

#[cfg(not(windows))]
fn file_identity(_path: &Path) -> std::io::Result<Option<FileIdentityEvidence>> {
    Ok(None)
}

#[cfg(windows)]
pub(crate) fn open_source_file(path: &Path) -> std::io::Result<File> {
    open_source_file_with_hook(path, || {})
}

#[cfg(windows)]
fn open_source_file_with_hook(
    path: &Path,
    after_validation: impl FnOnce(),
) -> std::io::Result<File> {
    let validation = open_validation_handle(path)?;
    let validation_evidence = exact_attribute_tag_evidence(path, &validation, 0)?;
    validate_source_file_info(&validation_evidence)?;
    let expected_identity = file_identity_from_handle(&validation)?;
    after_validation();
    let source = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_NO_RECALL)
        .open(path)?;
    let source_info = file_attribute_tag_info_from_handle(&source)?;
    validate_source_file_info(&AttributeTagEvidence {
        attributes: source_info.FileAttributes,
        reparse_tag: source_info.ReparseTag,
    })?;
    if file_identity_from_handle(&source)? != expected_identity {
        return Err(std::io::Error::other(
            "The source path changed while it was being opened",
        ));
    }
    Ok(source)
}

#[cfg(windows)]
fn validate_source_file_info(info: &AttributeTagEvidence) -> std::io::Result<()> {
    let attributes = info.attributes;
    let placeholder_state = metadata_placeholder_state_from_attributes(attributes);
    if placeholder_state != MetadataInventoryPlaceholderState::Available {
        return Err(std::io::Error::other(
            "The cloud source content is not locally available",
        ));
    }
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        let cloud_state = cloud_placeholder_state_from_attribute_tag(attributes, info.reparse_tag)?;
        if cloud_state & CF_PLACEHOLDER_STATE_PLACEHOLDER == 0 {
            return Err(std::io::Error::other(
                "The source path is an unsupported reparse point",
            ));
        }
        if cloud_state & (CF_PLACEHOLDER_STATE_PARTIAL | CF_PLACEHOLDER_STATE_PARTIALLY_ON_DISK)
            != 0
        {
            return Err(std::io::Error::other(
                "The cloud source content is not locally available",
            ));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn open_source_file(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

#[cfg(windows)]
fn cloud_placeholder_state_from_attribute_tag(
    attributes: u32,
    reparse_tag: u32,
) -> std::io::Result<u32> {
    // SAFETY: ADR 0023 limits this value-only Cloud Files call to initialized file attributes and
    // a reparse tag returned by `FileAttributeTagInfo` for the same no-follow, no-recall handle.
    let state = unsafe { CfGetPlaceholderStateFromAttributeTag(attributes, reparse_tag) };
    if state == CF_PLACEHOLDER_STATE_INVALID {
        Err(std::io::Error::other(
            "Cloud Files rejected the file attributes and reparse tag",
        ))
    } else {
        Ok(state)
    }
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
    let mut file = open_source_file(path)?;
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
    metadata_placeholder_state_from_attributes(metadata.file_attributes())
}

#[cfg(windows)]
fn metadata_placeholder_state_from_attributes(
    attributes: u32,
) -> MetadataInventoryPlaceholderState {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    };

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
        let automatic_visit = discovery.visit_relative_path("offline.png");
        let guarded_open = open_source_file(&file_path);

        let clear_offline = Command::new("attrib.exe")
            .arg("-O")
            .arg(&file_path)
            .status()
            .expect("attrib executable");
        assert!(clear_offline.success());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, "cloud_placeholder_skipped");
        assert!(guarded_open.is_err());
        assert!(matches!(
            automatic_visit.outcome,
            FileVisitOutcome::Issue(issue) if issue.code == "cloud_placeholder_skipped"
        ));
        assert_eq!(
            inventory_entry.placeholder_state,
            MetadataInventoryPlaceholderState::Offline
        );
        assert_eq!(inventory_entry.kind, MetadataInventoryEntryKind::File);
        assert!(inventory_entry.file_identity.is_none());
        assert_eq!(fs::read(file_path).expect("fixture bytes"), original);
    }

    #[cfg(windows)]
    #[test]
    fn guarded_source_open_rejects_replacement_and_rechecks_availability() {
        use std::process::Command;

        let directory = tempdir().expect("temporary directory");
        let source_path = directory.path().join("source.png");
        fs::write(&source_path, b"source-bytes").expect("source fixture");
        let replaced = open_source_file_with_hook(&source_path, || {
            fs::remove_file(&source_path).expect("remove original fixture");
            fs::write(&source_path, b"replacement").expect("replacement fixture");
        });
        assert!(replaced.is_err());

        fs::write(&source_path, b"source-bytes").expect("restore source fixture");

        let dehydrated = open_source_file_with_hook(&source_path, || {
            let status = Command::new("attrib.exe")
                .arg("+O")
                .arg(&source_path)
                .status()
                .expect("attrib executable");
            assert!(status.success());
        });
        let clear_offline = Command::new("attrib.exe")
            .arg("-O")
            .arg(&source_path)
            .status()
            .expect("attrib executable");
        assert!(clear_offline.success());
        assert!(dehydrated.is_err());
    }

    #[test]
    fn reparse_cloud_placeholder_remains_a_present_file_entry() {
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::CloudFiles),
            MetadataInventoryEntryKind::File
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, false, ReparseKind::CloudFiles),
            MetadataInventoryEntryKind::Other
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::Other),
            MetadataInventoryEntryKind::Other
        );
        assert_eq!(
            metadata_inventory_entry_kind(false, true, ReparseKind::None),
            MetadataInventoryEntryKind::File
        );
    }

    #[cfg(windows)]
    #[test]
    fn cloud_files_tag_distinguishes_hydrated_placeholder_from_other_reparse_points() {
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_RECALL_ON_OPEN;
        use windows_sys::Win32::System::SystemServices::{
            IO_REPARSE_TAG_CLOUD_2, IO_REPARSE_TAG_SYMLINK,
        };

        let cloud = reparse_evidence_from_attribute_tag(
            FILE_ATTRIBUTE_REPARSE_POINT,
            IO_REPARSE_TAG_CLOUD_2,
        )
        .expect("Cloud Files tag state");
        let handle_info = FILE_ATTRIBUTE_TAG_INFO {
            FileAttributes: FILE_ATTRIBUTE_REPARSE_POINT,
            ReparseTag: IO_REPARSE_TAG_CLOUD_2,
        };
        let enumeration_info = WIN32_FIND_DATAW {
            dwFileAttributes: FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_RECALL_ON_OPEN,
            dwReserved0: IO_REPARSE_TAG_CLOUD_2,
            ..WIN32_FIND_DATAW::default()
        };
        let exact_evidence = combine_attribute_tag_evidence(0, &handle_info, &enumeration_info)
            .expect("exact enumeration evidence");
        let recall = reparse_evidence_from_attribute_tag(
            exact_evidence.attributes,
            exact_evidence.reparse_tag,
        )
        .expect("recall Cloud Files tag state");
        let symlink = reparse_evidence_from_attribute_tag(
            FILE_ATTRIBUTE_REPARSE_POINT,
            IO_REPARSE_TAG_SYMLINK,
        )
        .expect("symlink tag state");

        assert_eq!(
            cloud,
            (
                ReparseKind::CloudFiles,
                MetadataInventoryPlaceholderState::Available,
            )
        );
        assert_eq!(
            recall,
            (
                ReparseKind::CloudFiles,
                MetadataInventoryPlaceholderState::RecallOnOpen,
            )
        );
        assert_eq!(
            symlink,
            (
                ReparseKind::Other,
                MetadataInventoryPlaceholderState::Available,
            )
        );
    }

    #[cfg(windows)]
    #[test]
    fn exact_enumeration_uses_an_extended_literal_path() {
        let path = Path::new(r"C:\library\图片\photo.png");
        let wide = windows_extended_path(path);
        let rendered = String::from_utf16(&wide[..wide.len() - 1]).expect("UTF-16 path");

        assert_eq!(rendered, r"\\?\C:\library\图片\photo.png");
        assert_eq!(wide.last(), Some(&0));
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
