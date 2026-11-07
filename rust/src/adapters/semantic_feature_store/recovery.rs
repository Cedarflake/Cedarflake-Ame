use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, ErrorCode, OpenFlags, config::DbConfig};

use super::{
    ENTRY_ALLOWANCE, SqliteSemanticFeatureStore, checksum, database_error, invalid, ownership,
    storage,
};
use crate::adapters::{PreviewCacheNamespace, local_files::open_catalog_identity_guard};
use crate::domain::{
    ScanError,
    image_features::{AnalysisRevision, Embedding},
};

pub(super) fn recover(directory: &Path) -> Result<(), ScanError> {
    let namespace = PreviewCacheNamespace::for_operation(directory)?;
    let root = namespace
        .root()
        .ok_or_else(|| invalid("The feature directory is unavailable"))?;
    let path = root.join(storage::FILE_NAME);
    let file = match open_catalog_identity_guard(&path) {
        Ok((file, _)) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            SqliteSemanticFeatureStore::open(directory)?;
            return Ok(());
        }
        Err(error) => return Err(invalid(error.to_string())),
    };
    // Only a previously admitted physical file may be reset, including a damaged header.
    let _ownership = ownership::require(&namespace, &file)?;
    let mut connection = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(database_error)?;
    connection
        .set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)
        .map_err(database_error)?;
    match verify(&connection) {
        Ok(()) => return Ok(()),
        Err(error) if error.code == "semantic_storage_invalid" => {}
        Err(error) => return Err(error),
    }
    // SQLite owns reset locking and journaling. The namespace and file identity stay pinned;
    // the application must retire both analysis executors before entering this operation.
    connection
        .set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)
        .map_err(database_error)?;
    let reset = connection.execute_batch("VACUUM");
    let disabled = connection.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false);
    reset.map_err(database_error)?;
    disabled.map_err(database_error)?;
    storage::initialize_schema(&mut connection)?;
    verify(&connection)
}

fn verify(connection: &Connection) -> Result<(), ScanError> {
    let application_id: u32 = connection
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(validation_error)?;
    let schema: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(validation_error)?;
    if application_id != storage::APPLICATION_ID || schema != storage::SCHEMA_VERSION {
        return Err(invalid("The owned semantic store header is damaged"));
    }
    let integrity: String = connection
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .map_err(validation_error)?;
    if integrity != "ok" {
        return Err(invalid("The semantic feature database is damaged"));
    }
    let mut statement = connection
        .prepare(
            "SELECT substr(feature_key, 1, 33), dimensions, substr(vector, 1, 16385),
                substr(checksum, 1, 33), sequence
         FROM image_features ORDER BY sequence",
        )
        .map_err(validation_error)?;
    let mut rows = statement.query([]).map_err(validation_error)?;
    let mut count = 0_u64;
    let mut bytes = 0_u64;
    let mut sequence = 0_u64;
    while let Some(row) = rows.next().map_err(validation_error)? {
        let key: Vec<u8> = row.get(0).map_err(validation_error)?;
        let dimensions: i64 = row.get(1).map_err(validation_error)?;
        let payload: Vec<u8> = row.get(2).map_err(validation_error)?;
        let expected: Vec<u8> = row.get(3).map_err(validation_error)?;
        let current = super::unsigned(row, 4).map_err(validation_error)?;
        let key = AnalysisRevision::from_bytes(
            key.try_into().map_err(|_| invalid("Invalid feature key"))?,
        );
        if !(1..=4096).contains(&dimensions)
            || payload.len() != dimensions as usize * 4
            || checksum(key, &payload).bytes().as_slice() != expected
            || current <= sequence
        {
            return Err(invalid("The stored image feature is damaged"));
        }
        let values = payload
            .chunks_exact(4)
            .map(|value| f32::from_le_bytes(value.try_into().expect("fixed float")))
            .collect();
        Embedding::from_normalized_values(values).map_err(|error| invalid(error.to_string()))?;
        count += 1;
        bytes += payload.len() as u64 + ENTRY_ALLOWANCE;
        sequence = current;
    }
    let state = connection.query_row(
        "SELECT generation, feature_count, accounted_bytes FROM semantic_store_state WHERE singleton = 1",
        [], |row| Ok((super::unsigned(row, 0)?, super::unsigned(row, 1)?, super::unsigned(row, 2)?)),
    ).map_err(validation_error)?;
    if state != (sequence, count, bytes) {
        return Err(invalid("The semantic feature accounting is damaged"));
    }
    Ok(())
}

fn validation_error(error: rusqlite::Error) -> ScanError {
    match &error {
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase
            ) =>
        {
            invalid(error.to_string())
        }
        rusqlite::Error::SqlInputError { .. }
        | rusqlite::Error::QueryReturnedNoRows
        | rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::IntegralValueOutOfRange(..) => invalid(error.to_string()),
        _ => database_error(error),
    }
}
