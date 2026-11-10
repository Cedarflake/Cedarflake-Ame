use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::file_operation_journal::{
    FileOperationEvent, FileOperationPhase, FileOperationRecord, invalid,
};
use crate::domain::file_operation_publication::FileOperationDestinationPublication;
use crate::domain::file_operations::FileAction;
use crate::domain::{IncrementalCatalogRoot, ScanError};
use crate::ports::IncrementalCatalogRepository;
use crate::ports::file_operation_catalog::FileOperationCatalog;

use super::super::{
    catalog_delta, database_error, exact_duplicate_review, load_catalog_revision, sqlite_integer,
};
use super::{SqliteFileOperationJournal, reads, writes};

mod roots;
mod transfer_binding;

impl FileOperationCatalog for SqliteFileOperationJournal {
    fn publication_roots(&self) -> Result<Vec<IncrementalCatalogRoot>, ScanError> {
        self.catalog.load_incremental_catalog_roots()
    }

    fn publish_operation(
        &mut self,
        id: u64,
        expected_version: u64,
        destination: Option<&FileOperationDestinationPublication>,
        now_unix_ms: i64,
    ) -> Result<FileOperationRecord, ScanError> {
        let transaction = self.catalog.begin_user_interactive_write()?;
        let record = reads::load(&transaction, id)?;
        if record.checkpoint.phase == FileOperationPhase::Published {
            return Ok(record);
        }
        if record.checkpoint.version != expected_version
            || record.checkpoint.phase != FileOperationPhase::NativeCompleted
        {
            return Err(invalid(
                "Only the current completed native result can update the catalog",
            ));
        }
        validate_destination(&record, destination)?;
        let binding = &record.authorization.catalog;
        let replacement = exact_duplicate_review::file_operations::capture(&transaction, &record)?;
        let mut affected_assets = HashSet::from([binding.asset_id.clone()]);
        let mut affected_previews = HashSet::new();
        let mut identity_generations = HashMap::new();
        let mut staging_generations = HashMap::new();
        if record.authorization.request.action != FileAction::Copy
            && destination.is_none_or(|target| {
                target.location.location_id != binding.location_id
                    || target.location.root_id != binding.root_id
            })
        {
            remove_source(
                &transaction,
                &record,
                &mut affected_assets,
                &mut affected_previews,
            )?;
        }
        let target = match destination {
            Some(destination) => {
                let root = roots::load(&transaction, &destination.root.root_id)?
                    .ok_or_else(|| invalid("The destination library is no longer available"))?;
                root.revalidate(&destination.root)?;
                roots::validate_location(&root, &destination.location)?;
                let location =
                    existing_target(&transaction, &root.scan_id, &record, &destination.location)?;
                let before = catalog_delta::load_affected_state(
                    &transaction,
                    &root.scan_id,
                    &root.id,
                    &HashSet::from([destination.location.location_id.clone()]),
                    &mut affected_assets,
                    &mut affected_previews,
                )?;
                catalog_delta::location_mutation::apply_location(
                    &transaction,
                    &root.scan_id,
                    root.running_scan_id.as_deref(),
                    &root.id,
                    catalog_delta::location_mutation::LocationChange {
                        remove_location_ids: &[],
                        upsert_location: Some(&location),
                    },
                    &mut identity_generations,
                    &mut staging_generations,
                )?;
                root.update_count(&transaction, 1 - before)?;
                affected_assets.insert(destination.location.asset_id.clone());
                Some(location)
            }
            None => None,
        };
        transfer_binding::retain(&transaction, &record, target.as_ref())?;
        exact_duplicate_review::file_operations::publish(
            &transaction,
            &record,
            replacement,
            target.as_ref(),
        )?;
        catalog_delta::mark_affected_preview_artifacts_stale(&transaction, &affected_previews)?;
        catalog_delta::delete_affected_orphan_assets(&transaction, &affected_assets)?;
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        let revision = load_catalog_revision(&transaction)?;
        let record = writes::advance(
            &transaction,
            id,
            expected_version,
            FileOperationEvent::Published(revision),
            now_unix_ms,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(record)
    }
}

fn validate_destination(
    record: &FileOperationRecord,
    destination: Option<&FileOperationDestinationPublication>,
) -> Result<(), ScanError> {
    let receipt = record
        .checkpoint
        .receipt
        .as_ref()
        .ok_or_else(|| invalid("Missing native result"))?;
    if let Some(target) = destination {
        let native = receipt
            .destination
            .as_ref()
            .ok_or_else(|| invalid("This result has no destination file"))?;
        let location = &target.location;
        if location.absolute_path != native.state.absolute_path
            || location.file_identity != native.state.file_identity
            || location.source_revision != native.state.source_revision
            || location.file_size != native.state.file_size
            || location.modified_unix_ms != native.state.modified_unix_ms
            || Path::new(&location.absolute_path).parent() != Some(Path::new(&native.parent.path))
            || matches!(
                record.authorization.request.action,
                FileAction::Move | FileAction::Rename
            ) && location.asset_id != record.authorization.catalog.asset_id
        {
            return Err(invalid(
                "Catalog publication changed the verified native destination",
            ));
        }
    }
    Ok(())
}

fn remove_source(
    transaction: &Transaction<'_>,
    record: &FileOperationRecord,
    assets: &mut HashSet<String>,
    previews: &mut HashSet<String>,
) -> Result<(), ScanError> {
    let binding = &record.authorization.catalog;
    let Some(root) = roots::load(transaction, &binding.root_id)? else {
        return Ok(());
    };
    let generation = sqlite_integer(binding.source_generation, "source generation")?;
    let source = &record.authorization.request.source.state;
    let identity = source
        .file_identity
        .as_ref()
        .ok_or_else(|| invalid("Missing source identity"))?;
    let moved_revision = record
        .checkpoint
        .receipt
        .as_ref()
        .and_then(|receipt| receipt.destination.as_ref())
        .filter(|destination| destination.state.file_identity == source.file_identity)
        .and_then(|destination| destination.state.source_revision.as_ref())
        .map(super::super::source_revision_token)
        .transpose()?;
    let matches = |scan: &str| -> Result<bool, ScanError> {
        transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_locations WHERE scan_id = ?1 AND location_id = ?2 AND asset_id = ?3
               AND file_identity_scheme = ?5 AND file_identity_value = ?6 AND file_size = ?7
               AND (source_generation = ?4 OR (?8 IS NOT NULL AND source_revision_token = ?8)))",
            params![scan, binding.location_id, binding.asset_id, generation, identity.scheme, identity.value,
                sqlite_integer(source.file_size, "source size")?, moved_revision], |row| row.get(0),
        ).map_err(database_error)
    };
    if !matches(&root.scan_id)? {
        return Ok(());
    }
    catalog_delta::load_affected_state(
        transaction,
        &root.scan_id,
        &root.id,
        &HashSet::from([binding.location_id.clone()]),
        assets,
        previews,
    )?;
    let running = match root.running_scan_id.as_deref() {
        Some(scan) if matches(scan)? => Some(scan),
        _ => None,
    };
    catalog_delta::location_removal::remove(
        transaction,
        &root.scan_id,
        running,
        &root.id,
        &binding.location_id,
    )?;
    root.update_count(transaction, -1)
}

fn existing_target(
    transaction: &Transaction<'_>,
    scan: &str,
    record: &FileOperationRecord,
    target: &crate::domain::AssetLocationView,
) -> Result<crate::domain::AssetLocationView, ScanError> {
    let mut target = target.clone();
    let identity = target
        .file_identity
        .as_ref()
        .ok_or_else(|| invalid("Missing destination identity"))?;
    let revision = target
        .source_revision
        .as_ref()
        .ok_or_else(|| invalid("Missing destination revision"))?;
    let observed: Option<(String, i64)> = transaction.query_row(
        "SELECT asset_id, source_generation FROM asset_locations WHERE scan_id = ?1 AND location_id = ?2
           AND file_identity_scheme = ?3 AND file_identity_value = ?4 AND source_revision_token = ?5
           AND file_size = ?6 AND modified_unix_ms = ?7",
        params![scan, target.location_id, identity.scheme, identity.value,
            super::super::source_revision_token(revision)?, sqlite_integer(target.file_size, "destination size")?, target.modified_unix_ms],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(database_error)?;
    if let Some((asset_id, generation)) = observed {
        target.source_generation =
            super::super::sqlite_unsigned(generation, "observed generation")?;
        if record.authorization.request.action == FileAction::Copy {
            target.asset_id = asset_id;
        }
    }
    Ok(target)
}
