use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::file_operation_journal::{FileOperationRecord, invalid};
use crate::domain::file_operations::FileAction;
use crate::domain::{AssetLocationView, ScanError};

use super::super::super::{
    allocate_source_generation, database_error, organization, source_revision_token, sqlite_integer,
};

pub(super) fn retain(
    transaction: &Transaction<'_>,
    record: &FileOperationRecord,
    target: Option<&AssetLocationView>,
) -> Result<(), ScanError> {
    if !matches!(
        record.authorization.request.action,
        FileAction::Move | FileAction::Rename
    ) {
        return Ok(());
    }
    let binding = &record.authorization.catalog;
    let source = &record.authorization.request.source.state;
    let old_identity = source
        .file_identity
        .as_ref()
        .ok_or_else(|| invalid("Missing original identity"))?;
    let old_revision = source
        .source_revision
        .as_ref()
        .ok_or_else(|| invalid("Missing original version"))?;
    let prior: Option<(String, Option<String>, Option<String>)> = transaction.query_row(
        "SELECT record_id, fingerprint_scheme, digest FROM organization_bindings
         WHERE asset_id = ?1 AND source_generation = ?2 AND byte_length = ?3
           AND file_identity_scheme = ?4 AND file_identity_value = ?5 AND source_revision_token = ?6",
        params![binding.asset_id, sqlite_integer(binding.source_generation, "original generation")?, sqlite_integer(source.file_size, "original size")?, old_identity.scheme, old_identity.value, source_revision_token(old_revision)?],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(database_error)?;
    let Some((record_id, scheme, digest)) = prior else {
        return Ok(());
    };
    let destination = record
        .checkpoint
        .receipt
        .as_ref()
        .and_then(|receipt| receipt.destination.as_ref())
        .ok_or_else(|| invalid("Missing verified transfer"))?;
    let identity = destination
        .state
        .file_identity
        .as_ref()
        .ok_or_else(|| invalid("Missing destination identity"))?;
    let revision = destination
        .state
        .source_revision
        .as_ref()
        .ok_or_else(|| invalid("Missing destination version"))?;
    let generation = match target {
        Some(target) => transaction.query_row(
            "SELECT source_generation FROM asset_locations WHERE location_id = ?1 AND asset_id = ?2
             AND scan_id = (SELECT active_scan_id FROM library_roots WHERE id = ?3)",
            params![target.location_id, binding.asset_id, target.root_id], |row| row.get(0),
        ).map_err(database_error)?,
        None => allocate_source_generation(transaction)?,
    };
    let conflicting: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM organization_bindings AS b
         JOIN organization_record_groups AS current ON current.record_id = b.record_id
         JOIN organization_record_groups AS original ON original.record_id = ?3
         WHERE b.asset_id = ?1 AND b.source_generation = ?2 AND current.group_id <> original.group_id)",
        params![binding.asset_id, generation, record_id], |row| row.get(0),
    ).map_err(database_error)?;
    if conflicting {
        return Err(invalid(
            "The destination has a different durable organization decision",
        ));
    }
    transaction.execute(
        "INSERT OR IGNORE INTO organization_bindings(record_id, asset_id, source_generation, byte_length,
           file_identity_scheme, file_identity_value, source_revision_token, fingerprint_scheme, digest)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![record_id, binding.asset_id, generation, sqlite_integer(destination.state.file_size, "destination size")?, identity.scheme, identity.value, source_revision_token(revision)?, scheme, digest],
    ).map_err(database_error)?;
    if let Some(target) = target {
        transaction
            .execute(
                "INSERT OR IGNORE INTO organization_sources(root_id, record_id) VALUES (?1, ?2)",
                params![target.root_id, record_id],
            )
            .map_err(database_error)?;
        organization::association_lifetime::restore_generation(transaction, generation)?;
    }
    Ok(())
}
