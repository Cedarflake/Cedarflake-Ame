use std::fs::Metadata;
use std::path::{Path, PathBuf};

use crate::domain::{
    DiscoveredFile, FileIdentityEvidence, MetadataInventoryPlaceholderState, ScanIssue,
};

#[cfg(windows)]
use super::file_source_evidence;
use super::media_signature::{has_image_extension, has_supported_magic_from_reader};
use super::{FileDiscovery, ReparseKind, created_unix_ms, modified_unix_ms, path_text};
#[cfg(not(windows))]
use super::{file_identity, open_source_file};

pub enum FileVisitOutcome {
    File(DiscoveredFile),
    TerminalMedia {
        file: DiscoveredFile,
        issue: ScanIssue,
        report_issue: bool,
    },
    RetryableFile {
        file: DiscoveredFile,
        issue: ScanIssue,
    },
    Issue(ScanIssue),
    Directory,
    Ignored,
}

pub struct FileVisit {
    pub relative_path: String,
    pub outcome: FileVisitOutcome,
}

impl FileDiscovery {
    pub(super) fn visit_relative_path_with_metadata(
        &self,
        relative_path: String,
        path: PathBuf,
        metadata: Metadata,
        reparse_kind: ReparseKind,
        placeholder_state: MetadataInventoryPlaceholderState,
        known_identity: Option<FileIdentityEvidence>,
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
            match self.has_supported_magic_for_relative_path(&relative_path, &path) {
                Ok(true) => {}
                Ok(false) => {
                    let file = self.discovered_file(
                        &relative_path,
                        &path,
                        &metadata,
                        known_identity.clone(),
                    );
                    return FileVisit {
                        relative_path,
                        outcome: FileVisitOutcome::TerminalMedia {
                            file,
                            issue: ScanIssue {
                                path: Some(path_text(&path)),
                                code: "media_type_unsupported".to_owned(),
                                message: "The file is not a supported image".to_owned(),
                            },
                            report_issue: false,
                        },
                    };
                }
                Err(error) => {
                    let file =
                        self.discovered_file(&relative_path, &path, &metadata, known_identity);
                    return FileVisit {
                        relative_path,
                        outcome: FileVisitOutcome::RetryableFile {
                            file,
                            issue: ScanIssue {
                                path: Some(path_text(&path)),
                                code: "media_signature_unreadable".to_owned(),
                                message: format!("The file signature could not be read: {error}"),
                            },
                        },
                    };
                }
            }
        }

        let file = self.discovered_file(&relative_path, &path, &metadata, known_identity);

        FileVisit {
            relative_path,
            outcome: FileVisitOutcome::File(file),
        }
    }

    fn has_supported_magic_for_relative_path(
        &self,
        relative_path: &str,
        absolute_path: &Path,
    ) -> std::io::Result<bool> {
        #[cfg(windows)]
        {
            let _ = absolute_path;
            let mut file = self.open_pinned_source_file(relative_path)?;
            has_supported_magic_from_reader(&mut file)
        }
        #[cfg(not(windows))]
        {
            let mut file = open_source_file(absolute_path, &self.canonical_root)?;
            has_supported_magic_from_reader(&mut file)
        }
    }

    fn discovered_file(
        &self,
        relative_path: &str,
        path: &Path,
        metadata: &Metadata,
        known_identity: Option<FileIdentityEvidence>,
    ) -> DiscoveredFile {
        #[cfg(windows)]
        let evidence = file_source_evidence(path)
            .map(|(identity, revision)| (known_identity.or(identity), Some(revision)));
        #[cfg(not(windows))]
        let evidence = known_identity
            .map_or_else(|| file_identity(path), |identity| Ok(Some(identity)))
            .map(|identity| (identity, None));
        let (file_identity, source_revision, issues) = match evidence {
            Ok((identity, revision)) => (identity, revision, Vec::new()),
            Err(error) => (
                None,
                None,
                vec![ScanIssue {
                    path: Some(path_text(path)),
                    code: "source_revision_unavailable".to_owned(),
                    message: error.to_string(),
                }],
            ),
        };

        DiscoveredFile {
            source_root_path: path_text(&self.canonical_root),
            absolute_path: path_text(path),
            relative_path: relative_path.to_owned(),
            file_size: metadata.len(),
            created_unix_ms: created_unix_ms(metadata),
            modified_unix_ms: modified_unix_ms(metadata),
            file_identity,
            source_revision,
            source_generation: 0,
            issues,
        }
    }
}

#[cfg(all(test, windows))]
mod tests;
