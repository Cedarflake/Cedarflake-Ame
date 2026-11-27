use rusqlite::Connection;

use crate::domain::ScanError;

use super::persistent_journal_baseline::validate_persistent_journal_baseline_contract_with_depth;
use super::{SCHEMA_VERSION, database_error};
use super::{
    catch_up_contract, change_lane_contract, inventory_spool_contract, journal_contract,
    metadata_inventory_contract, preview_expectation_repair, recovery_authority_contract,
    recovery_execution_contract, root_namespace_contract, scan_handoff_contract,
    scan_owner_contract, terminal_media_contract,
};
use super::{live_gap_contract, source_revision_contract};

#[derive(Clone, Copy)]
pub(super) enum ContractValidationDepth {
    StructureOnly,
    Full,
}

impl ContractValidationDepth {
    pub(super) fn includes_rows(self) -> bool {
        matches!(self, Self::Full)
    }

    pub(super) fn includes_active_rows(self) -> bool {
        self.includes_rows()
    }
}

pub(super) fn validate_current_schema_contract(connection: &Connection) -> Result<(), ScanError> {
    #[cfg(test)]
    CURRENT_SCHEMA_VALIDATION_COUNT.with(|count| count.set(count.get() + 1));
    validate_current_schema_contract_with_source_revision_rows(connection)
}

pub(super) fn validate_current_schema_contract_with_source_revision_rows(
    connection: &Connection,
) -> Result<(), ScanError> {
    validate_schema_version(connection, SCHEMA_VERSION)
}

pub(super) fn validate_schema_version(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    if schema_version != SCHEMA_VERSION {
        return Err(ScanError::new(
            "catalog_schema_unsupported",
            format!("Expected catalog schema {SCHEMA_VERSION}, found {schema_version}"),
        ));
    }
    if connection.is_autocommit() {
        let transaction = connection.unchecked_transaction().map_err(database_error)?;
        validate_in_snapshot(&transaction, schema_version)?;
        transaction.commit().map_err(database_error)
    } else {
        validate_in_snapshot(connection, schema_version)
    }
}

pub(super) fn validate_in_snapshot(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    validate_schema_structure(connection, schema_version)?;

    // The process session owns reuse. Only its initial validation (or explicit stale renewal)
    // reaches this full proof; ordinary connections retain bounded identity/schema checks.
    validate_pre_live_gap_schema_contract(connection, schema_version)?;
    live_gap_contract::validate(connection)?;
    super::compact_gallery_search::validate_rows(connection)?;
    super::organization_review_filters::validate_rows(connection)?;
    // The journal proof above checked all foreign keys in this same snapshot.
    super::classification::validate_content_rows(connection)?;
    super::classification_work::validate_content_rows(connection)?;
    super::classification_setup::validate_rows(connection)?;
    super::classification_availability::validate_rows(connection)?;
    super::classification_publications::validate_rows(connection)?;
    super::classification_maintenance::validate_rows(connection)?;
    if schema_version >= 84 {
        super::classification_display::validate_rows(connection)?;
    }
    super::similar_images::validate_rows(connection)?;
    super::album_icons::validate_rows(connection)?;
    super::file_operations::validate_rows(connection)?;
    super::scan_resume_inventory::validate_rows(connection)?;
    super::directory_identities::validate_rows(connection)?;
    super::journal_scope_recovery::validate_rows(connection)?;
    super::media_admission_rechecks::validate_rows(connection)?;
    super::metadata_fallback::validate_rows(connection)?;
    super::organization::validate_rows(connection)?;
    super::organization_transitions::validate_rows(connection)?;
    super::organization_reconciliation::validate_rows(connection)?;
    super::organization_admission::validate_rows(connection)?;
    super::organization_review::validate_rows(connection)?;
    super::organization_restoration::validate_rows(connection)?;
    super::organization_association_index::validate_rows(connection)?;
    super::organization_history::validate_rows(connection)?;
    super::organization_review_order::validate_rows(connection)?;
    super::sidebar_order::validate_rows(connection)?;
    super::organization_admission_clock::validate_rows(connection)?;
    source_revision_contract::validate_rows(connection)
}

pub(super) fn validate_current_schema_structure(connection: &Connection) -> Result<(), ScanError> {
    validate_schema_structure(connection, SCHEMA_VERSION)
}

pub(super) fn validate_schema_structure(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    // Reject missing or altered schema before queries interpret any authority rows. Successful
    // DDL markers cannot prove canonical journal payloads, cross-root ownership or generations.
    validate_pre_live_gap_schema_contract_with_depth(
        connection,
        schema_version,
        ContractValidationDepth::StructureOnly,
    )?;
    live_gap_contract::validate_with_depth(connection, ContractValidationDepth::StructureOnly)?;
    super::metadata_fallback::validate(connection)?;
    source_revision_contract::validate_structure(connection)?;
    super::scan_resume_inventory::validate_structure(connection)?;
    super::directory_identities::validate_structure(connection)?;
    super::journal_directory_namespace::validate(connection)?;
    super::journal_scope_recovery::validate(connection)?;
    super::media_admission_rechecks::validate(connection)?;
    super::queue_retirement_indexes::validate(connection)?;
    super::exact_duplicates::validate(connection)?;
    super::gallery_duplicates::validate(connection)?;
    super::exact_duplicate_review::validate(connection)?;
    super::exact_candidate_index_retirement::validate(connection)?;
    super::metadata_recheck_index::validate(connection)?;
    super::retained_gallery_membership::validate(connection)?;
    super::organization::validate(connection)?;
    super::organization_transitions::validate(connection)?;
    super::organization_index_retirement::validate(connection)?;
    super::organization_admission::validate(connection)?;
    super::organization_review::validate(connection)?;
    super::organization_native_recovery::validate(connection)?;
    super::organization_restoration::validate(connection)?;
    super::organization_association_index::validate(connection)?;
    super::organization_history::validate(connection)?;
    super::organization_review_order::validate(connection)?;
    super::sidebar_order::validate(connection)?;
    super::preferred_copy_index::validate(connection)?;
    super::organization_admission_clock::validate(connection)?;
    super::compact_gallery_search::validate(connection)?;
    super::camera_metadata::validate(connection)?;
    super::multiple_metadata_formats::validate(connection)?;
    super::classification::validate(connection)?;
    super::classification_work::validate(connection)?;
    super::classification_setup::validate(connection)?;
    super::classification_availability::validate(connection)?;
    super::classification_publications::validate(connection)?;
    super::classification_projection::validate(connection)?;
    if schema_version >= 84 {
        super::classification_display::validate(connection)?;
    }
    super::classification_maintenance::validate(connection)?;
    super::similar_images::validate(connection)?;
    super::album_icons::validate(connection)?;
    super::file_operations::validate(connection)?;
    super::file_operation_notifications::validate(connection)?;
    super::file_operation_exports::validate(connection, schema_version)?;
    super::organization_group_filters::validate(connection)?;
    validate_existing_source_metadata_marker(connection)
}

fn validate_existing_source_metadata_marker(connection: &Connection) -> Result<(), ScanError> {
    let marker_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE name = 'library_source_revision_metadata_contract')",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !marker_exists || !source_revision_contract::metadata_contract_is_complete(connection)? {
        return Err(ScanError::new(
            "catalog_source_revision_contract_unverifiable",
            "The catalog cannot prove its source metadata contract",
        ));
    }
    Ok(())
}

fn validate_scan_authority_with_depth(
    connection: &Connection,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    scan_owner_contract::validate_queue_authority(connection)?;
    scan_owner_contract::validate_recovery_marker(connection)?;
    catch_up_contract::validate_with_depth(connection, depth)?;
    scan_handoff_contract::validate_with_depth(connection, depth)?;
    let (has_scan_runs, has_single_scan_owner_index, has_scan_owner) = connection
        .query_row(
            "SELECT
               EXISTS(SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'scan_runs'),
               EXISTS(SELECT 1 FROM pragma_index_list('scan_runs')
                 WHERE name = 'scan_runs_one_active_root'
                   AND \"unique\" = 1 AND partial = 1),
               EXISTS(SELECT 1 FROM pragma_table_info('scan_runs')
                 WHERE name = 'scan_owner')",
            [],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            },
        )
        .map_err(database_error)?;
    if has_scan_runs && (!has_single_scan_owner_index || !has_scan_owner) {
        return Err(scan_owner_contract::unverifiable_recovery_contract());
    }
    if !preview_expectation_repair::marker_is_complete(connection)? {
        return Err(ScanError::new(
            "catalog_change_catch_up_contract_unverifiable",
            "The catalog preview repair authority is missing",
        ));
    }
    Ok(())
}

pub(super) fn validate_pre_live_gap_schema_contract(
    connection: &Connection,
    schema_version: i64,
) -> Result<(), ScanError> {
    validate_pre_live_gap_schema_contract_with_depth(
        connection,
        schema_version,
        ContractValidationDepth::Full,
    )
}

fn validate_pre_live_gap_schema_contract_with_depth(
    connection: &Connection,
    schema_version: i64,
    depth: ContractValidationDepth,
) -> Result<(), ScanError> {
    validate_scan_authority_with_depth(connection, depth)?;
    metadata_inventory_contract::validate_with_depth(connection, depth)?;
    terminal_media_contract::validate_with_depth(connection, depth)?;
    journal_contract::validate_with_depth(connection, depth)?;
    change_lane_contract::validate_with_depth(connection, depth)?;
    recovery_authority_contract::validate_with_depth(connection, depth)?;
    validate_persistent_journal_baseline_contract_with_depth(connection, depth)?;
    recovery_execution_contract::validate_with_depth(connection, depth)?;
    inventory_spool_contract::validate_version_with_depth(connection, schema_version, depth)?;
    root_namespace_contract::validate_with_depth(connection, depth)
}

#[cfg(test)]
std::thread_local! {
    static CURRENT_SCHEMA_VALIDATION_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
    static CURRENT_SCHEMA_ROW_AUDIT_COUNT: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_current_schema_validation_count() {
    CURRENT_SCHEMA_VALIDATION_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn current_schema_validation_count() -> usize {
    CURRENT_SCHEMA_VALIDATION_COUNT.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(super) fn reset_current_schema_row_audit_count() {
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn current_schema_row_audit_count() -> usize {
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(std::cell::Cell::get)
}

pub(super) fn record_current_schema_row_audit() {
    #[cfg(test)]
    CURRENT_SCHEMA_ROW_AUDIT_COUNT.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
mod tests;
