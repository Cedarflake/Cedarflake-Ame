use crate::domain::{AssetLocationView, PreviewReclamationCandidate};

#[derive(Clone, Copy)]
pub enum PreviewHealthTarget<'a> {
    Artifact(&'a PreviewReclamationCandidate),
    Location(&'a AssetLocationView),
}

impl PreviewHealthTarget<'_> {
    pub fn path(&self) -> &str {
        match self {
            Self::Artifact(candidate) => &candidate.path,
            Self::Location(location) => &location.preview_path,
        }
    }
}

#[derive(Clone, Copy)]
pub enum PreviewHealthObservation {
    Missing,
    File { byte_size: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewHealthOutcome {
    Deferred,
    Unchanged,
    Invalidated,
    CorrectedSize,
    Unavailable,
}
