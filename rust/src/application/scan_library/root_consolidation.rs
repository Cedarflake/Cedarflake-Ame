use std::path::Path;

use crate::domain::ScanError;
use crate::ports::scan_admission::ScanAdmissionRepository;
use crate::ports::scan_publication::ScanPublicationRepository;
use crate::ports::source_publication::{PublicationNamespace, PublicationNamespaceFactory};
use crate::ports::{RootConsolidationCandidate, RootConsolidationPlan};

pub(super) struct PreparedRootConsolidation<Namespace> {
    plan: Option<RootConsolidationPlan>,
    _namespaces: Vec<Namespace>,
}

impl<Namespace: PublicationNamespace> PreparedRootConsolidation<Namespace> {
    pub(super) fn capture(
        catalog: &(impl ScanAdmissionRepository + ScanPublicationRepository),
        namespace_factory: &impl PublicationNamespaceFactory<Namespace = Namespace>,
        scan_id: &str,
        root_id: &str,
        parent_namespace: &Namespace,
        was_limited: bool,
    ) -> Result<Self, ScanError> {
        let parent = catalog
            .load_incremental_catalog_root(root_id)?
            .ok_or_else(|| {
                ScanError::new("catalog_root_missing", "The import source no longer exists")
            })?;
        if parent.active_scan_id.is_some() {
            return Ok(Self {
                plan: None,
                _namespaces: Vec::new(),
            });
        }
        let mut candidates = Vec::new();
        let mut namespaces = Vec::new();
        for root in catalog.load_root_consolidation_registrations(&parent.root_id)? {
            let Ok(relative) = Path::new(&root.root_path).strip_prefix(&parent.root_path) else {
                continue;
            };
            if relative.as_os_str().is_empty() {
                continue;
            }
            if was_limited {
                return Err(ScanError::new(
                    "catalog_root_consolidation_incomplete",
                    "A limited parent import cannot replace an existing child source",
                ));
            }
            if root.has_running_scan {
                return Err(ScanError::new(
                    "catalog_root_consolidation_busy",
                    "A covered source must finish its active scan before consolidation",
                ));
            }
            let relative_path = relative.to_string_lossy().replace('\\', "/");
            if root.active_scan_id.is_some() && root.publication_root_identity.is_none() {
                return Err(ScanError::new(
                    "catalog_root_consolidation_unpublished",
                    "The covered publication has no verifiable directory identity",
                ));
            }
            let namespace =
                namespace_factory.open(&root.root_path, root.publication_root_identity.as_ref())?;
            let identity = namespace
                .metadata_inventory_root_identity()?
                .ok_or_else(|| {
                    ScanError::new(
                        "catalog_root_consolidation_identity_changed",
                        "The covered directory identity cannot be verified",
                    )
                })?;
            if !parent_namespace
                .contains_directory_identity(&relative_path, &identity)
                .map_err(|issue| ScanError::new(issue.code, issue.message))?
            {
                return Err(ScanError::new(
                    "catalog_root_consolidation_identity_changed",
                    "A covered source no longer belongs to the imported directory namespace",
                ));
            }
            namespaces.push(namespace);
            candidates.push(RootConsolidationCandidate {
                expected: root,
                relative_path,
            });
        }
        if !candidates.is_empty() {
            super::finalization::retained_issue_recovery::require_complete_namespace(
                catalog, scan_id,
            )?;
        }
        Ok(Self {
            plan: Some(RootConsolidationPlan {
                scan_id: scan_id.to_owned(),
                destination_root_id: parent.root_id,
                destination_path: parent.root_path,
                candidates,
            }),
            _namespaces: namespaces,
        })
    }

    pub(super) fn plan(&self) -> Option<&RootConsolidationPlan> {
        self.plan.as_ref()
    }
}

#[cfg(test)]
mod tests;
