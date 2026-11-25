use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::manifest::invalid;
use super::package::{RetainedPackage, storage};
use crate::domain::application_update::ApplicationUpdateError;
use crate::ports::application_updates::ApplicationDownloadObserver;

pub(super) struct StagedEngine {
    pub sha256: String,
    pub length: u64,
    _lease: File,
}

pub(super) fn stage(
    package: &RetainedPackage,
    version: &str,
    directory: &Path,
) -> Result<StagedEngine, ApplicationUpdateError> {
    let _package_lease = package.verified_file(&Preparation)?;
    let mut bundle = velopack::bundle::load_bundle_from_file(&package.path).map_err(storage)?;
    if bundle.len() > 16_384 || bundle.calculate_size().1 > 4 * 1024 * 1024 * 1024 {
        return Err(invalid());
    }
    let manifest = bundle.read_manifest().map_err(storage)?;
    if manifest.id != "CedarflakeAme"
        || manifest.version.to_string() != version
        || manifest.main_exe != "cedarflake_ame.exe"
    {
        return Err(invalid());
    }
    let names = bundle.get_file_names().map_err(storage)?;
    if names
        .iter()
        .filter(|name| name.as_str() == "lib/app/Squirrel.exe")
        .count()
        != 1
    {
        return Err(invalid());
    }
    let index = bundle
        .find_zip_file(|name| name == "lib/app/Squirrel.exe")
        .ok_or_else(invalid)?;
    let path = directory.join("Update.exe");
    bundle
        .extract_zip_idx_to_path(index, &path)
        .map_err(storage)?;
    let mut lease = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .map_err(storage)?;
    let length = lease.metadata().map_err(storage)?.len();
    if length == 0 || length > 64 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut digest = Sha256::new();
    let mut buffer = [0; 128 * 1024];
    loop {
        let count = lease.read(&mut buffer).map_err(storage)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(StagedEngine {
        sha256: format!("{:x}", digest.finalize()),
        length,
        _lease: lease,
    })
}

struct Preparation;
impl ApplicationDownloadObserver for Preparation {
    fn cancelled(&self) -> bool {
        false
    }
    fn received(&self, _: u64) {}
    fn verifying(&self) {}
}
