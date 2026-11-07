use std::fs::File;
use std::io::{Read, Write};

use crate::adapters::PreviewCacheNamespace;
use crate::adapters::local_files::{open_catalog_identity_guard, read_open_catalog_identity};
use crate::domain::ScanError;

use super::invalid;

const FILE_NAME: &str = "semantic-features.owner";
const PREFIX: &str = "ame.semantic-features.owner.v1\n";

pub(super) fn record(namespace: &PreviewCacheNamespace, database: &File) -> Result<(), ScanError> {
    let root = namespace
        .root()
        .ok_or_else(|| invalid("The feature directory is unavailable"))?;
    let path = root.join(FILE_NAME);
    let expected = receipt(database)?;
    match read(&path) {
        Ok((_, current)) if current == expected => return Ok(()),
        Ok((_, current)) if current.starts_with(PREFIX) => {}
        Ok(_) => return Err(invalid("The semantic ownership receipt is unrecognized")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(invalid(error.to_string())),
    }
    let mut staged =
        tempfile::NamedTempFile::new_in(root).map_err(|error| invalid(error.to_string()))?;
    staged
        .write_all(expected.as_bytes())
        .map_err(|error| invalid(error.to_string()))?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| invalid(error.to_string()))?;
    staged
        .persist(path)
        .map_err(|error| invalid(error.error.to_string()))?;
    Ok(())
}

pub(super) fn require(
    namespace: &PreviewCacheNamespace,
    database: &File,
) -> Result<File, ScanError> {
    let root = namespace
        .root()
        .ok_or_else(|| invalid("The feature directory is unavailable"))?;
    let (guard, observed) =
        read(&root.join(FILE_NAME)).map_err(|error| invalid(error.to_string()))?;
    if observed != receipt(database)? {
        return Err(invalid(
            "The semantic feature file does not match its ownership receipt",
        ));
    }
    Ok(guard)
}

fn receipt(database: &File) -> Result<String, ScanError> {
    let identity = read_open_catalog_identity(database)
        .map_err(|error| invalid(error.to_string()))?
        .file_identity
        .ok_or_else(|| invalid("The semantic feature file identity is unavailable"))?;
    Ok(format!("{PREFIX}{}\n{}\n", identity.scheme, identity.value))
}

fn read(path: &std::path::Path) -> std::io::Result<(File, String)> {
    let (file, _) = open_catalog_identity_guard(path)?;
    let mut value = String::new();
    (&file).take(4097).read_to_string(&mut value)?;
    if value.len() > 4096 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid ownership receipt",
        ));
    }
    Ok((file, value))
}
