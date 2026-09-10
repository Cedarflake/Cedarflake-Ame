#[cfg(test)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::{LazyLock, Mutex};

use crate::domain::{
    FileIdentityEvidence, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryFrontierEntry, MetadataInventoryFrontierState, MetadataInventoryPage,
    MetadataInventoryScope, ScanError,
};
use crate::ports::MetadataInventorySource;

use super::FileDiscovery;
use super::local_files::CheckedDirectoryEntryPaths;

const MAX_PAGE_ENTRIES: u32 = 4_096;
const MAX_DIRECTORY_DEPTH: usize = 1_024;

#[cfg(test)]
static SOURCE_ENUMERATION_COUNTS: LazyLock<Mutex<HashMap<String, (u64, u64)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub(crate) fn reset_source_root_entry_enumeration_count(root_path: &str) {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source-root enumeration counters")
        .insert(root_path.to_owned(), (0, 0));
}

#[cfg(test)]
pub(crate) fn source_root_entry_enumeration_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source-root enumeration counters")
        .get(root_path)
        .map_or(0, |counts| counts.0)
}

#[cfg(test)]
pub(crate) fn source_directory_open_count(root_path: &str) -> u64 {
    SOURCE_ENUMERATION_COUNTS
        .lock()
        .expect("source-root enumeration counters")
        .get(root_path)
        .map_or(0, |counts| counts.1)
}

pub(crate) struct LocalMetadataInventory {
    discovery: FileDiscovery,
    scope_relative_directory: String,
    scope_identity: Option<FileIdentityEvidence>,
    stack: Vec<MetadataInventoryDirectoryFrame>,
    pending_entry: Option<MetadataInventoryEntry>,
    pending_directory: Option<PendingMetadataInventoryDirectory>,
    page_index: u64,
    is_complete: bool,
    #[cfg(test)]
    test_counter_root: Option<String>,
}

struct MetadataInventoryDirectoryFrame {
    relative_directory: String,
    directory_identity: Option<FileIdentityEvidence>,
    entries: CheckedDirectoryEntryPaths,
    resume_after_relative_path: Option<String>,
    enumerated_entry_count: u64,
}

struct PendingMetadataInventoryDirectory {
    relative_directory: String,
    directory_identity: Option<FileIdentityEvidence>,
}

impl LocalMetadataInventory {
    pub fn new(root_path: &str, scope: &MetadataInventoryScope) -> Result<Self, ScanError> {
        let discovery = FileDiscovery::new(root_path)?;
        let mut inventory = Self {
            discovery,
            scope_relative_directory: scope.relative_path().to_owned(),
            scope_identity: None,
            stack: Vec::new(),
            pending_entry: None,
            pending_directory: None,
            page_index: 1,
            is_complete: false,
            #[cfg(test)]
            test_counter_root: SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source-root enumeration counters")
                .contains_key(root_path)
                .then(|| root_path.to_owned()),
        };
        match scope {
            MetadataInventoryScope::Root => inventory.open_directory("", None)?,
            MetadataInventoryScope::Subtree { relative_path } => {
                match inventory.discovery.metadata_inventory_entry(relative_path) {
                    Ok(entry) => {
                        if entry.kind == MetadataInventoryEntryKind::Directory {
                            let directory_identity = inventory
                                .discovery
                                .metadata_inventory_directory_identity(&entry.relative_path)
                                .map_err(issue_error)?;
                            inventory.scope_identity = directory_identity.clone();
                            inventory.pending_directory = Some(PendingMetadataInventoryDirectory {
                                relative_directory: entry.relative_path.clone(),
                                directory_identity,
                            });
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

    #[cfg(test)]
    pub(crate) fn resume(
        root_path: &str,
        scope: &MetadataInventoryScope,
        next_page_index: u64,
        staged_entry_count: u64,
        enumeration_cursor: Option<&str>,
        frontier: &[MetadataInventoryFrontierEntry],
    ) -> Result<Self, ScanError> {
        if next_page_index == 0 {
            return Err(ScanError::new(
                "metadata_inventory_resume_invalid",
                "The durable inventory page cursor is invalid",
            ));
        }
        if next_page_index == 1 {
            if staged_entry_count != 0 || enumeration_cursor.is_some() || !frontier.is_empty() {
                return Err(frontier_invalid());
            }
            return Self::new(root_path, scope);
        }
        if frontier.is_empty() || frontier.len() > MAX_DIRECTORY_DEPTH.saturating_add(1) {
            return Err(frontier_invalid());
        }
        let discovery = FileDiscovery::new(root_path)?;
        let mut inventory = Self {
            discovery,
            scope_relative_directory: scope.relative_path().to_owned(),
            scope_identity: None,
            stack: Vec::new(),
            pending_entry: None,
            pending_directory: None,
            page_index: next_page_index,
            is_complete: false,
            #[cfg(test)]
            test_counter_root: SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source-root enumeration counters")
                .contains_key(root_path)
                .then(|| root_path.to_owned()),
        };
        let mut active_entry_count = 0_u64;
        for (ordinal, durable) in frontier.iter().enumerate() {
            if durable.ordinal != ordinal as u64
                || durable.state == MetadataInventoryFrontierState::Completed
                || inventory.pending_directory.is_some()
            {
                return Err(frontier_invalid());
            }
            active_entry_count = active_entry_count
                .checked_add(durable.enumerated_entry_count)
                .ok_or_else(frontier_invalid)?;
            match durable.state {
                MetadataInventoryFrontierState::Enumerating => {
                    inventory.open_directory(
                        &durable.relative_directory,
                        durable.directory_identity.as_ref(),
                    )?;
                    let frame = inventory.stack.last_mut().ok_or_else(frontier_invalid)?;
                    if let Some(cursor) = durable.resume_after_relative_path.as_deref() {
                        frame.entries.seek_after(cursor);
                    }
                    frame.resume_after_relative_path = durable.resume_after_relative_path.clone();
                    frame.enumerated_entry_count = durable.enumerated_entry_count;
                    if ordinal == 0 {
                        inventory.scope_identity = durable.directory_identity.clone();
                    }
                }
                MetadataInventoryFrontierState::Pending => {
                    if durable.resume_after_relative_path.is_some() || ordinal + 1 != frontier.len()
                    {
                        return Err(frontier_invalid());
                    }
                    inventory.pending_directory = Some(PendingMetadataInventoryDirectory {
                        relative_directory: durable.relative_directory.clone(),
                        directory_identity: durable.directory_identity.clone(),
                    });
                }
                MetadataInventoryFrontierState::Completed => unreachable!(),
            }
        }
        let durable_cursor = inventory
            .stack
            .iter()
            .rev()
            .find_map(|frame| frame.resume_after_relative_path.as_deref())
            .or_else(|| {
                inventory
                    .pending_directory
                    .as_ref()
                    .map(|pending| pending.relative_directory.as_str())
            });
        if active_entry_count > staged_entry_count || durable_cursor != enumeration_cursor {
            return Err(frontier_invalid());
        }
        Ok(inventory)
    }

    fn open_directory(
        &mut self,
        relative_path: &str,
        expected_identity: Option<&FileIdentityEvidence>,
    ) -> Result<(), ScanError> {
        if self.stack.len() >= MAX_DIRECTORY_DEPTH {
            return Err(ScanError::new(
                "metadata_inventory_depth_exceeded",
                "The metadata inventory exceeded its bounded directory depth",
            ));
        }
        #[cfg(test)]
        if let Some(root_path) = self.test_counter_root.as_deref()
            && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                .lock()
                .expect("source-root enumeration counters")
                .get_mut(root_path)
        {
            counts.1 = counts.1.saturating_add(1);
        }
        let entries = self
            .discovery
            .checked_entry_paths_in_directory(relative_path)
            .map_err(issue_error)?;
        let directory_identity = entries.directory_identity().cloned();
        if expected_identity.is_some() && directory_identity.as_ref() != expected_identity {
            return Err(ScanError::new(
                "metadata_inventory_frontier_identity_changed",
                "The directory identity no longer matches the durable inventory frontier",
            ));
        }
        if self.stack.is_empty() && relative_path == self.scope_relative_directory {
            self.scope_identity = directory_identity.clone();
        }
        self.stack.push(MetadataInventoryDirectoryFrame {
            relative_directory: relative_path.to_owned(),
            directory_identity,
            entries,
            resume_after_relative_path: None,
            enumerated_entry_count: 0,
        });
        Ok(())
    }

    fn frontier(
        &self,
        is_complete: bool,
    ) -> Result<Vec<MetadataInventoryFrontierEntry>, ScanError> {
        if is_complete {
            return Ok(vec![MetadataInventoryFrontierEntry {
                ordinal: 0,
                relative_directory: self.scope_relative_directory.clone(),
                state: MetadataInventoryFrontierState::Completed,
                directory_identity: self.scope_identity.clone(),
                resume_after_relative_path: None,
                enumerated_entry_count: 0,
            }]);
        }
        let mut frontier = Vec::with_capacity(
            self.stack
                .len()
                .saturating_add(usize::from(self.pending_directory.is_some())),
        );
        for frame in &self.stack {
            frontier.push(MetadataInventoryFrontierEntry {
                ordinal: u64::try_from(frontier.len()).map_err(|_| frontier_invalid())?,
                relative_directory: frame.relative_directory.clone(),
                state: MetadataInventoryFrontierState::Enumerating,
                directory_identity: frame.directory_identity.clone(),
                resume_after_relative_path: frame.resume_after_relative_path.clone(),
                enumerated_entry_count: frame.enumerated_entry_count,
            });
        }
        if let Some(pending) = &self.pending_directory {
            frontier.push(MetadataInventoryFrontierEntry {
                ordinal: u64::try_from(frontier.len()).map_err(|_| frontier_invalid())?,
                relative_directory: pending.relative_directory.clone(),
                state: MetadataInventoryFrontierState::Pending,
                directory_identity: pending.directory_identity.clone(),
                resume_after_relative_path: None,
                enumerated_entry_count: 0,
            });
        }
        if frontier.is_empty() {
            return Err(frontier_invalid());
        }
        Ok(frontier)
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
                self.open_directory(
                    &directory.relative_directory,
                    directory.directory_identity.as_ref(),
                )?;
                continue;
            }
            let Some(current) = self.stack.last_mut() else {
                self.is_complete = true;
                break;
            };
            let Some(directory_entry) = current.entries.next() else {
                self.stack.pop();
                continue;
            };
            #[cfg(test)]
            if let Some(root_path) = self.test_counter_root.as_deref()
                && let Some(counts) = SOURCE_ENUMERATION_COUNTS
                    .lock()
                    .expect("source-root enumeration counters")
                    .get_mut(root_path)
            {
                counts.0 = counts.0.saturating_add(1);
            }
            let directory_entry = directory_entry.map_err(issue_error)?;
            let entry = self
                .discovery
                .metadata_inventory_entry_from_directory_entry(directory_entry)
                .map_err(issue_error)?;
            current.resume_after_relative_path = Some(entry.relative_path.clone());
            current.enumerated_entry_count = current
                .enumerated_entry_count
                .checked_add(1)
                .ok_or_else(frontier_invalid)?;
            if entry.kind == MetadataInventoryEntryKind::Directory {
                let directory_identity = self
                    .discovery
                    .metadata_inventory_directory_identity(&entry.relative_path)
                    .map_err(issue_error)?;
                self.pending_directory = Some(PendingMetadataInventoryDirectory {
                    relative_directory: entry.relative_path.clone(),
                    directory_identity,
                });
            }
            entries.push(entry);
        }
        let cursor = entries.last().map(|entry| entry.relative_path.clone());
        let is_complete = self.is_complete
            && self.pending_entry.is_none()
            && self.pending_directory.is_none()
            && self.stack.is_empty();
        let page = MetadataInventoryPage {
            page_index: self.page_index,
            entries,
            cursor,
            is_complete,
            frontier: self.frontier(is_complete)?,
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

fn frontier_invalid() -> ScanError {
    ScanError::new(
        "metadata_inventory_frontier_invalid",
        "The durable metadata inventory frontier is invalid",
    )
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

    #[test]
    fn durable_frontier_resumes_at_the_next_page_without_replaying_entries() {
        let source = tempdir().expect("source directory");
        for index in 0..7 {
            fs::write(
                source.path().join(format!("image-{index}.jpg")),
                b"metadata fixture",
            )
            .expect("file fixture");
        }
        let root = source.path().to_string_lossy();
        let scope = MetadataInventoryScope::Root;
        let cancellation = AtomicBool::new(false);
        let mut original =
            LocalMetadataInventory::new(&root, &scope).expect("original inventory source");
        let first = original
            .next_page(2, &cancellation)
            .expect("first durable page");
        let expected = original
            .next_page(2, &cancellation)
            .expect("expected resumed page");

        let mut resumed = LocalMetadataInventory::resume(
            &root,
            &scope,
            2,
            u64::try_from(first.entries.len()).expect("staged count"),
            first.cursor.as_deref(),
            &first.frontier,
        )
        .expect("resume durable frontier");
        let actual = resumed.next_page(2, &cancellation).expect("resumed page");

        assert_eq!(actual, expected);
        assert_eq!(actual.page_index, 2);
    }

    #[test]
    fn resume_fails_closed_when_the_durable_frontier_does_not_match() {
        let source = tempdir().expect("source directory");
        fs::write(source.path().join("image.jpg"), b"metadata fixture").expect("file fixture");
        let result = LocalMetadataInventory::resume(
            &source.path().to_string_lossy(),
            &MetadataInventoryScope::Root,
            2,
            2,
            Some("image.jpg"),
            &[MetadataInventoryFrontierEntry::enumerating(
                0,
                "",
                None,
                Some("different.jpg".to_owned()),
                1,
            )],
        );
        let error = match result {
            Ok(_) => panic!("mismatched durable frontier must fail closed"),
            Err(error) => error,
        };

        assert_eq!(error.code, "metadata_inventory_frontier_invalid");
    }

    #[cfg(windows)]
    #[test]
    fn resume_fails_closed_when_a_pending_directory_identity_changes() {
        let source = tempdir().expect("source directory");
        fs::create_dir(source.path().join("album")).expect("album directory");
        fs::write(source.path().join("album").join("old.jpg"), b"old").expect("old fixture");
        let root = source.path().to_string_lossy();
        let scope = MetadataInventoryScope::Root;
        let mut inventory = LocalMetadataInventory::new(&root, &scope).expect("inventory source");
        let first = inventory
            .next_page(1, &AtomicBool::new(false))
            .expect("pending-directory page");
        assert_eq!(first.entries[0].relative_path, "album");
        drop(inventory);
        fs::rename(source.path().join("album"), source.path().join("album-old"))
            .expect("replace old album");
        fs::create_dir(source.path().join("album")).expect("replacement album");
        fs::write(source.path().join("album").join("new.jpg"), b"new")
            .expect("replacement fixture");

        let mut resumed = LocalMetadataInventory::resume(
            &root,
            &scope,
            2,
            1,
            first.cursor.as_deref(),
            &first.frontier,
        )
        .expect("restore durable frames");
        let error = resumed
            .next_page(1, &AtomicBool::new(false))
            .expect_err("replacement directory must fail closed");

        assert_eq!(error.code, "metadata_inventory_frontier_identity_changed");
    }

    #[cfg(windows)]
    #[test]
    fn reparse_directory_is_a_non_traversed_leaf_without_blocking_siblings() {
        let source = tempdir().expect("source directory");
        let outside = tempdir().expect("outside directory");
        fs::write(outside.path().join("hidden.png"), b"outside bytes").expect("outside fixture");
        fs::write(source.path().join("visible.png"), b"visible bytes").expect("visible fixture");
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

        let page = inventory
            .next_page(32, &AtomicBool::new(false))
            .expect("reparse boundary page");

        assert!(page.is_complete);
        assert!(page.entries.iter().any(|entry| {
            entry.relative_path == "linked"
                && entry.kind == MetadataInventoryEntryKind::Other
                && entry.is_reparse_point
        }));
        assert!(
            page.entries
                .iter()
                .any(|entry| entry.relative_path == "visible.png")
        );
        assert!(
            !page
                .entries
                .iter()
                .any(|entry| entry.relative_path == "linked/hidden.png")
        );
    }
}
