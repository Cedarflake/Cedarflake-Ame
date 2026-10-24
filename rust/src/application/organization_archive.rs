use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::domain::organization_archive::{
    OrganizationArchiveProgress, OrganizationArchiveReceipt,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::CatalogRepository;
use crate::ports::organization_archive::OrganizationArchiveRepository;

static EXPORT_ACTIVE: AtomicBool = AtomicBool::new(false);

struct ExportLease<'a>(&'a AtomicBool);

impl<'a> ExportLease<'a> {
    fn acquire(active: &'a AtomicBool) -> Result<Self, ScanError> {
        active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ScanError::new(
                    "organization_archive_busy",
                    "An organization export is already running",
                )
            })?;
        Ok(Self(active))
    }
}

impl Drop for ExportLease<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) fn export(
    destination: &str,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveReceipt, ScanError> {
    let destination = Path::new(destination);
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(ScanError::new(
            "organization_archive_destination_invalid",
            "Choose an absolute destination filename",
        ));
    }
    let _lease = ExportLease::acquire(&EXPORT_ACTIVE)?;
    let mut catalog = super::organization::open_catalog()?;
    let roots = catalog
        .load_snapshot(
            1,
            &GalleryQuery::default(),
            "organization-export",
            None,
            None,
            None,
        )?
        .roots;
    let storage = super::storage_paths()?;
    let catalog_directory = storage.catalog_path.parent().ok_or_else(|| {
        ScanError::new(
            "organization_archive_destination_invalid",
            "Missing catalog directory",
        )
    })?;
    let settings_directory = storage.settings_path.parent().ok_or_else(|| {
        ScanError::new(
            "organization_archive_destination_invalid",
            "Missing settings directory",
        )
    })?;
    validate_destination(
        destination,
        [
            catalog_directory,
            settings_directory,
            storage.preview_root.as_path(),
        ]
        .into_iter()
        .chain(roots.iter().map(|root| Path::new(&root.path))),
    )?;
    catalog.export_organization_archive(destination, progress)
}

fn validate_destination<'a>(
    destination: &Path,
    protected_paths: impl Iterator<Item = &'a Path>,
) -> Result<(), ScanError> {
    for protected in protected_paths {
        if super::storage::resolved_paths_overlap(protected, destination)? {
            return Err(ScanError::new(
                "organization_archive_destination_protected",
                "Choose a backup destination outside library sources and application storage",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organization_archive_bounds_concurrent_exports_and_releases_admission() {
        let active = AtomicBool::new(false);
        let first = ExportLease::acquire(&active).unwrap();
        assert_eq!(
            ExportLease::acquire(&active).err().unwrap().code,
            "organization_archive_busy"
        );
        assert!(active.load(Ordering::Acquire));
        drop(first);
        let second = ExportLease::acquire(&active).unwrap();
        drop(second);
        assert!(!active.load(Ordering::Acquire));
    }

    #[test]
    fn organization_archive_destination_excludes_sources_and_storage_sidecars() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("照片");
        let storage = directory.path().join("storage");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&storage).unwrap();
        for destination in [
            source.join("backup.ame-backup"),
            storage.join("catalog.sqlite3-wal"),
            source.join("sub").join("..").join("backup.ame-backup"),
        ] {
            let failure = validate_destination(
                &destination,
                [source.as_path(), storage.as_path()].into_iter(),
            )
            .unwrap_err();
            assert_eq!(failure.code, "organization_archive_destination_protected");
            assert!(!destination.exists());
        }
        validate_destination(
            &directory.path().join("export.ame-backup"),
            [source.as_path(), storage.as_path()].into_iter(),
        )
        .unwrap();
    }
}
