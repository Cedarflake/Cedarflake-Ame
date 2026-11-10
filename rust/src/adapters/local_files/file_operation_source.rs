use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_OPEN_NO_RECALL, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
    FILE_READ_DATA, FILE_SHARE_DELETE, FILE_SHARE_READ,
};

use crate::domain::{ExpectedFileState, FileIdentityEvidence, MetadataInventoryPlaceholderState};

use super::{PublicationGuardedFileDiscovery, revalidate_open_preview_source};

enum OperationSourceAccess {
    Read,
    Mutate,
    Recycle,
}

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
        let access = if for_move {
            OperationSourceAccess::Mutate
        } else {
            OperationSourceAccess::Read
        };
        self.open_guarded_operation_source(name, expected, access)
    }

    pub(crate) fn open_recycling_source(
        &self,
        name: &str,
        expected: &ExpectedFileState,
    ) -> std::io::Result<File> {
        self.open_guarded_operation_source(name, expected, OperationSourceAccess::Recycle)
    }

    fn open_guarded_operation_source(
        &self,
        name: &str,
        expected: &ExpectedFileState,
        access: OperationSourceAccess,
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
        let (mutation_access, sharing) = match access {
            OperationSourceAccess::Read => (0, FILE_SHARE_READ),
            OperationSourceAccess::Mutate => (DELETE, FILE_SHARE_READ),
            // Shell needs delete sharing. This blocks content writes, not same-name replacement.
            OperationSourceAccess::Recycle => (0, FILE_SHARE_READ | FILE_SHARE_DELETE),
        };
        let file = OpenOptions::new()
            .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | mutation_access)
            .share_mode(sharing)
            .custom_flags(FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?;
        revalidate_open_preview_source(&file, expected)
            .map_err(|issue| std::io::Error::other(issue.message))?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{file_source_evidence, modified_unix_ms, path_text};
    use super::*;
    use std::fs;

    #[test]
    fn recycling_source_blocks_writes_and_rejects_stale_identity_without_blocking_rename() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("selected.txt");
        fs::write(&source, b"selected content").unwrap();
        let metadata = source.metadata().unwrap();
        let (file_identity, revision) = file_source_evidence(&source).unwrap();
        let expected = ExpectedFileState {
            absolute_path: path_text(&source),
            file_size: metadata.len(),
            modified_unix_ms: modified_unix_ms(&metadata),
            file_identity,
            source_revision: Some(revision),
        };
        let guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
            &directory.path().to_string_lossy(),
            None,
        )
        .unwrap();
        let writer = OpenOptions::new().write(true).open(&source).unwrap();
        assert_eq!(
            guard
                .open_recycling_source("selected.txt", &expected)
                .unwrap_err()
                .raw_os_error(),
            Some(32)
        );
        drop(writer);
        let held = guard
            .open_recycling_source("selected.txt", &expected)
            .unwrap();
        assert_eq!(
            OpenOptions::new()
                .write(true)
                .open(&source)
                .unwrap_err()
                .raw_os_error(),
            Some(32)
        );
        let renamed = directory.path().join("renamed.txt");
        fs::rename(&source, &renamed).unwrap();
        fs::write(&source, b"replacement").unwrap();
        assert!(
            guard
                .open_recycling_source("selected.txt", &expected)
                .is_err()
        );
        assert_eq!(fs::read(&source).unwrap(), b"replacement");
        assert_eq!(fs::read(&renamed).unwrap(), b"selected content");
        drop(held);
        fs::write(&renamed, b"write after release").unwrap();
    }
}
