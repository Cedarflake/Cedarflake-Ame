use std::fs::{self, ReadDir};
use std::path::{Path, PathBuf};

use crate::domain::{FileIdentityEvidence, MetadataInventoryEntry, ScanIssue};

use super::{
    CheckedDirectoryEntry, FileDiscovery, FileVisit, PublicationGuardedFileDiscovery,
    checked_directory_entry_from_metadata, file_identity, is_link_or_reparse_point,
    path_containment_issue, path_text, relative_path_text, validated_relative_path,
};
#[cfg(test)]
use instrumentation::SOURCE_ENUMERATION_COUNTS;
use streaming::StreamingCheckedDirectoryEntryPaths;

#[cfg(test)]
pub(super) mod instrumentation;
#[cfg(windows)]
mod records;
mod streaming;

pub(crate) struct CheckedDirectoryEntryPaths {
    entries: Vec<CheckedDirectoryEntryPath>,
    next_index: usize,
    #[cfg(test)]
    directory_identity: Option<FileIdentityEvidence>,
}

struct CheckedDirectoryEntryPath {
    relative_path: String,
    path: PathBuf,
}

pub(crate) struct PublicationGuardedFileVisits<'guard> {
    discovery: &'guard FileDiscovery,
    entries: CheckedDirectoryEntryPaths,
}

pub(crate) struct PublicationGuardedMetadataInventoryEntries {
    entries: StreamingCheckedDirectoryEntryPaths,
    _guard: PublicationGuardedFileDiscovery,
}

impl Iterator for CheckedDirectoryEntryPaths {
    type Item = Result<CheckedDirectoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.get(self.next_index)?;
        self.next_index = self.next_index.saturating_add(1);
        Some(
            fs::symlink_metadata(&entry.path)
                .map_err(|error| ScanIssue {
                    path: Some(path_text(&entry.path)),
                    code: "directory_entry_metadata_unreadable".to_owned(),
                    message: error.to_string(),
                })
                .and_then(|metadata| {
                    checked_directory_entry_from_metadata(
                        entry.relative_path.clone(),
                        &entry.path,
                        metadata,
                    )
                }),
        )
    }
}

impl Iterator for PublicationGuardedFileVisits<'_> {
    type Item = Result<FileVisit, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries
            .next()
            .map(|entry| entry.map(|entry| self.discovery.visit_directory_entry(entry)))
    }
}

impl Iterator for PublicationGuardedMetadataInventoryEntries {
    type Item = Result<MetadataInventoryEntry, ScanIssue>;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next()
    }
}

impl PublicationGuardedMetadataInventoryEntries {
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.entries.directory_identity()
    }

    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        self.entries.finish()
    }
}

impl CheckedDirectoryEntryPaths {
    #[cfg(test)]
    pub(crate) fn directory_identity(&self) -> Option<&FileIdentityEvidence> {
        self.directory_identity.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn seek_after(&mut self, relative_path: &str) {
        self.next_index = self
            .entries
            .partition_point(|entry| entry.relative_path.as_str() <= relative_path);
    }
}

fn stable_checked_directory_entries(
    root: &Path,
    directory_path: &Path,
    entries: ReadDir,
) -> Result<Vec<CheckedDirectoryEntryPath>, ScanIssue> {
    let mut stable_entries = entries
        .map(|entry| {
            let entry = entry.map_err(|error| ScanIssue {
                path: Some(path_text(directory_path)),
                code: "directory_entry_unreadable".to_owned(),
                message: error.to_string(),
            })?;
            let path = entry.path();
            let relative_path = path
                .strip_prefix(root)
                .map(relative_path_text)
                .map_err(|_| path_containment_issue(&path))?;
            Ok(CheckedDirectoryEntryPath {
                relative_path,
                path,
            })
        })
        .collect::<Result<Vec<_>, ScanIssue>>()?;
    stable_entries.sort_unstable_by(|left, right| left.relative_path.cmp(&right.relative_path));
    if stable_entries
        .windows(2)
        .any(|entries| entries[0].relative_path == entries[1].relative_path)
    {
        return Err(ScanIssue {
            path: Some(path_text(directory_path)),
            code: "directory_entry_identity_ambiguous".to_owned(),
            message: "The directory contains paths that cannot be ordered uniquely".to_owned(),
        });
    }
    Ok(stable_entries)
}

impl PublicationGuardedFileDiscovery {
    pub(crate) fn file_visits_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<PublicationGuardedFileVisits<'_>, ScanIssue> {
        Ok(PublicationGuardedFileVisits {
            discovery: &self.discovery,
            entries: self
                .discovery
                .checked_entry_paths_in_directory(relative_directory)?,
        })
    }

    pub(crate) fn streaming_metadata_inventory_entries_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<PublicationGuardedMetadataInventoryEntries, ScanIssue> {
        let guard = self.try_clone_capability().map_err(|error| ScanIssue {
            path: Some(path_text(&self.discovery.canonical_root)),
            code: "root_publication_namespace_guard_clone_failed".to_owned(),
            message: format!("Could not retain the publication guard for enumeration: {error}"),
        })?;
        let entries = guard
            .discovery
            .streaming_checked_entry_paths_in_directory(relative_directory)?;
        Ok(PublicationGuardedMetadataInventoryEntries {
            entries,
            _guard: guard,
        })
    }
}

impl FileDiscovery {
    pub(crate) fn checked_entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<CheckedDirectoryEntryPaths, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
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
        let entries = stable_checked_directory_entries(&self.root, &directory_path, entries)?;
        #[cfg(test)]
        {
            let root_path = path_text(&self.root);
            if let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source enumeration instrumentation")
                .get_mut(&root_path)
            {
                counts.directory_opens = counts.directory_opens.saturating_add(1);
                counts.entry_reads = counts
                    .entry_reads
                    .saturating_add(u64::try_from(entries.len()).unwrap_or(u64::MAX));
            }
        }
        let closing_identity = file_identity(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })?;
        if directory_identity != closing_identity {
            return Err(ScanIssue {
                path: Some(path_text(&directory_path)),
                code: "directory_identity_changed".to_owned(),
                message: "The directory identity changed while its stable frontier was opened"
                    .to_owned(),
            });
        }
        Ok(CheckedDirectoryEntryPaths {
            entries,
            next_index: 0,
            #[cfg(test)]
            directory_identity,
        })
    }

    #[cfg(test)]
    pub(crate) fn metadata_inventory_directory_identity(
        &self,
        relative_directory: &str,
    ) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !metadata.is_dir()
            || (!relative_directory_path.as_os_str().is_empty()
                && is_link_or_reparse_point(&metadata))
        {
            return Err(path_containment_issue(&directory_path));
        }
        file_identity(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_identity_unavailable".to_owned(),
            message: error.to_string(),
        })
    }
}
