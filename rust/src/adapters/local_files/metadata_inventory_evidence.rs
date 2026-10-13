use std::path::Path;

#[cfg(windows)]
use super::file_source_evidence;
use super::{
    CheckedDirectoryEntry, ReparseKind, file_identity, metadata_inventory_entry_kind,
    modified_unix_ms,
};
use crate::domain::{
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryPlaceholderState,
    ScanIssue,
};

pub(super) fn metadata_inventory_entry_from_checked_directory_entry(
    entry: CheckedDirectoryEntry,
    path: &Path,
) -> Result<MetadataInventoryEntry, ScanIssue> {
    let is_reparse_point = entry.reparse_kind != ReparseKind::None;
    let is_opaque_directory = entry.metadata.is_dir()
        && (is_reparse_point
            || entry.placeholder_state != MetadataInventoryPlaceholderState::Available);
    let kind = if entry.reparse_kind == ReparseKind::Other || is_opaque_directory {
        MetadataInventoryEntryKind::Other
    } else {
        metadata_inventory_entry_kind(
            entry.metadata.is_dir(),
            entry.metadata.is_file(),
            entry.reparse_kind,
        )
    };
    let (file_identity, source_revision) = if kind == MetadataInventoryEntryKind::File
        && entry.placeholder_state == MetadataInventoryPlaceholderState::Available
    {
        #[cfg(windows)]
        {
            file_source_evidence(path)
                .map(|(identity, revision)| (identity, Some(revision)))
                .unwrap_or((None, None))
        }
        #[cfg(not(windows))]
        {
            (file_identity(path).ok().flatten(), None)
        }
    } else if kind == MetadataInventoryEntryKind::Directory {
        (file_identity(path).ok().flatten(), None)
    } else {
        (None, None)
    };
    Ok(MetadataInventoryEntry {
        relative_path: entry.relative_path,
        kind,
        file_size: (kind == MetadataInventoryEntryKind::File).then_some(entry.metadata.len()),
        modified_unix_ms: modified_unix_ms(&entry.metadata),
        file_identity,
        source_revision,
        placeholder_state: entry.placeholder_state,
        is_reparse_point,
    })
}

#[cfg(all(windows, test))]
mod tests;
