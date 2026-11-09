use serde::{Deserialize, Serialize};

use crate::domain::file_operations::{
    FileOperationDestination, FileOperationDirectory, FileOperationSource,
};
use crate::domain::{ExpectedFileState, FileIdentityEvidence, SourceRevisionEvidence};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Directory {
    path: String,
    identity: [String; 2],
}

impl From<&FileOperationDirectory> for Directory {
    fn from(value: &FileOperationDirectory) -> Self {
        Self {
            path: value.path.clone(),
            identity: [value.identity.scheme.clone(), value.identity.value.clone()],
        }
    }
}

impl From<Directory> for FileOperationDirectory {
    fn from(value: Directory) -> Self {
        let [scheme, identity] = value.identity;
        Self {
            path: value.path,
            identity: FileIdentityEvidence {
                scheme,
                value: identity,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    parent: Directory,
    path: String,
    size: u64,
    modified_unix_ms: i64,
    identity: Option<[String; 2]>,
    revision: Option<[String; 2]>,
}

impl From<&FileOperationSource> for Source {
    fn from(value: &FileOperationSource) -> Self {
        Self {
            parent: (&value.parent).into(),
            path: value.state.absolute_path.clone(),
            size: value.state.file_size,
            modified_unix_ms: value.state.modified_unix_ms,
            identity: value
                .state
                .file_identity
                .as_ref()
                .map(|identity| [identity.scheme.clone(), identity.value.clone()]),
            revision: value
                .state
                .source_revision
                .as_ref()
                .map(|revision| [revision.scheme.clone(), revision.value.clone()]),
        }
    }
}

impl From<Source> for FileOperationSource {
    fn from(value: Source) -> Self {
        Self {
            parent: value.parent.into(),
            state: ExpectedFileState {
                absolute_path: value.path,
                file_size: value.size,
                modified_unix_ms: value.modified_unix_ms,
                file_identity: value
                    .identity
                    .map(|[scheme, value]| FileIdentityEvidence { scheme, value }),
                source_revision: value
                    .revision
                    .map(|[scheme, value]| SourceRevisionEvidence { scheme, value }),
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Destination {
    directory: Directory,
    name: String,
}

impl From<&FileOperationDestination> for Destination {
    fn from(value: &FileOperationDestination) -> Self {
        Self {
            directory: (&value.directory).into(),
            name: value.name.clone(),
        }
    }
}

impl From<Destination> for FileOperationDestination {
    fn from(value: Destination) -> Self {
        Self {
            directory: value.directory.into(),
            name: value.name,
        }
    }
}
