use rusqlite::Connection;

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, SQLITE_APPLICATION_ID, database_error, unix_time_ms};

mod album_icons;
mod album_year_exports;
mod camera_metadata;
mod catch_up_contract;
mod change_lane_contract;
mod classification;
mod classification_availability;
mod classification_display;
mod classification_maintenance;
mod classification_merge;
mod classification_projection;
mod classification_publications;
mod classification_setup;
mod classification_work;
mod compact_gallery_search;
mod compatibility_repair_shape;
mod current_recovery_state;
mod current_schema;
mod directory_identities;
mod exact_candidate_index_retirement;
mod exact_duplicate_review;
mod exact_duplicates;
mod file_operation_exports;
mod file_operation_notifications;
mod file_operations;
mod foreground_scan_capture;
mod gallery_duplicates;
mod interrupted_foreground_repair;
mod inventory_spool_contract;
mod inventory_spool_retirement;
pub(super) mod inventory_spool_rows;
pub(super) mod journal_contract;
mod journal_directory_namespace;
mod journal_scope_recovery;
mod live_gap_contract;
mod location_columns;
mod media_admission_rechecks;
mod metadata_fallback;
mod metadata_inventory_contract;
mod metadata_recheck_index;
mod multiple_metadata_formats;
mod organization;
mod organization_admission;
mod organization_admission_clock;
mod organization_association_index;
mod organization_group_filters;
mod organization_history;
mod organization_index_retirement;
mod organization_native_recovery;
mod organization_reconciliation;
mod organization_restoration;
mod organization_review;
mod organization_review_filters;
mod organization_review_order;
mod organization_transitions;
mod persistent_journal_baseline;
mod persistent_journal_range_history;
mod preferred_copy_index;
mod preview_expectation_repair;
mod queue_retirement_indexes;
mod recovery_authority_contract;
mod recovery_execution_contract;
#[cfg(test)]
mod recovery_test_catalog;
mod replaced_namespace_authority;
mod retained_gallery_membership;
mod retained_source_state;
mod retired_live_gap_repair;
mod retired_root_authority;
mod root_namespace_contract;
mod scan_handoff_contract;
mod scan_owner_contract;
mod scan_resume_inventory;
mod schema_shape;
mod sidebar_order;
mod similar_images;
mod source_revision_contract;
mod terminal_inventory_repair;
mod terminal_journal_handoff;
mod terminal_media_contract;

mod initialization;
mod schema_definitions;

pub(super) use initialization::prepare_fresh_catalog_auto_vacuum;
use schema_definitions::*;

pub(super) fn initialize_schema(connection: &mut Connection) -> Result<(), ScanError> {
    initialization::initialize(connection)?;
    current_recovery_state::repair_and_validate(connection)?;
    interrupted_foreground_repair::recover(connection)
}

#[cfg(test)]
mod tests;
