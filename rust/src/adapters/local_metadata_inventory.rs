use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::{
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryPage,
    MetadataInventoryScope, ScanError,
};
use crate::ports::MetadataInventorySource;

use super::FileDiscovery;
use super::local_files::CheckedDirectoryEntryPaths;

const MAX_PAGE_ENTRIES: u32 = 4_096;
const MAX_DIRECTORY_DEPTH: usize = 1_024;

pub struct LocalMetadataInventory {
    discovery: FileDiscovery,
    stack: Vec<CheckedDirectoryEntryPaths>,
    pending_entry: Option<MetadataInventoryEntry>,
    pending_directory: Option<String>,
    page_index: u64,
    is_complete: bool,
}

impl LocalMetadataInventory {
    pub fn new(root_path: &str, scope: &MetadataInventoryScope) -> Result<Self, ScanError> {
        let discovery = FileDiscovery::new(root_path)?;
        let mut inventory = Self {
            discovery,
            stack: Vec::new(),
            pending_entry: None,
            pending_directory: None,
            page_index: 1,
            is_complete: false,
        };
        match scope {
            MetadataInventoryScope::Root => inventory.open_directory("")?,
            MetadataInventoryScope::Subtree { relative_path } => {
                match inventory.discovery.metadata_inventory_entry(relative_path) {
                    Ok(entry) => {
                        if entry.kind == MetadataInventoryEntryKind::Directory {
                            inventory.pending_directory = Some(entry.relative_path.clone());
                        }
                        inventory.pending_entry = Some(entry);
                    }
                    Err(issue) if issue.code == "file_missing" => inventory.is_complete = true,
                    Err(issue) => return Err(issue_error(issue)),
                }
            }
        }
        Ok(inventory)
    }

    fn open_directory(&mut self, relative_path: &str) -> Result<(), ScanError> {
        if self.stack.len() >= MAX_DIRECTORY_DEPTH {
            return Err(ScanError::new(
                "metadata_inventory_depth_exceeded",
                "The metadata inventory exceeded its bounded directory depth",
            ));
        }
        let entries = self
            .discovery
            .checked_entry_paths_in_directory(relative_path)
            .map_err(issue_error)?;
        self.stack.push(entries);
        Ok(())
    }
}

impl MetadataInventorySource for LocalMetadataInventory {
    fn next_page(
        &mut self,
        max_entries: u32,
        cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        if max_entries == 0 || max_entries > MAX_PAGE_ENTRIES {
            return Err(ScanError::new(
                "metadata_inventory_page_limit_invalid",
                "Metadata inventory pages must contain between 1 and 4096 entries",
            ));
        }
        if self.page_index > 1
            && self.is_complete
            && self.pending_entry.is_none()
            && self.stack.is_empty()
        {
            return Err(ScanError::new(
                "metadata_inventory_source_complete",
                "The metadata inventory source has already completed",
            ));
        }
        let mut entries = Vec::with_capacity(max_entries as usize);
        while entries.len() < max_entries as usize {
            if cancelled.load(Ordering::Relaxed) {
                return Err(ScanError::new(
                    "metadata_inventory_cancelled",
                    "The metadata inventory was cancelled",
                ));
            }
            if let Some(entry) = self.pending_entry.take() {
                entries.push(entry);
                continue;
            }
            if let Some(directory) = self.pending_directory.take() {
                self.open_directory(&directory)?;
                continue;
            }
            let Some(current) = self.stack.last_mut() else {
                self.is_complete = true;
                break;
            };
            let Some(directory_entry) = current.next() else {
                self.stack.pop();
                continue;
            };
            let directory_entry = directory_entry.map_err(issue_error)?;
            let entry = self
                .discovery
                .metadata_inventory_entry_from_directory_entry(directory_entry)
                .map_err(issue_error)?;
            if entry.kind == MetadataInventoryEntryKind::Directory {
                self.pending_directory = Some(entry.relative_path.clone());
            }
            entries.push(entry);
        }
        let cursor = entries.last().map(|entry| entry.relative_path.clone());
        let page = MetadataInventoryPage {
            page_index: self.page_index,
            entries,
            cursor,
            is_complete: self.is_complete
                && self.pending_entry.is_none()
                && self.pending_directory.is_none()
                && self.stack.is_empty(),
        };
        self.page_index = self.page_index.checked_add(1).ok_or_else(|| {
            ScanError::new(
                "metadata_inventory_page_overflow",
                "The metadata inventory page counter overflowed",
            )
        })?;
        Ok(page)
    }
}

fn issue_error(issue: crate::domain::ScanIssue) -> ScanError {
    ScanError::new(issue.code, issue.message)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::AtomicBool;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn inventory_pages_include_all_entry_kinds_without_media_filtering() {
        let source = tempdir().expect("source directory");
        fs::create_dir(source.path().join("album")).expect("album directory");
        fs::write(source.path().join("notes.txt"), b"metadata only").expect("text fixture");
        fs::write(
            source.path().join("album").join("image.unknown"),
            b"not decoded",
        )
        .expect("unknown fixture");
        let mut inventory = LocalMetadataInventory::new(
            &source.path().to_string_lossy(),
            &MetadataInventoryScope::Root,
        )
        .expect("inventory source");
        let cancellation = AtomicBool::new(false);
        let mut entries = Vec::new();
        loop {
            let page = inventory
                .next_page(2, &cancellation)
                .expect("bounded metadata page");
            assert!(page.entries.len() <= 2);
            entries.extend(page.entries);
            if page.is_complete {
                break;
            }
        }

        assert!(entries.iter().any(|entry| {
            entry.relative_path == "album" && entry.kind == MetadataInventoryEntryKind::Directory
        }));
        assert!(entries.iter().any(|entry| {
            entry.relative_path == "album/image.unknown"
                && entry.kind == MetadataInventoryEntryKind::File
        }));
        assert!(entries.iter().any(|entry| {
            entry.relative_path == "notes.txt" && entry.kind == MetadataInventoryEntryKind::File
        }));
    }

    #[test]
    fn missing_subtree_produces_one_complete_empty_page() {
        let source = tempdir().expect("source directory");
        let mut inventory = LocalMetadataInventory::new(
            &source.path().to_string_lossy(),
            &MetadataInventoryScope::Subtree {
                relative_path: "removed".to_owned(),
            },
        )
        .expect("missing subtree inventory");
        let cancellation = AtomicBool::new(false);

        let page = inventory
            .next_page(32, &cancellation)
            .expect("complete empty page");

        assert!(page.entries.is_empty());
        assert!(page.cursor.is_none());
        assert!(page.is_complete);
        assert_eq!(page.page_index, 1);
        assert_eq!(
            inventory
                .next_page(32, &cancellation)
                .expect_err("completed source")
                .code,
            "metadata_inventory_source_complete"
        );
    }

    #[cfg(windows)]
    #[test]
    fn terminal_reparse_directory_blocks_complete_inventory_authority() {
        let source = tempdir().expect("source directory");
        let outside = tempdir().expect("outside directory");
        fs::write(outside.path().join("hidden.png"), b"outside bytes").expect("outside fixture");
        let link = source.path().join("linked");
        if let Err(error) = std::os::windows::fs::symlink_dir(outside.path(), &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("create directory link: {error}");
        }
        let mut inventory = LocalMetadataInventory::new(
            &source.path().to_string_lossy(),
            &MetadataInventoryScope::Root,
        )
        .expect("inventory source");

        let error = inventory
            .next_page(32, &AtomicBool::new(false))
            .expect_err("reparse directory must block complete authority");

        assert_eq!(error.code, "metadata_inventory_reparse_directory");
    }
}
