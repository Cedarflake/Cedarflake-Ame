use std::fs::{self, ReadDir};
use std::path::PathBuf;

use crate::domain::{FileIdentityEvidence, ScanIssue};

use super::{
    FileDiscovery, is_link_or_reparse_point, path_containment_issue, path_text, relative_path_text,
    validated_relative_path,
};

pub struct DirectoryEntryPaths<'discovery> {
    discovery: &'discovery FileDiscovery,
    relative_directory: String,
    directory_path: PathBuf,
    entries: ReadDir,
    index: u64,
    exhausted: bool,
    failed: bool,
    #[cfg(windows)]
    identity: FileIdentityEvidence,
    #[cfg(windows)]
    _directory: std::fs::File,
}

impl FileDiscovery {
    pub fn entry_paths_in_directory(
        &self,
        relative_directory: &str,
    ) -> Result<DirectoryEntryPaths<'_>, ScanIssue> {
        let relative_directory_path = validated_relative_path(relative_directory)?;
        let (directory_path, metadata) = self.checked_existing_path(relative_directory_path)?;
        if !relative_directory_path.as_os_str().is_empty() && is_link_or_reparse_point(&metadata) {
            return Err(path_containment_issue(&directory_path));
        }
        #[cfg(windows)]
        let (directory, identity) =
            self.open_handle_anchored_directory(relative_directory_path, || {})?;
        let entries = fs::read_dir(&directory_path).map_err(|error| ScanIssue {
            path: Some(path_text(&directory_path)),
            code: "directory_unreadable".to_owned(),
            message: error.to_string(),
        })?;
        Ok(DirectoryEntryPaths {
            discovery: self,
            relative_directory: relative_directory.to_owned(),
            directory_path,
            entries,
            index: 0,
            exhausted: false,
            failed: false,
            #[cfg(windows)]
            identity,
            #[cfg(windows)]
            _directory: directory,
        })
    }
}

impl DirectoryEntryPaths<'_> {
    pub(crate) fn finish(self) -> Result<Option<FileIdentityEvidence>, ScanIssue> {
        if !self.exhausted {
            return Err(ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_enumeration_incomplete".to_owned(),
                message: "A partial enumeration cannot establish a directory identity binding"
                    .to_owned(),
            });
        }
        if self.failed {
            return Err(ScanIssue {
                path: Some(path_text(&self.directory_path)),
                code: "directory_enumeration_failed".to_owned(),
                message: "An unread directory entry prevents complete namespace capture".to_owned(),
            });
        }
        let relative = validated_relative_path(&self.relative_directory)?;
        #[cfg(windows)]
        {
            let (_closing_handle, closing_identity) = self
                .discovery
                .open_handle_anchored_directory(relative, || {})?;
            if self.identity != closing_identity {
                return Err(ScanIssue {
                    path: Some(path_text(&self.directory_path)),
                    code: "directory_identity_changed".to_owned(),
                    message: "The directory binding changed while its entries were enumerated"
                        .to_owned(),
                });
            }
            Ok(Some(self.identity))
        }
        #[cfg(not(windows))]
        {
            self.discovery.checked_existing_path(relative)?;
            Ok(None)
        }
    }
}

impl Iterator for DirectoryEntryPaths<'_> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(entry) = self.entries.next() else {
            self.exhausted = true;
            return None;
        };
        let relative_path = match entry {
            Ok(entry) => entry
                .path()
                .strip_prefix(&self.discovery.root)
                .map(relative_path_text)
                .unwrap_or_else(|_| path_text(entry.path())),
            Err(_) => {
                self.failed = true;
                let unresolved = self
                    .directory_path
                    .join(format!("<unresolved-entry-{}>", self.index));
                unresolved
                    .strip_prefix(&self.discovery.root)
                    .map(relative_path_text)
                    .unwrap_or_else(|_| path_text(unresolved))
            }
        };
        self.index = self.index.saturating_add(1);
        Some(relative_path)
    }
}

#[cfg(all(test, windows))]
mod tests;
