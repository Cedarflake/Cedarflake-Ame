use crate::adapters::{PublicationGuardedFileDiscovery, SqliteCatalog};
use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, ScanCheckpoint, ScanError, ScanRequest,
};
#[cfg(test)]
use crate::ports::CatalogRepository;
use crate::ports::IncrementalCatalogRepository;

use super::{FullScanReason, stable_id};

pub(super) enum ScanRootSelection {
    RegisteredPath,
    Relocate {
        root_id: String,
        expected_path: String,
    },
}

enum ResolvedScanRoot {
    Registered(String),
    ExplicitReplacement(IncrementalCatalogRoot),
    IdentityRecovered(IncrementalCatalogRoot),
}

pub(super) struct ScanRootAdmission {
    pub root_id: String,
    pub checkpoint: ScanCheckpoint,
    pub had_published_root: bool,
}

impl ScanRootSelection {
    pub fn begin(
        self,
        catalog: &mut SqliteCatalog,
        request: &ScanRequest,
        root_path: &str,
        identity: &FileIdentityEvidence,
        reason: FullScanReason,
    ) -> Result<ScanRootAdmission, ScanError> {
        let target = self.resolve(catalog, request, root_path, identity, reason)?;
        let (root_id, had_published_root) = match &target {
            ResolvedScanRoot::Registered(root_id) => {
                let previous = catalog.load_incremental_catalog_root(root_id)?;
                (
                    root_id.clone(),
                    previous.is_some_and(|root| root.active_scan_id.is_some()),
                )
            }
            ResolvedScanRoot::ExplicitReplacement(root)
            | ResolvedScanRoot::IdentityRecovered(root) => {
                (root.root_id.clone(), root.active_scan_id.is_some())
            }
        };
        let _namespace_guard = match &target {
            ResolvedScanRoot::Registered(_) => None,
            ResolvedScanRoot::ExplicitReplacement(_) | ResolvedScanRoot::IdentityRecovered(_) => {
                Some(
                    PublicationGuardedFileDiscovery::new_metadata_inventory_publication_guard(
                        root_path, identity,
                    )?,
                )
            }
        };
        let checkpoint = match target {
            ResolvedScanRoot::ExplicitReplacement(previous) => {
                catalog.begin_relocated_scan(request, &previous, root_path, identity)?
            }
            ResolvedScanRoot::IdentityRecovered(previous) => {
                catalog.begin_identity_recovered_scan(request, &previous, root_path, identity)?
            }
            ResolvedScanRoot::Registered(_) => match reason {
                FullScanReason::ExplicitUserRequest => catalog
                    .begin_scan_with_publication_namespace(
                        request, &root_id, root_path, identity,
                    )?,
                FullScanReason::ResumeForegroundCheckpoint => catalog
                    .resume_scan_with_publication_namespace(
                        request, &root_id, root_path, identity,
                    )?,
                #[cfg(test)]
                FullScanReason::ResumeAuthoritativeCheckpoint => {
                    catalog.resume_authoritative_scan(request, &root_id, root_path)?
                }
            },
        };
        Ok(ScanRootAdmission {
            root_id,
            checkpoint,
            had_published_root,
        })
    }

    fn resolve(
        self,
        catalog: &SqliteCatalog,
        request: &ScanRequest,
        root_path: &str,
        identity: &FileIdentityEvidence,
        reason: FullScanReason,
    ) -> Result<ResolvedScanRoot, ScanError> {
        match self {
            Self::Relocate {
                root_id,
                expected_path,
            } => {
                let previous = require_root(catalog, &root_id)?;
                if previous.root_path != expected_path && previous.root_path != root_path {
                    return Err(ScanError::new(
                        "catalog_root_relocation_stale",
                        "The selected library root has a different location",
                    ));
                }
                Ok(ResolvedScanRoot::ExplicitReplacement(previous))
            }
            Self::RegisteredPath => {
                if let Some(root_id) = catalog.load_registered_root_id(root_path)? {
                    return Ok(ResolvedScanRoot::Registered(root_id));
                }
                if reason == FullScanReason::ExplicitUserRequest
                    && let Some(root_id) = catalog.load_published_root_id_by_identity(identity)?
                {
                    return Ok(ResolvedScanRoot::IdentityRecovered(require_root(
                        catalog, &root_id,
                    )?));
                }
                let legacy_id = stable_id("library-root-v1", root_path);
                if !catalog.has_registered_root_id(&legacy_id)? {
                    return Ok(ResolvedScanRoot::Registered(legacy_id));
                }
                // The former path's identifier can belong to a source relocated elsewhere.
                Ok(ResolvedScanRoot::Registered(stable_id(
                    "library-root-registration-v1",
                    &request.scan_id,
                )))
            }
        }
    }
}

fn require_root(
    catalog: &SqliteCatalog,
    root_id: &str,
) -> Result<IncrementalCatalogRoot, ScanError> {
    catalog
        .load_incremental_catalog_root(root_id)?
        .ok_or_else(|| {
            ScanError::new(
                "catalog_root_relocation_missing",
                "The selected library root no longer exists",
            )
        })
}
