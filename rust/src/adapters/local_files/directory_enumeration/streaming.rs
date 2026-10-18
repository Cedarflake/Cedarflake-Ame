#[cfg(windows)]
use std::collections::{HashSet, VecDeque};
#[cfg(windows)]
use std::fs::File;
#[cfg(not(windows))]
use std::fs::{self, ReadDir};
#[cfg(windows)]
use std::mem::size_of;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::path::Path;
#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_FILES, ERROR_REPARSE_POINT_ENCOUNTERED};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_LIST_DIRECTORY,
    FILE_READ_ATTRIBUTES, FileIdExtdDirectoryInfo, FileIdExtdDirectoryRestartInfo,
    GetFileInformationByHandleEx,
};

use crate::domain::{FileIdentityEvidence, MetadataInventoryEntry, ScanIssue};

use super::super::{
    FileDiscovery, path_containment_issue, path_text, relative_path_text, validated_relative_path,
};
#[cfg(not(windows))]
use super::super::{
    checked_directory_entry_from_metadata, file_identity, is_link_or_reparse_point,
    metadata_inventory_entry_from_checked_directory_entry, path_metadata_issue,
};
#[cfg(windows)]
use super::super::{
    file_attribute_tag_info_from_handle, file_identity_from_handle, open_root_relative_handle,
};
#[cfg(test)]
use super::instrumentation::{SOURCE_ENUMERATION_COUNTS, wait_for_source_enumeration_permit};
#[cfg(windows)]
use super::records::{directory_enumeration_issue, parse_handle_directory_buffer};

#[cfg(windows)]
const HANDLE_DIRECTORY_BUFFER_BYTES: usize = 64 * 1024;

pub(super) struct StreamingCheckedDirectoryEntryPaths {
    #[cfg(not(windows))]
    root: PathBuf,
    #[cfg(not(windows))]
    directory_path: PathBuf,
    #[cfg(not(windows))]
    entries: ReadDir,
    #[cfg(windows)]
    directory: File,
    #[cfg(windows)]
    directory_relative_path: String,
    #[cfg(windows)]
    buffer: Box<[u64]>,
    #[cfg(windows)]
    pending_entries: VecDeque<Result<MetadataInventoryEntry, ScanIssue>>,
    #[cfg(windows)]
    observed_paths: HashSet<String>,
    #[cfg(windows)]
    volume_serial: u64,
    #[cfg(windows)]
    should_restart: bool,
    #[cfg(windows)]
    enumeration_complete: bool,
    #[cfg(windows)]
    enumeration_failed: bool,
    directory_identity: Option<FileIdentityEvidence>,
    #[cfg(test)]
    test_counter_root: Option<String>,
}

#[cfg(not(windows))]
impl Iterator for StreamingCheckedDirectoryEntryPaths {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref() {
            wait_for_source_enumeration_permit(root_path);
        }
        let entry = self.entries.next()?;
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref()
            && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(root_path)
        {
            counts.entry_reads = counts.entry_reads.saturating_add(1);
        }
        Some(
            entry
                .map_err(|error| ScanIssue {
                    path: Some(path_text(&self.directory_path)),
                    code: "directory_entry_unreadable".to_owned(),
                    message: error.to_string(),
                })
                .and_then(|entry| {
                    let path = entry.path();
                    let relative_path = path
                        .strip_prefix(&self.root)
                        .map(relative_path_text)
                        .map_err(|_| path_containment_issue(&path))?;
                    fs::symlink_metadata(&path)
                        .map_err(|error| ScanIssue {
                            path: Some(path_text(&path)),
                            code: "directory_entry_metadata_unreadable".to_owned(),
                            message: error.to_string(),
                        })
                        .and_then(|metadata| {
                            checked_directory_entry_from_metadata(relative_path, &path, metadata)
                        })
                        .and_then(|entry| {
                            metadata_inventory_entry_from_checked_directory_entry(entry, &path)
                        })
                }),
        )
    }
}

#[cfg(windows)]
impl Iterator for StreamingCheckedDirectoryEntryPaths {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref() {
            wait_for_source_enumeration_permit(root_path);
        }
        if let Some(entry) = self.pending_entries.pop_front() {
            self.record_entry_read();
            return Some(entry);
        }
        if self.enumeration_complete || self.enumeration_failed {
            return None;
        }
        let mut empty_nonterminal_pages = 0_u8;
        loop {
            match self.read_next_handle_page() {
                Ok(()) => {
                    if let Some(entry) = self.pending_entries.pop_front() {
                        self.record_entry_read();
                        return Some(entry);
                    }
                    if self.enumeration_complete {
                        return None;
                    }
                    empty_nonterminal_pages = empty_nonterminal_pages.saturating_add(1);
                    if empty_nonterminal_pages >= 2 {
                        self.enumeration_failed = true;
                        return Some(Err(directory_enumeration_issue(
                            &self.directory_relative_path,
                            "The directory handle returned repeated empty non-terminal pages",
                        )));
                    }
                }
                Err(issue) => {
                    self.enumeration_failed = true;
                    return Some(Err(issue));
                }
            }
        }
    }
}

#[cfg(windows)]
impl StreamingCheckedDirectoryEntryPaths {
    fn record_entry_read(&self) {
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref()
            && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(root_path)
        {
            counts.entry_reads = counts.entry_reads.saturating_add(1);
        }
    }

    fn read_next_handle_page(&mut self) -> Result<(), ScanIssue> {
        self.buffer.fill(0);
        let information_class = if self.should_restart {
            FileIdExtdDirectoryRestartInfo
        } else {
            FileIdExtdDirectoryInfo
        };
        let buffer_size = u32::try_from(self.buffer.len().saturating_mul(size_of::<u64>()))
            .map_err(|_| {
                directory_enumeration_issue(
                    &self.directory_relative_path,
                    "The directory enumeration buffer is too large",
                )
            })?;
        // SAFETY: ADR 0024 owns this adapter-only boundary. `directory` remains live and was
        // opened no-follow with FILE_LIST_DIRECTORY; `buffer` is aligned, initialized, writable,
        // and exactly `buffer_size` bytes long for the duration of the call.
        let result = unsafe {
            GetFileInformationByHandleEx(
                self.directory.as_raw_handle(),
                information_class,
                self.buffer.as_mut_ptr().cast(),
                buffer_size,
            )
        };
        if result == 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                self.enumeration_complete = true;
                return Ok(());
            }
            return Err(ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            });
        }
        self.should_restart = false;
        let byte_len = self.buffer.len().saturating_mul(size_of::<u64>());
        // SAFETY: the initialized `u64` allocation is exposed only as immutable bytes for the
        // safe, bounds-checked parser; the allocation remains alive and is not mutated meanwhile.
        let bytes =
            unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), byte_len) };
        let entries = parse_handle_directory_buffer(
            bytes,
            self.volume_serial,
            &self.directory_relative_path,
        )?;
        for entry in entries {
            if !self.observed_paths.insert(entry.relative_path.clone()) {
                return Err(directory_enumeration_issue(
                    &self.directory_relative_path,
                    "The directory handle returned a duplicate relative path",
                ));
            }
            self.pending_entries.push_back(Ok(entry));
        }
        Ok(())
    }
}

impl StreamingCheckedDirectoryEntryPaths {
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.directory_identity.as_ref()
    }

    #[cfg(not(windows))]
    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        let closing_metadata = self
            .directory_path
            .symlink_metadata()
            .map_err(|error| path_metadata_issue(&self.directory_path, error))?;
        if !closing_metadata.is_dir()
            || (self.directory_path != self.root && is_link_or_reparse_point(&closing_metadata))
        {
            return Err(path_containment_issue(&self.directory_path));
        }
        let closing_identity = file_identity(&self.directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&self.directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if closing_identity != self.directory_identity {
            return Err(ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its durable spool was captured"
                    .to_owned(),
            });
        }
        Ok(closing_identity)
    }

    #[cfg(windows)]
    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        if self.enumeration_failed || !self.enumeration_complete || !self.pending_entries.is_empty()
        {
            return Err(directory_enumeration_issue(
                &self.directory_relative_path,
                "The directory handle enumeration did not reach a trustworthy terminal boundary",
            ));
        }
        let closing_identity =
            file_identity_from_handle(&self.directory).map_err(|error| ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
        if closing_identity != self.directory_identity {
            return Err(ScanIssue {
                path: Some(self.directory_relative_path.clone()),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its durable spool was captured"
                    .to_owned(),
            });
        }
        Ok(closing_identity)
    }
}

impl FileDiscovery {
    pub(super) fn streaming_checked_entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<StreamingCheckedDirectoryEntryPaths, ScanIssue> {
        self.streaming_checked_entry_paths_in_directory_with_hook(relative_directory, || {})
    }

    fn streaming_checked_entry_paths_in_directory_with_hook(
        &self,
        relative_directory: &str,
        after_root_proof: impl FnOnce(),
    ) -> Result<StreamingCheckedDirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        #[cfg(not(windows))]
        {
            let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
            if !metadata.is_dir()
                || (!relative_directory_path.as_os_str().is_empty()
                    && is_link_or_reparse_point(&metadata))
            {
                return Err(path_containment_issue(&directory_path));
            }
            let directory_identity = file_identity(&directory_path).map_err(|error| ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?;
            let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            after_root_proof();
            #[cfg(test)]
            let test_counter_root = {
                let root_path = path_text(&self.root);
                let mut instrumentation = SOURCE_ENUMERATION_COUNTS
                    .lock()
                    .expect("source enumeration instrumentation");
                if let Some(counts) = instrumentation.get_mut(&root_path) {
                    counts.directory_opens = counts.directory_opens.saturating_add(1);
                    Some(root_path)
                } else {
                    None
                }
            };
            Ok(StreamingCheckedDirectoryEntryPaths {
                root: self.root.clone(),
                directory_path,
                entries,
                directory_identity,
                #[cfg(test)]
                test_counter_root,
            })
        }
        #[cfg(windows)]
        {
            let (directory, directory_identity) =
                self.open_handle_anchored_directory(relative_directory_path, after_root_proof)?;
            #[cfg(test)]
            let test_counter_root = {
                let root_path = path_text(&self.root);
                let mut instrumentation = SOURCE_ENUMERATION_COUNTS
                    .lock()
                    .expect("source enumeration instrumentation");
                if let Some(counts) = instrumentation.get_mut(&root_path) {
                    counts.directory_opens = counts.directory_opens.saturating_add(1);
                    Some(root_path)
                } else {
                    None
                }
            };
            Ok(StreamingCheckedDirectoryEntryPaths {
                directory,
                directory_relative_path: relative_path_text(relative_directory_path),
                buffer: vec![0_u64; HANDLE_DIRECTORY_BUFFER_BYTES / size_of::<u64>()]
                    .into_boxed_slice(),
                pending_entries: VecDeque::new(),
                observed_paths: HashSet::new(),
                volume_serial: self.root_proof.volume_serial,
                should_restart: true,
                enumeration_complete: false,
                enumeration_failed: false,
                directory_identity: Some(directory_identity),
                #[cfg(test)]
                test_counter_root,
            })
        }
    }

    #[cfg(windows)]
    pub(in crate::adapters::local_files) fn open_handle_anchored_directory(
        &self,
        relative_directory: &Path,
        after_root_proof: impl FnOnce(),
    ) -> Result<(File, FileIdentityEvidence), ScanIssue> {
        let root_identity =
            file_identity_from_handle(self.root_proof.identity_handle()).map_err(|error| {
                ScanIssue {
                    path: Some(path_text(&self.canonical_root)),
                    code: "root_identity_unavailable".to_owned(),
                    message: error.to_string(),
                }
            })?;
        if root_identity.as_ref() != Some(&self.root_proof.identity) {
            return Err(ScanIssue {
                path: Some(path_text(&self.canonical_root)),
                code: "root_identity_changed".to_owned(),
                message: "The authorized source root identity changed during inventory".to_owned(),
            });
        }
        let expected_path = self.canonical_root.join(relative_directory);
        after_root_proof();
        let directory = open_root_relative_handle(
            &self.root_proof.handle,
            relative_directory,
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES,
            Some(true),
        )
        .map_err(|error| {
            if error.raw_os_error()
                == Some(i32::try_from(ERROR_REPARSE_POINT_ENCOUNTERED).expect("Win32 error fits"))
            {
                path_containment_issue(&expected_path)
            } else {
                ScanIssue {
                    path: Some(path_text(&expected_path)),
                    code: "directory_unreadable".to_owned(),
                    message: error.to_string(),
                }
            }
        })?;
        let info = file_attribute_tag_info_from_handle(&directory).map_err(|error| ScanIssue {
            path: Some(path_text(&expected_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            || info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        {
            return Err(path_containment_issue(&expected_path));
        }
        let identity = file_identity_from_handle(&directory)
            .map_err(|error| ScanIssue {
                path: Some(path_text(&expected_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: error.to_string(),
            })?
            .ok_or_else(|| ScanIssue {
                path: Some(path_text(&expected_path)),
                code: "directory_identity_unavailable".to_owned(),
                message: "The directory handle has no stable Windows file identity".to_owned(),
            })?;
        if relative_directory.as_os_str().is_empty() && identity != self.root_proof.identity {
            return Err(path_containment_issue(&expected_path));
        }
        Ok((directory, identity))
    }
}

#[cfg(all(test, windows))]
mod tests;
