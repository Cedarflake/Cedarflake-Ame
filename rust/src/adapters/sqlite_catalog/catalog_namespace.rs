use std::fs::File;
use std::path::{Path, PathBuf};

use super::super::local_files::{
    CatalogFileIdentity, open_catalog_identity_guard, read_catalog_identity,
};
#[cfg(windows)]
use super::operation_diagnostics::measure;
use super::{FileIdentityEvidence, ScanError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SqliteDatabaseIdentity {
    pub(super) canonical_path: PathBuf,
    pub(super) file_identity: FileIdentityEvidence,
}

pub(super) struct SqliteCatalogIdentityGuard {
    file: File,
    pub(super) identity: SqliteDatabaseIdentity,
}

pub(super) fn catalog_database_identity(path: &Path) -> Result<SqliteDatabaseIdentity, ScanError> {
    database_identity(read_catalog_identity(path))
}

pub(super) fn open_catalog_database_identity_guard(
    path: &Path,
) -> Result<SqliteCatalogIdentityGuard, ScanError> {
    let (file, file_identity) = open_catalog_identity_guard(path).map_err(identity_error)?;
    if file_identity.is_none() {
        return Err(ScanError::new(
            "catalog_identity_unsupported",
            "The current platform cannot provide stable catalog file identity evidence",
        ));
    }
    #[cfg(windows)]
    let identity = database_identity(super::super::local_files::read_open_catalog_identity(&file))?;
    #[cfg(not(windows))]
    let identity = SqliteDatabaseIdentity {
        canonical_path: std::fs::canonicalize(path).map_err(identity_error)?,
        file_identity: file_identity.clone().ok_or_else(unsupported_identity)?,
    };
    if Some(&identity.file_identity) != file_identity.as_ref() {
        return Err(identity_error(std::io::Error::other(
            "The held catalog identity changed during observation",
        )));
    }
    Ok(SqliteCatalogIdentityGuard { file, identity })
}

impl SqliteCatalogIdentityGuard {
    pub(super) fn observe_current_namespace(
        &self,
        path: &Path,
    ) -> Result<SqliteDatabaseIdentity, ScanError> {
        #[cfg(windows)]
        if measure("proof_namespace_resolution", || {
            resolves_to_pinned_disk_namespace(path, &self.identity.canonical_path)
        }) {
            // SQLite and this guard both deny deletion. Query the pinned file afresh;
            // its current final path must still match, including after ancestor renames.
            return measure("proof_pinned_identity", || {
                database_identity(super::super::local_files::read_open_catalog_identity(
                    &self.file,
                ))
            });
        }
        #[cfg(not(windows))]
        let _ = &self.file;
        // An alias may retarget while the opened database remains alive.
        catalog_database_identity(path)
    }
}

fn database_identity(
    observation: std::io::Result<CatalogFileIdentity>,
) -> Result<SqliteDatabaseIdentity, ScanError> {
    let observed = observation.map_err(identity_error)?;
    Ok(SqliteDatabaseIdentity {
        canonical_path: observed.canonical_path,
        file_identity: observed.file_identity.ok_or_else(unsupported_identity)?,
    })
}

fn identity_error(error: std::io::Error) -> ScanError {
    ScanError::new(
        "catalog_identity_unavailable",
        format!("Could not inspect the catalog identity: {error}"),
    )
}

fn unsupported_identity() -> ScanError {
    ScanError::new(
        "catalog_identity_unavailable",
        "The catalog file identity is unavailable",
    )
}

#[cfg(windows)]
fn resolves_to_pinned_disk_namespace(path: &Path, canonical: &Path) -> bool {
    if direct_disk_namespace(path, canonical) {
        return true;
    }
    if !direct_disk_namespace(path, path) {
        return false;
    }
    // Expand DOS names afresh: a cached expansion would miss a changed short-name mapping.
    // Junction spellings still differ from the pinned file's resolved physical namespace.
    match super::super::local_files::windows_long_dos_path(path) {
        Ok(expanded) => direct_disk_namespace(&expanded, canonical),
        Err(_) => false,
    }
}

#[cfg(windows)]
fn direct_disk_namespace(path: &Path, canonical: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    fn disk_path(value: &[u16]) -> Option<&[u16]> {
        const BACKSLASH: u16 = b'\\' as u16;
        const COLON: u16 = b':' as u16;
        const DOT: u16 = b'.' as u16;
        const SPACE: u16 = b' ' as u16;
        const SLASH: u16 = b'/' as u16;
        const VERBATIM_PREFIX: [u16; 4] = [BACKSLASH, BACKSLASH, b'?' as u16, BACKSLASH];

        let value = value.strip_prefix(&VERBATIM_PREFIX).unwrap_or(value);
        let drive = value.first().and_then(|unit| u8::try_from(*unit).ok());
        if value.len() < 4
            || !drive.is_some_and(|letter| letter.is_ascii_alphabetic())
            || value[1..3] != [COLON, BACKSLASH]
        {
            return None;
        }
        for component in value[3..].split(|unit| *unit == BACKSLASH) {
            if component.is_empty()
                || matches!(component.last().copied(), Some(SPACE | DOT))
                || component.iter().any(|unit| matches!(*unit, SLASH | COLON))
            {
                return None;
            }
        }
        Some(value)
    }

    let requested: Vec<_> = path.as_os_str().encode_wide().collect();
    let resolved: Vec<_> = canonical.as_os_str().encode_wide().collect();
    match (disk_path(&requested), disk_path(&resolved)) {
        (Some(requested), Some(resolved)) => requested == resolved,
        _ => false,
    }
}

#[cfg(all(test, windows))]
mod tests;
