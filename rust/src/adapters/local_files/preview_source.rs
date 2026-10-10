use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::{ExpectedFileState, FileIdentityEvidence, ScanIssue, SourceRevisionEvidence};

use super::revalidate_open_preview_source;
#[cfg(windows)]
use super::{WindowsDirectoryRootProof, open_source_file_with_root_proof};
#[cfg(not(windows))]
use super::{canonical_source_root_path, open_source_file};

pub(crate) struct OpenedPreviewSource {
    pub(crate) file: File,
    pub(crate) source_revision: Option<SourceRevisionEvidence>,
    pub(crate) source_root_path: PathBuf,
}

pub(crate) fn open_preview_source(
    expected: &ExpectedFileState,
    source_root: &Path,
    expected_root_identity: Option<&FileIdentityEvidence>,
) -> Result<OpenedPreviewSource, ScanIssue> {
    let path = Path::new(&expected.absolute_path);

    #[cfg(windows)]
    let (file, source_root_path) = {
        let expected_root_identity = expected_root_identity.ok_or_else(|| ScanIssue {
            path: Some(expected.absolute_path.clone()),
            code: "preview_root_identity_unproven".to_owned(),
            message: "The preview source root lacks durable Windows identity evidence".to_owned(),
        })?;
        let (root_proof, canonical_root) =
            WindowsDirectoryRootProof::open_configured(source_root, true).map_err(|error| {
                ScanIssue {
                    path: Some(expected.absolute_path.clone()),
                    code: "preview_root_unavailable".to_owned(),
                    message: format!("The preview source root cannot be resolved safely: {error}"),
                }
            })?;
        if &root_proof.identity != expected_root_identity {
            return Err(ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_root_identity_changed".to_owned(),
                message: "The preview source root no longer matches its publication identity"
                    .to_owned(),
            });
        }
        let file = open_source_file_with_root_proof(path, &canonical_root, &root_proof, || {})
            .map_err(|error| source_open_issue(expected, error))?;
        (file, canonical_root)
    };

    #[cfg(not(windows))]
    let (file, source_root_path) = {
        let source_root_path =
            canonical_source_root_path(source_root).map_err(|error| ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: "preview_root_unavailable".to_owned(),
                message: format!("The preview source root cannot be resolved safely: {error}"),
            })?;
        let file = open_source_file(path, &source_root_path)
            .map_err(|error| source_open_issue(expected, error))?;
        (file, source_root_path)
    };

    #[cfg(not(windows))]
    let _ = expected_root_identity;
    let source_revision = revalidate_open_preview_source(&file, expected)?;
    Ok(OpenedPreviewSource {
        file,
        source_revision,
        source_root_path,
    })
}

fn source_open_issue(expected: &ExpectedFileState, error: io::Error) -> ScanIssue {
    // Root admission precedes this observation. Absence still needs independent
    // path reconciliation before it can remove a catalog location.
    let code = if error.kind() == io::ErrorKind::NotFound {
        "preview_source_missing"
    } else {
        "preview_source_open_failed"
    };
    ScanIssue {
        path: Some(expected.absolute_path.clone()),
        code: code.to_owned(),
        message: error.to_string(),
    }
}
