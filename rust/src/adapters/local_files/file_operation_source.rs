use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
    FILE_READ_DATA, FILE_SHARE_READ,
};

use crate::domain::{ExpectedFileState, FileIdentityEvidence, MetadataInventoryPlaceholderState};

use super::{PublicationGuardedFileDiscovery, revalidate_open_preview_source};

pub(crate) fn operation_file_identity(file: &File) -> std::io::Result<FileIdentityEvidence> {
    super::file_identity_from_handle(file)?
        .ok_or_else(|| std::io::Error::other("The file identity is unavailable"))
}

impl PublicationGuardedFileDiscovery {
    pub(crate) fn operation_path(&self, name: &str) -> std::path::PathBuf {
        self.discovery.canonical_root.join(name)
    }

    pub(crate) fn open_operation_source(
        &self,
        name: &str,
        expected: &ExpectedFileState,
        for_move: bool,
    ) -> std::io::Result<File> {
        self.revalidate_relative_file_state(name, expected)
            .map_err(|issue| std::io::Error::other(issue.message))?;
        let entry = self
            .metadata_inventory_entry(name)
            .map_err(|issue| std::io::Error::other(issue.message))?;
        if entry.placeholder_state != MetadataInventoryPlaceholderState::Available {
            return Err(std::io::Error::other("The source is not locally available"));
        }
        let path = self.operation_path(name);
        if Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name) {
            return Err(std::io::Error::other("The source must be one filename"));
        }
        // The namespace is pinned; this handle denies writes/deletion while CopyFile2 reads,
        // or carries DELETE access for a rename of this exact file through its own handle.
        let file = OpenOptions::new()
            .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | if for_move { DELETE } else { 0 })
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        revalidate_open_preview_source(&file, expected)
            .map_err(|issue| std::io::Error::other(issue.message))?;
        Ok(file)
    }
}
