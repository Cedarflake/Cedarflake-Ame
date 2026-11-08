use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use super::{ScanError, database_error, invalid};
use crate::adapters::PreviewCacheNamespace;

pub(super) const APPLICATION_ID: u32 = 0x414D5346;
pub(super) const SCHEMA_VERSION: u32 = 1;
pub(super) const FILE_NAME: &str = "semantic-features.sqlite3";

pub(super) fn open(
    directory: &Path,
) -> Result<(Connection, File, PreviewCacheNamespace), ScanError> {
    let namespace = PreviewCacheNamespace::for_operation(directory)?;
    let root = namespace
        .root()
        .ok_or_else(|| invalid("The semantic feature directory is unavailable"))?;
    let path = root.join(FILE_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => initialize(root, &path)?,
        Err(error) => {
            return Err(invalid(format!(
                "Could not inspect semantic features: {error}"
            )));
        }
    }
    let (file, _) = crate::adapters::local_files::open_catalog_identity_guard(&path)
        .map_err(|error| invalid(format!("Semantic feature storage is unavailable: {error}")))?;
    let mut header = [0; 72];
    (&file)
        .read_exact(&mut header)
        .map_err(|error| invalid(error.to_string()))?;
    if &header[..16] != b"SQLite format 3\0"
        || u32::from_be_bytes(header[60..64].try_into().expect("fixed header field"))
            != SCHEMA_VERSION
        || u32::from_be_bytes(header[68..72].try_into().expect("fixed header field"))
            != APPLICATION_ID
    {
        return Err(invalid(
            "This file is not a supported semantic feature store",
        ));
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(database_error)?;
    let page_size: i64 = connection
        .pragma_query_value(None, "page_size", |row| row.get(0))
        .map_err(database_error)?;
    if page_size != 4096 {
        return Err(invalid(
            "Semantic feature pages do not match their storage budget",
        ));
    }
    connection
        .execute_batch(
            "PRAGMA trusted_schema = OFF;
         PRAGMA journal_mode = DELETE;
         PRAGMA synchronous = FULL;
         PRAGMA cache_size = -2048;
         PRAGMA max_page_count = 524288;
         PRAGMA journal_size_limit = 0;",
        )
        .map_err(database_error)?;
    Ok((connection, file, namespace))
}

fn initialize(root: &Path, destination: &Path) -> Result<(), ScanError> {
    let staged = tempfile::Builder::new()
        .prefix("semantic-feature-setup-")
        .suffix(".sqlite3")
        .tempfile_in(root)
        .map_err(|error| invalid(error.to_string()))?;
    let mut connection = Connection::open(staged.path()).map_err(database_error)?;
    initialize_schema(&mut connection)?;
    drop(connection);
    match staged.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(invalid(format!(
            "Could not create semantic features: {}",
            error.error
        ))),
    }
}

pub(super) fn initialize_schema(connection: &mut Connection) -> Result<(), ScanError> {
    connection
        .execute_batch("PRAGMA page_size = 4096;")
        .map_err(database_error)?;
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(include_str!("schema.sql"))
        .map_err(database_error)?;
    transaction
        .pragma_update(None, "application_id", APPLICATION_ID)
        .map_err(database_error)?;
    transaction
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}
