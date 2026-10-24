use std::path::Path;

use crate::domain::organization_archive::{
    OrganizationArchiveInspection, OrganizationArchiveProgress, OrganizationArchiveReceipt,
    OrganizationArchiveRestoreReceipt, OrganizationArchiveRestoreRequest,
};
use crate::domain::{GalleryQuery, ScanError};
use crate::ports::CatalogRepository;
use crate::ports::organization_archive::OrganizationArchiveRepository;

mod operation;
pub(crate) use operation::ArchiveTaskControl;

pub(crate) fn export(
    destination: &str,
    control: &ArchiveTaskControl,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveReceipt, ScanError> {
    let destination = Path::new(destination);
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(ScanError::new(
            "organization_archive_destination_invalid",
            "Choose an absolute destination filename",
        ));
    }
    let _lease = control.admit()?;
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
    catalog.export_organization_archive(destination, &mut |update| {
        control.should_continue() && progress(update)
    })
}

pub(crate) fn inspect(
    archive: &str,
    control: &ArchiveTaskControl,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveInspection, ScanError> {
    let archive = Path::new(archive);
    if !archive.is_absolute() || archive.file_name().is_none() {
        return Err(ScanError::new(
            "organization_archive_input_invalid",
            "Choose an absolute backup filename",
        ));
    }
    let _lease = control.admit()?;
    super::organization::open_catalog()?.inspect_organization_archive(archive, &mut |update| {
        control.should_continue() && progress(update)
    })
}

pub(crate) fn restore(
    archive: &str,
    request: &OrganizationArchiveRestoreRequest,
    control: &ArchiveTaskControl,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<OrganizationArchiveRestoreReceipt, ScanError> {
    request.validate()?;
    let archive = Path::new(archive);
    if !archive.is_absolute() || archive.file_name().is_none() {
        return Err(ScanError::new(
            "organization_archive_input_invalid",
            "Choose an absolute backup filename",
        ));
    }
    let _lease = control.admit()?;
    super::organization::open_catalog()?.restore_organization_archive(
        archive,
        request,
        &mut |update| control.should_continue() && progress(update),
    )
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
