use crate::adapters::{PublicationGuardedFileDiscovery, SqliteCatalog};
use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, ScanCheckpoint, ScanError, ScanRequest,
};
#[cfg(test)]
use crate::ports::CatalogRepository;
use crate::ports::IncrementalCatalogRepository;

use super::{FullScanReason, stable_id};

mod coverage;

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

enum ResolvedSelection {
    Scan(ResolvedScanRoot),
    AlreadyCovered(CoveredRoot),
}

pub(super) struct CoveredRoot {
    root_id: String,
    relative_path: String,
}

pub(super) enum ScanRootAdmission {
    Started {
        root_id: String,
        checkpoint: ScanCheckpoint,
        had_published_root: bool,
    },
    AlreadyCovered(CoveredRoot),
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
        let target = match self.resolve(catalog, request, root_path, identity, reason)? {
            ResolvedSelection::Scan(target) => target,
            ResolvedSelection::AlreadyCovered(covered) => {
                return Ok(ScanRootAdmission::AlreadyCovered(covered));
            }
        };
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
                FullScanReason::ResumeForegroundCheckpoint => {
                    let coverage = super::resumption::load_resume_coverage(
                        catalog,
                        &request.scan_id,
                        &root_id,
                        root_path,
                    )?;
                    catalog.resume_scan_with_publication_namespace(
                        request,
                        &root_id,
                        root_path,
                        identity,
                        coverage.as_ref(),
                    )?
                }
                #[cfg(test)]
                FullScanReason::ResumeAuthoritativeCheckpoint => {
                    catalog.resume_authoritative_scan(request, &root_id, root_path)?
                }
            },
        };
        Ok(ScanRootAdmission::Started {
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
    ) -> Result<ResolvedSelection, ScanError> {
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
                Ok(ResolvedSelection::Scan(
                    ResolvedScanRoot::ExplicitReplacement(previous),
                ))
            }
            Self::RegisteredPath => {
                if let Some(root_id) = catalog.load_registered_root_id(root_path)? {
                    return Ok(ResolvedSelection::Scan(ResolvedScanRoot::Registered(
                        root_id,
                    )));
                }
                if reason == FullScanReason::ExplicitUserRequest
                    && let Some(root_id) = catalog.load_published_root_id_by_identity(identity)?
                {
                    return Ok(ResolvedSelection::Scan(
                        ResolvedScanRoot::IdentityRecovered(require_root(catalog, &root_id)?),
                    ));
                }
                if reason == FullScanReason::ExplicitUserRequest
                    && let Some(covered) = coverage::find(catalog, root_path, identity)?
                {
                    return Ok(ResolvedSelection::AlreadyCovered(covered));
                }
                let legacy_id = stable_id("library-root-v1", root_path);
                if !catalog.has_registered_root_id(&legacy_id)? {
                    return Ok(ResolvedSelection::Scan(ResolvedScanRoot::Registered(
                        legacy_id,
                    )));
                }
                // The former path's identifier can belong to a source relocated elsewhere.
                Ok(ResolvedSelection::Scan(ResolvedScanRoot::Registered(
                    stable_id("library-root-registration-v1", &request.scan_id),
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
