use std::io::ErrorKind;
use std::path::Path;

use crate::domain::{CatalogSnapshot, PreviewStatus, ScanError};
use crate::ports::preview_cache_storage::PreviewCacheStorage;
use crate::ports::{
    CatalogRepository, PreviewHealthObservation, PreviewHealthOutcome, PreviewHealthTarget,
};

#[cfg(test)]
thread_local! {
    static AFTER_OBSERVATION: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
    static AFTER_MISSING_HINT: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
}

pub(super) fn reconcile_snapshot_previews(
    catalog: &mut impl CatalogRepository,
    root: &Path,
    snapshot: &mut CatalogSnapshot,
) -> Result<(), ScanError> {
    for entry in &mut snapshot.assets {
        let asset = &mut entry.source;
        if !matches!(asset.preview_status, PreviewStatus::Ready)
            || is_active_location_file(&asset.preview_path, root)?
        {
            continue;
        }
        #[cfg(test)]
        if let Some(hook) = AFTER_MISSING_HINT.with(|value| value.borrow_mut().take()) {
            hook();
        }
        if reconcile(catalog, root, PreviewHealthTarget::Location(asset))?
            == PreviewHealthOutcome::Invalidated
        {
            asset.preview_path.clear();
            asset.preview_status = PreviewStatus::Pending;
            asset.preview_issue_code = None;
            asset.preview_issue_message = None;
        }
    }
    Ok(())
}

pub(super) fn is_active_location_file(path: &str, root: &Path) -> Result<bool, ScanError> {
    Ok(!path.is_empty()
        && super::storage::production::preview_cache()
            .current_artifact_key(Path::new(path))
            .is_some()
        && Path::new(path).is_file()
        && super::storage::resolved_path_is_within(Path::new(path), root)?)
}

pub(super) fn reconcile_missing_root(
    catalog: &mut impl CatalogRepository,
    root: &Path,
    target: PreviewHealthTarget<'_>,
) -> Result<PreviewHealthOutcome, ScanError> {
    let _access = match super::acquire_preview_reclamation() {
        Ok(access) => access,
        Err(error) if error.code == "preview_cleanup_active" => {
            return Ok(PreviewHealthOutcome::Deferred);
        }
        Err(error) => return Err(error),
    };
    // Revalidate absence under publication exclusion; never inspect a later replacement root.
    match std::fs::symlink_metadata(root) {
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Ok(_) | Err(_) => return Ok(PreviewHealthOutcome::Unavailable),
    }
    let outcome =
        catalog.try_reconcile_preview_health(target, PreviewHealthObservation::Missing)?;
    if outcome == PreviewHealthOutcome::Invalidated {
        super::preview::invalidate_active_preview_store()?;
    }
    Ok(outcome)
}

/// Candidates are only lookup hints. Final filesystem evidence and its conditional catalog
/// write share the publication exclusion; database contention returns without retaining it.
pub(super) fn reconcile(
    catalog: &mut impl CatalogRepository,
    root: &Path,
    target: PreviewHealthTarget<'_>,
) -> Result<PreviewHealthOutcome, ScanError> {
    let _access = match super::acquire_preview_reclamation() {
        Ok(access) => access,
        Err(error) if error.code == "preview_cleanup_active" => {
            return Ok(PreviewHealthOutcome::Deferred);
        }
        Err(error) => return Err(error),
    };
    let path = Path::new(target.path());
    let cache = super::storage::production::preview_cache();
    let is_compatible = match target {
        PreviewHealthTarget::Artifact(_) => {
            path.parent() == Some(root) && cache.managed_entry_kind(path).is_some()
        }
        PreviewHealthTarget::Location(_) => {
            !path.as_os_str().is_empty()
                && cache.current_artifact_key(path).is_some()
                && super::storage::resolved_path_is_within(path, root)?
        }
    };
    let observation = if !is_compatible {
        PreviewHealthObservation::Missing
    } else {
        match path.metadata() {
            Ok(metadata) if metadata.is_file() => PreviewHealthObservation::File {
                byte_size: metadata.len(),
            },
            Err(error) if error.kind() == ErrorKind::NotFound => PreviewHealthObservation::Missing,
            Ok(_) | Err(_) => return Ok(PreviewHealthOutcome::Unavailable),
        }
    };
    if matches!(target, PreviewHealthTarget::Location(_))
        && matches!(observation, PreviewHealthObservation::File { .. })
    {
        return Ok(PreviewHealthOutcome::Unchanged);
    }
    #[cfg(test)]
    if let Some(hook) = AFTER_OBSERVATION.with(|value| value.borrow_mut().take()) {
        hook();
    }
    let outcome = catalog.try_reconcile_preview_health(target, observation)?;
    if outcome == PreviewHealthOutcome::Invalidated {
        super::preview::invalidate_active_preview_store()?;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests;
