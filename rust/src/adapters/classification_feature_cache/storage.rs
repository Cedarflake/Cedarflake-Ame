use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, Transaction, params};

use super::{
    ClassificationFeatureCacheStats, ClassificationRevision, PreviewCacheNamespace, ScanError,
    database_error, invalid,
};

pub(super) const APPLICATION_ID: i64 = 0x414D4643;
const SCHEMA_VERSION: i64 = 1;
pub(super) const FILE_NAME: &str = "classification-features.sqlite3";

pub(super) fn open(
    directory: &Path,
) -> Result<(Connection, File, PreviewCacheNamespace), ScanError> {
    let namespace = PreviewCacheNamespace::for_operation(directory)?;
    let root = namespace
        .root()
        .ok_or_else(|| invalid("The feature cache directory is unavailable"))?;
    let path = root.join(FILE_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => initialize(root, &path)?,
        Err(error) => {
            return Err(invalid(format!(
                "Could not inspect the feature cache: {error}"
            )));
        }
    }
    let (file, _) = super::super::local_files::open_catalog_identity_guard(&path)
        .map_err(|error| invalid(format!("The feature cache file is unavailable: {error}")))?;
    let mut header = [0; 72];
    (&file)
        .read_exact(&mut header)
        .map_err(|error| invalid(format!("Could not read the feature cache header: {error}")))?;
    let version = u32::from_be_bytes(header[60..64].try_into().expect("fixed header field"));
    let application = u32::from_be_bytes(header[68..72].try_into().expect("fixed header field"));
    if &header[..16] != b"SQLite format 3\0"
        || application != APPLICATION_ID as u32
        || version != SCHEMA_VERSION as u32
    {
        return Err(invalid(
            "This file is not a supported classification feature cache",
        ));
    }
    let connection = Connection::open_with_flags(
        &path,
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
            "The feature cache page size does not match its storage budget",
        ));
    }
    connection
        .execute_batch(
            "PRAGMA journal_mode = DELETE;
         PRAGMA trusted_schema = OFF;
         PRAGMA cache_size = -2048;
         PRAGMA max_page_count = 139264;
         PRAGMA journal_size_limit = 0;",
        )
        .map_err(database_error)?;
    stats(&connection)?;
    Ok((connection, file, namespace))
}

fn initialize(root: &Path, destination: &Path) -> Result<(), ScanError> {
    let staged = tempfile::Builder::new()
        .prefix("classification-feature-setup-")
        .suffix(".sqlite3")
        .tempfile_in(root)
        .map_err(|error| invalid(format!("Could not prepare the feature cache: {error}")))?;
    let mut connection = Connection::open(staged.path()).map_err(database_error)?;
    connection
        .execute_batch("PRAGMA page_size = 4096; PRAGMA auto_vacuum = INCREMENTAL;")
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
    transaction.commit().map_err(database_error)?;
    drop(connection);
    // Publish only a complete database; a concurrent creator retains its own committed cache.
    match staged.persist_noclobber(destination) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(invalid(format!(
            "Could not install the feature cache: {}",
            error.error
        ))),
    }
}

pub(super) fn stats(connection: &Connection) -> Result<ClassificationFeatureCacheStats, ScanError> {
    let (entries, bytes) = connection
        .query_row(
            "SELECT entry_count, accounted_bytes FROM feature_cache_state WHERE singleton = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(database_error)?;
    Ok(ClassificationFeatureCacheStats {
        entries: u64::try_from(entries)
            .map_err(|_| invalid("The cached feature count is invalid"))?,
        accounted_bytes: u64::try_from(bytes)
            .map_err(|_| invalid("The cached feature size is invalid"))?,
    })
}

pub(super) fn touch(
    transaction: &Transaction<'_>,
    key: ClassificationRevision,
) -> Result<(), ScanError> {
    transaction
        .execute(
            "UPDATE feature_cache_state SET clock = clock + 1 WHERE singleton = 1",
            [],
        )
        .map_err(database_error)?;
    transaction.execute(
        "UPDATE features SET touched = (SELECT clock FROM feature_cache_state WHERE singleton = 1) WHERE key = ?1",
        [key.bytes().as_slice()],
    ).map_err(database_error)?;
    Ok(())
}

pub(super) fn make_room(
    transaction: &Transaction<'_>,
    budget: u64,
    incoming: u64,
) -> Result<(), ScanError> {
    let used = stats(transaction)?.accounted_bytes;
    let required = used.saturating_add(incoming).saturating_sub(budget);
    if required == 0 {
        return Ok(());
    }
    let mut reclaimed = 0;
    let mut statement = transaction
        .prepare("SELECT key, length(payload) + 256 FROM features ORDER BY touched, key LIMIT 64")
        .map_err(database_error)?;
    let candidates = statement
        .query_map([], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(database_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(database_error)?;
    for (key, bytes) in candidates {
        transaction
            .execute("DELETE FROM features WHERE key = ?1", params![key])
            .map_err(database_error)?;
        reclaimed +=
            u64::try_from(bytes).map_err(|_| invalid("The cached feature size is invalid"))?;
        if reclaimed >= required {
            return Ok(());
        }
    }
    Err(invalid(
        "The feature cache could not reclaim its bounded insertion budget",
    ))
}
