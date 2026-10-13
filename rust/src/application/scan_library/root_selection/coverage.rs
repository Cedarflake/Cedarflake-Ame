use std::path::Path;

use crate::adapters::{PublicationGuardedFileDiscovery, SqliteCatalog};
use crate::domain::{FileIdentityEvidence, ScanError, ScanEvent, ScanRequest};
use crate::ports::IncrementalCatalogRepository;

use super::super::{CONTROL_CANCEL, CONTROL_SUSPEND};
use super::CoveredRoot;

pub(super) fn find(
    catalog: &SqliteCatalog,
    root_path: &str,
    identity: &FileIdentityEvidence,
) -> Result<Option<CoveredRoot>, ScanError> {
    let selected = Path::new(root_path);
    let roots = catalog.load_incremental_catalog_roots()?;
    let candidate = roots
        .iter()
        .filter_map(|root| {
            let relative = selected.strip_prefix(&root.root_path).ok()?;
            if relative.as_os_str().is_empty() {
                return None;
            }
            Some((root, relative))
        })
        .max_by_key(|(root, _)| Path::new(&root.root_path).components().count());
    let Some((root, relative)) = candidate else {
        return Ok(None);
    };
    let relative_path = relative.to_string_lossy().replace('\\', "/");
    // Coverage selects a configured scope; it never publishes an unfinished import.
    let namespace = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &root.root_path,
        root.publication_root_identity.as_ref(),
    )?;
    if !namespace
        .contains_directory_identity(&relative_path, identity)
        .map_err(|issue| ScanError::new(issue.code, issue.message))?
    {
        return Err(ScanError::new(
            "catalog_root_coverage_identity_changed",
            "The selected directory no longer belongs to the registered source namespace",
        ));
    }
    let current = catalog.load_incremental_catalog_root(&root.root_id)?;
    if !current.as_ref().is_some_and(|current| {
        current.root_path == root.root_path
            && current.root_generation == root.root_generation
            && current.publication_root_identity == root.publication_root_identity
    }) {
        return Err(ScanError::new(
            "catalog_root_coverage_changed",
            "The covering library source changed during directory selection",
        ));
    }
    Ok(Some(CoveredRoot {
        root_id: root.root_id.clone(),
        relative_path,
    }))
}

impl CoveredRoot {
    pub(in crate::application::scan_library) fn finish(
        self,
        request: &ScanRequest,
        control: u8,
        mut publish: impl FnMut(ScanEvent) -> bool,
    ) {
        let event = if control == CONTROL_CANCEL || control == CONTROL_SUSPEND {
            ScanEvent::Cancelled {
                scan_id: request.scan_id.clone(),
                accepted_items: 0,
                issue_count: 0,
            }
        } else {
            ScanEvent::AlreadyCovered {
                scan_id: request.scan_id.clone(),
                root_id: self.root_id,
                relative_path: self.relative_path,
            }
        };
        publish(event);
    }
}
