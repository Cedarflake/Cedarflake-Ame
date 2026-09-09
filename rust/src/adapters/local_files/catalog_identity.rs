use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::FileIdentityEvidence;

#[cfg(windows)]
use super::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_NO_RECALL,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_SHARE_READ,
    FILE_SHARE_WRITE, MetadataExt, OpenOptions, OpenOptionsExt, SYNCHRONIZE,
    file_identity_from_handle, final_path_from_handle, open_validation_handle,
};

pub(crate) struct CatalogFileIdentity {
    pub(crate) canonical_path: PathBuf,
    pub(crate) file_identity: Option<FileIdentityEvidence>,
}

#[cfg(windows)]
pub(crate) fn read_catalog_identity(path: &Path) -> io::Result<CatalogFileIdentity> {
    let file = open_validation_handle(path)?;
    identity_from_handle(&file)
}

#[cfg(windows)]
fn identity_from_handle(file: &File) -> io::Result<CatalogFileIdentity> {
    // The path and ID must describe one fresh object, even if an ancestor junction changes.
    Ok(CatalogFileIdentity {
        canonical_path: final_path_from_handle(file)?,
        file_identity: file_identity_from_handle(file)?,
    })
}

#[cfg(not(windows))]
pub(crate) fn read_catalog_identity(path: &Path) -> io::Result<CatalogFileIdentity> {
    Ok(CatalogFileIdentity {
        canonical_path: std::fs::canonicalize(path)?,
        file_identity: super::file_identity(path)?,
    })
}

#[cfg(windows)]
pub(crate) fn open_catalog_identity_guard(
    path: &Path,
) -> io::Result<(File, Option<FileIdentityEvidence>)> {
    let file = OpenOptions::new()
        .access_mode(FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_OPEN_REPARSE_POINT,
        )
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "catalog identity guard requires a non-reparse regular file",
        ));
    }
    let identity = file_identity_from_handle(&file)?;
    Ok((file, identity))
}

#[cfg(not(windows))]
pub(crate) fn open_catalog_identity_guard(
    path: &Path,
) -> io::Result<(File, Option<FileIdentityEvidence>)> {
    let file = File::open(path)?;
    let identity = super::file_identity(path)?;
    Ok((file, identity))
}

#[cfg(all(test, windows))]
mod tests;
