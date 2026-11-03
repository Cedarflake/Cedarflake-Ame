use crate::domain::{
    AssetLocationView, IncrementalCatalogRoot, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangeScope, ScanError, ScanIssue,
};
use crate::ports::source_identity::SourceIdentityLocator;

pub(super) enum ReconciliationPaths {
    Exact(String),
    Relocated { previous: String, current: String },
}

impl ReconciliationPaths {
    pub(super) fn discover(
        locator: &impl SourceIdentityLocator,
        root: &IncrementalCatalogRoot,
        location: &AssetLocationView,
        issue: &ScanIssue,
    ) -> Result<Self, ScanError> {
        if let Some(relocated) = Self::discover_relocation(locator, root, location, issue)? {
            return Ok(relocated);
        }
        Ok(Self::Exact(location.relative_path.clone()))
    }

    fn discover_relocation(
        locator: &impl SourceIdentityLocator,
        root: &IncrementalCatalogRoot,
        location: &AssetLocationView,
        issue: &ScanIssue,
    ) -> Result<Option<Self>, ScanError> {
        if issue.code != "preview_source_missing" {
            return Ok(None);
        }
        let (Some(root_identity), Some(file_identity)) = (
            root.publication_root_identity.as_ref(),
            location.file_identity.as_ref(),
        ) else {
            return Ok(None);
        };
        let Some(paths) = locator.current_paths(
            &root.root_id,
            root.root_generation,
            &root.root_path,
            root_identity,
            file_identity,
        )?
        else {
            return Ok(None);
        };
        if paths.iter().any(|path| path == &location.relative_path) {
            return Ok(None);
        }
        Ok(paths
            .into_iter()
            .find(|path| !path.is_empty())
            .map(|current| Self::Relocated {
                previous: location.relative_path.clone(),
                current,
            }))
    }

    pub(super) fn into_work(
        self,
        root: &IncrementalCatalogRoot,
        now: i64,
    ) -> (LibraryChangeIntent, Vec<String>) {
        let (kind, relative_path, previous_relative_path, paths) = match self {
            Self::Exact(path) => (
                LibraryChangeIntentKind::Reconcile,
                path.clone(),
                None,
                vec![path],
            ),
            Self::Relocated { previous, current } => (
                LibraryChangeIntentKind::RenameCandidate,
                current.clone(),
                Some(previous.clone()),
                vec![previous, current],
            ),
        };
        (
            LibraryChangeIntent {
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                kind,
                scope: LibraryChangeScope::Path,
                relative_path,
                previous_relative_path,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: now,
                most_recent_observed_unix_ms: now,
                first_sequence: 0,
                most_recent_sequence: 0,
                coalesced_observation_count: 1,
            },
            paths,
        )
    }
}
