CREATE TABLE schema_info (
               version INTEGER NOT NULL
             );

CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             , next_source_generation INTEGER NOT NULL DEFAULT 1
               CHECK(next_source_generation > 0));

CREATE TABLE library_roots (
               id TEXT PRIMARY KEY,
               path TEXT NOT NULL UNIQUE,
               active_scan_id TEXT,
               created_unix_ms INTEGER NOT NULL
             );

CREATE TABLE scan_runs (
               id TEXT PRIMARY KEY,
               root_id TEXT NOT NULL,
               status TEXT NOT NULL,
               scan_owner TEXT NOT NULL DEFAULT 'foreground'
                 CHECK(scan_owner IN ('foreground', 'authoritative_recovery')),
               started_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               asset_count INTEGER NOT NULL DEFAULT 0,
               issue_count INTEGER NOT NULL DEFAULT 0,
               max_items INTEGER,
               max_entries INTEGER,
               preview_edge INTEGER NOT NULL,
               current_directory_relative_path TEXT,
               current_directory_enumerated INTEGER NOT NULL DEFAULT 0
                 CHECK(current_directory_enumerated IN (0, 1)),
               last_visited_relative_path TEXT,
               visited_entries INTEGER NOT NULL DEFAULT 0,
               accepted_items INTEGER NOT NULL DEFAULT 0,
               requires_previous_snapshot INTEGER NOT NULL DEFAULT 0
                 CHECK(requires_previous_snapshot IN (0, 1)),
               root_generation_at_start INTEGER
                 CHECK(root_generation_at_start IS NULL OR root_generation_at_start > 0),
               change_queue_high_watermark INTEGER
                 CHECK(change_queue_high_watermark IS NULL OR change_queue_high_watermark > 0),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );

CREATE TABLE assets (
               id TEXT PRIMARY KEY,
               created_unix_ms INTEGER NOT NULL
             );

CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               file_local_time TEXT,
               parent_relative_path TEXT NOT NULL DEFAULT '',
               natural_name_key TEXT NOT NULL DEFAULT '',
               width INTEGER NOT NULL,
               height INTEGER NOT NULL,
               preview_status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL DEFAULT 'unknown',
               metadata_engine_version TEXT NOT NULL DEFAULT '0',
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT,
               file_identity_scheme TEXT,
               file_identity_value TEXT, source_revision_token TEXT, source_generation INTEGER, camera_make TEXT CHECK(camera_make IS NULL OR (
    typeof(camera_make) = 'text' AND length(CAST(camera_make AS BLOB)) BETWEEN 1 AND 128
)), camera_model TEXT CHECK(camera_model IS NULL OR (
    typeof(camera_model) = 'text' AND length(CAST(camera_model AS BLOB)) BETWEEN 1 AND 128
)), gallery_search_id INTEGER CHECK(gallery_search_id IS NULL OR (
    typeof(gallery_search_id) = 'integer' AND gallery_search_id > 0
)),
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               PRIMARY KEY(scan_id, location_id),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               FOREIGN KEY(asset_id) REFERENCES assets(id),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );

CREATE TABLE scan_issues (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               path TEXT,
               code TEXT NOT NULL,
               message TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );

CREATE TABLE scan_directory_frontier (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               UNIQUE(scan_id, relative_path)
             );

CREATE TABLE scan_directory_entries (
               scan_id TEXT NOT NULL,
               directory_relative_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(scan_id, directory_relative_path, relative_path),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );

CREATE TABLE preview_artifacts (
               artifact_key TEXT PRIMARY KEY,
               source_file_size INTEGER NOT NULL CHECK(source_file_size >= 0),
               source_modified_unix_ms INTEGER NOT NULL,
               source_identity_scheme TEXT,
               source_identity_value TEXT,
               algorithm_id TEXT NOT NULL,
               algorithm_version INTEGER NOT NULL CHECK(algorithm_version >= 0),
               orientation_contract TEXT NOT NULL,
               size_bucket INTEGER NOT NULL CHECK(size_bucket > 0),
               encoded_width INTEGER NOT NULL CHECK(encoded_width > 0),
               encoded_height INTEGER NOT NULL CHECK(encoded_height > 0),
               artifact_path TEXT NOT NULL UNIQUE,
               byte_size INTEGER NOT NULL CHECK(byte_size >= 0),
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('ready', 'stale', 'evictable')),
               created_unix_ms INTEGER NOT NULL,
               last_used_unix_ms INTEGER NOT NULL, source_revision_token TEXT, source_generation INTEGER,
               CHECK(
                 (source_identity_scheme IS NULL AND source_identity_value IS NULL)
                 OR
                 (source_identity_scheme IS NOT NULL AND source_identity_value IS NOT NULL)
               )
             );

CREATE TABLE preview_artifact_locations (
               artifact_key TEXT NOT NULL,
               location_id TEXT NOT NULL,
               PRIMARY KEY(artifact_key, location_id),
               FOREIGN KEY(artifact_key) REFERENCES preview_artifacts(artifact_key)
                 ON DELETE CASCADE
             );

CREATE TABLE library_change_queue_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               root_authority_complete INTEGER NOT NULL
                 CHECK(root_authority_complete = 1)
             , authoritative_recovery_complete INTEGER NOT NULL DEFAULT 1
               CHECK(authoritative_recovery_complete = 1), scan_ownership_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_ownership_complete = 1), change_catch_up_complete INTEGER NOT NULL DEFAULT 1
               CHECK(change_catch_up_complete = 1), scan_catch_up_lineage_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_catch_up_lineage_complete = 1), scan_handoff_batch_complete INTEGER NOT NULL DEFAULT 1
               CHECK(scan_handoff_batch_complete = 1));

CREATE TABLE library_change_root_state (
               root_id TEXT PRIMARY KEY,
               generation INTEGER NOT NULL CHECK(generation > 0),
               is_active INTEGER NOT NULL CHECK(is_active IN (0, 1)),
               updated_unix_ms INTEGER NOT NULL
             , last_consistency_audit_unix_ms INTEGER);

CREATE TABLE library_change_catch_up_state (
               volume_id TEXT PRIMARY KEY CHECK(length(volume_id) BETWEEN 1 AND 512),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 32),
               next_usn TEXT NOT NULL CHECK(length(next_usn) BETWEEN 1 AND 32),
               root_set_fingerprint TEXT NOT NULL
                 CHECK(length(root_set_fingerprint) = 64),
               catalog_revision INTEGER NOT NULL CHECK(catalog_revision >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
             );

CREATE TABLE library_change_catch_up_handoffs (
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               file_identity_scheme TEXT NOT NULL,
               file_identity_value TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               source_location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL CHECK(width >= 0),
               height INTEGER NOT NULL CHECK(height >= 0),
               preview_status TEXT NOT NULL
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL,
               metadata_engine_version TEXT NOT NULL,
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0), source_revision_token TEXT, source_generation INTEGER, camera_make TEXT CHECK(camera_make IS NULL OR (
    typeof(camera_make) = 'text' AND length(CAST(camera_make AS BLOB)) BETWEEN 1 AND 128
)), camera_model TEXT CHECK(camera_model IS NULL OR (
    typeof(camera_model) = 'text' AND length(CAST(camera_model AS BLOB)) BETWEEN 1 AND 128
)),
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               PRIMARY KEY(
                 catch_up_source, catch_up_watermark,
                 file_identity_scheme, file_identity_value
               )
             );

CREATE TABLE scan_run_catch_up_lineage (
               scan_id TEXT NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(scan_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
             );

CREATE TABLE library_change_scan_handoff_batches (
               id TEXT NOT NULL PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 1024),
               source_root_id TEXT NOT NULL CHECK(length(source_root_id) BETWEEN 1 AND 4096),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
             );

CREATE TABLE library_change_scan_handoff_lineage (
               batch_id TEXT NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(batch_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(batch_id) REFERENCES library_change_scan_handoff_batches(id)
                 ON DELETE CASCADE
             );

CREATE TABLE library_change_scan_handoff_items (
               batch_id TEXT NOT NULL,
               file_identity_scheme TEXT NOT NULL,
               file_identity_value TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               source_location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               created_unix_ms INTEGER,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL CHECK(width >= 0),
               height INTEGER NOT NULL CHECK(height >= 0),
               preview_status TEXT NOT NULL
                 CHECK(preview_status IN ('pending', 'ready', 'failed')),
               preview_issue_code TEXT,
               preview_issue_message TEXT,
               metadata_engine_id TEXT NOT NULL,
               metadata_engine_version TEXT NOT NULL,
               capture_local_time TEXT,
               capture_offset_minutes INTEGER,
               capture_time_source TEXT
                 CHECK(capture_time_source IS NULL OR capture_time_source IN (
                   'exif_original', 'exif_digitized', 'exif_datetime'
                 )),
               capture_raw_value TEXT, source_revision_token TEXT, source_generation INTEGER, camera_make TEXT CHECK(camera_make IS NULL OR (
    typeof(camera_make) = 'text' AND length(CAST(camera_make AS BLOB)) BETWEEN 1 AND 128
)), camera_model TEXT CHECK(camera_model IS NULL OR (
    typeof(camera_model) = 'text' AND length(CAST(camera_model AS BLOB)) BETWEEN 1 AND 128
)),
               CHECK(
                 (capture_local_time IS NULL AND capture_time_source IS NULL
                   AND capture_raw_value IS NULL)
                 OR
                 (capture_local_time IS NOT NULL AND capture_time_source IS NOT NULL
                   AND capture_raw_value IS NOT NULL)
               ),
               PRIMARY KEY(batch_id, file_identity_scheme, file_identity_value),
               FOREIGN KEY(batch_id) REFERENCES library_change_scan_handoff_batches(id)
                 ON DELETE CASCADE
             );

CREATE TABLE library_change_preview_repair_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE "library_change_queue" (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               intent_kind TEXT NOT NULL CHECK(intent_kind IN (
                 'reconcile', 'rename_candidate', 'freshness_unknown'
               )),
               scope TEXT NOT NULL CHECK(scope IN ('path', 'subtree', 'root')),
               relative_path TEXT NOT NULL,
               previous_relative_path TEXT,
               origin TEXT NOT NULL CHECK(origin IN (
                 'live_notification', 'metadata_inventory', 'startup_catch_up',
                 'user_refresh', 'consistency_audit'
               )),
               first_observed_unix_ms INTEGER NOT NULL,
               most_recent_observed_unix_ms INTEGER NOT NULL,
               first_sequence TEXT NOT NULL CHECK(length(first_sequence) > 0),
               most_recent_sequence TEXT NOT NULL CHECK(length(most_recent_sequence) > 0),
               coalesced_observation_count INTEGER NOT NULL
                 CHECK(coalesced_observation_count > 0),
               status TEXT NOT NULL CHECK(status IN (
                 'pending', 'leased', 'retry_wait', 'completed', 'superseded'
               )),
               ready_unix_ms INTEGER NOT NULL,
               attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
               next_retry_unix_ms INTEGER,
               lease_generation INTEGER NOT NULL DEFAULT 0 CHECK(lease_generation >= 0),
               lease_expires_unix_ms INTEGER,
               last_failure_code TEXT,
               last_failure_message TEXT,
               catalog_revision_at_enqueue INTEGER NOT NULL
                 CHECK(catalog_revision_at_enqueue >= 0),
               catalog_revision_at_success INTEGER
                 CHECK(catalog_revision_at_success IS NULL OR catalog_revision_at_success >= 0),
               catch_up_source TEXT,
               catch_up_watermark TEXT,
               authoritative_scan_id TEXT,
               superseded_by_change_id INTEGER,
               created_unix_ms INTEGER NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               CHECK(first_observed_unix_ms <= most_recent_observed_unix_ms),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 (status = 'leased' AND lease_expires_unix_ms IS NOT NULL)
                 OR
                 (status <> 'leased' AND lease_expires_unix_ms IS NULL)
               ),
               CHECK(status = 'retry_wait' OR next_retry_unix_ms IS NULL),
               CHECK(
                 (status = 'completed' AND catalog_revision_at_success IS NOT NULL)
                 OR
                 (status <> 'completed' AND catalog_revision_at_success IS NULL)
               ),
               FOREIGN KEY(superseded_by_change_id) REFERENCES "library_change_queue"(id)
                 ON DELETE SET NULL
             );

CREATE TABLE "library_change_queue_catch_up_lineage" (
               change_id INTEGER NOT NULL,
               catch_up_source TEXT NOT NULL CHECK(length(catch_up_source) BETWEEN 1 AND 128),
               catch_up_watermark TEXT NOT NULL
                 CHECK(length(catch_up_watermark) BETWEEN 1 AND 1024),
               enrolled_unix_ms INTEGER NOT NULL,
               PRIMARY KEY(change_id, catch_up_source, catch_up_watermark),
               FOREIGN KEY(change_id) REFERENCES "library_change_queue"(id) ON DELETE CASCADE
             );

CREATE TABLE library_metadata_inventory_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE library_metadata_inventory_runs (
               id TEXT NOT NULL PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 256),
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               epoch INTEGER NOT NULL CHECK(epoch > 0),
               scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
               scope_relative_path TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN (
                 'running', 'comparing', 'completed', 'failed', 'cancelled', 'superseded'
               )),
               next_page_index INTEGER NOT NULL CHECK(next_page_index > 0),
               enumeration_cursor TEXT,
               comparison_cursor TEXT,
               absence_cursor TEXT,
               staged_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(staged_entry_count >= 0),
               candidate_count INTEGER NOT NULL DEFAULT 0 CHECK(candidate_count >= 0),
               enumeration_complete INTEGER NOT NULL DEFAULT 0
                 CHECK(enumeration_complete IN (0, 1)),
               absence_authority INTEGER NOT NULL DEFAULT 0
                 CHECK(absence_authority IN (0, 1)),
               started_unix_ms INTEGER NOT NULL,
               updated_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               last_issue_code TEXT,
               last_issue_message TEXT,
               CHECK(
                 (scope_kind = 'root' AND scope_relative_path = '')
                 OR
                 (scope_kind = 'subtree' AND length(scope_relative_path) > 0)
               ),
               CHECK(instr(scope_relative_path, char(92)) = 0),
               CHECK(
                 (last_issue_code IS NULL AND last_issue_message IS NULL)
                 OR
                 (last_issue_code IS NOT NULL AND last_issue_message IS NOT NULL)
               ),
               CHECK(enumeration_complete = 1 OR absence_authority = 0),
               CHECK(
                 (status = 'completed' AND completed_unix_ms IS NOT NULL
                   AND enumeration_complete = 1 AND absence_authority = 1)
                 OR
                 (status <> 'completed' AND completed_unix_ms IS NULL)
               ),
               UNIQUE(root_id, root_generation, epoch),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );

CREATE TABLE library_metadata_inventory_entries (
               run_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               entry_kind TEXT NOT NULL CHECK(entry_kind IN ('file', 'directory', 'other')),
               file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0),
               modified_unix_ms INTEGER NOT NULL,
               file_identity_scheme TEXT,
               file_identity_value TEXT,
               placeholder_state TEXT NOT NULL CHECK(placeholder_state IN (
                 'available', 'offline', 'recall_on_open', 'recall_on_data_access'
               )),
               is_reparse_point INTEGER NOT NULL CHECK(is_reparse_point IN (0, 1)),
               staged_page_index INTEGER NOT NULL CHECK(staged_page_index > 0),
               comparison_status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(comparison_status IN ('pending', 'unchanged', 'enqueued')),
               candidate_previous_relative_path TEXT,
               staged_unix_ms INTEGER NOT NULL, source_revision_token TEXT,
               CHECK(length(relative_path) > 0),
               CHECK(instr(relative_path, char(92)) = 0),
               CHECK(
                 (entry_kind = 'file' AND file_size IS NOT NULL)
                 OR
                 (entry_kind <> 'file' AND file_size IS NULL)
               ),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               CHECK(
                 candidate_previous_relative_path IS NULL
                 OR comparison_status = 'enqueued'
               ),
               PRIMARY KEY(run_id, relative_path),
               FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id)
                 ON DELETE CASCADE
             );

CREATE TABLE library_terminal_media_evidence_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE library_terminal_media_evidence (
               root_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               file_size INTEGER NOT NULL CHECK(file_size >= 0),
               modified_unix_ms INTEGER NOT NULL,
               file_identity_scheme TEXT,
               file_identity_value TEXT,
               inspection_engine_id TEXT NOT NULL,
               inspection_engine_version INTEGER NOT NULL
                 CHECK(inspection_engine_version > 0),
               issue_code TEXT NOT NULL,
               issue_message TEXT NOT NULL,
               updated_unix_ms INTEGER NOT NULL, source_revision_token TEXT, source_generation INTEGER,
               CHECK(length(relative_path) > 0),
               CHECK(instr(relative_path, char(92)) = 0),
               CHECK(length(inspection_engine_id) BETWEEN 1 AND 128),
               CHECK(length(issue_code) BETWEEN 1 AND 128),
               CHECK(length(issue_message) BETWEEN 1 AND 4096),
               CHECK(
                 (file_identity_scheme IS NULL AND file_identity_value IS NULL)
                 OR
                 (file_identity_scheme IS NOT NULL AND file_identity_value IS NOT NULL)
               ),
               PRIMARY KEY(root_id, relative_path),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE library_persistent_journal_root_state (
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               protocol_version INTEGER NOT NULL
                 CHECK(protocol_version BETWEEN 0 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               capability_state TEXT NOT NULL CHECK(capability_state IN (
                 'unknown', 'supported', 'live_only'
               )),
               continuity_state TEXT NOT NULL CHECK(continuity_state IN (
                 'baseline_required', 'catching_up', 'current', 'recovery_required',
                 'live_only', 'unavailable'
               )),
               last_failure_code TEXT,
               last_failure_message TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 (capability_state = 'unknown' AND protocol_version = 0)
                 OR
                 (capability_state <> 'unknown' AND protocol_version > 0)
               ),
               CHECK(capability_state <> 'supported' OR last_failure_code IS NULL),
               CHECK(
                 continuity_state <> 'current'
                 OR (capability_state = 'supported' AND last_failure_code IS NULL)
               ),
               PRIMARY KEY(root_id, root_generation),
               FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_checkpoints (
               root_id TEXT PRIMARY KEY,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               root_reference_version INTEGER NOT NULL
                 CHECK(root_reference_version IN (2, 3)),
               root_file_reference BLOB NOT NULL,
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               next_unread_usn TEXT NOT NULL CHECK(length(next_unread_usn) BETWEEN 1 AND 19),
               captured_exclusive_end TEXT NOT NULL
                 CHECK(length(captured_exclusive_end) BETWEEN 1 AND 19),
               covered_catalog_revision INTEGER NOT NULL
                 CHECK(covered_catalog_revision >= 0),
               protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               continuity_state TEXT NOT NULL CHECK(continuity_state IN (
                 'catching_up', 'current', 'recovery_required', 'live_only', 'unavailable'
               )),
               last_failure_code TEXT,
               last_failure_message TEXT,
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (root_reference_version = 2 AND length(root_file_reference) = 8)
                 OR
                 (root_reference_version = 3 AND length(root_file_reference) = 16)
               ),
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(next_unread_usn NOT GLOB '*[^0-9]*'),
               CHECK(captured_exclusive_end NOT GLOB '*[^0-9]*'),
               CHECK(CAST(next_unread_usn AS INTEGER) <= CAST(captured_exclusive_end AS INTEGER)),
               CHECK(
                 (last_failure_code IS NULL AND last_failure_message IS NULL)
                 OR
                 (last_failure_code IS NOT NULL AND last_failure_message IS NOT NULL)
               ),
               CHECK(
                 continuity_state <> 'current'
                 OR (
                   next_unread_usn = captured_exclusive_end
                   AND last_failure_code IS NULL
                 )
               ),
               FOREIGN KEY(root_id, root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
                 ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_source_ranges (
               id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
               root_id TEXT NOT NULL,
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               requested_start_usn TEXT NOT NULL
                 CHECK(length(requested_start_usn) BETWEEN 1 AND 19),
               requested_end_usn TEXT NOT NULL
                 CHECK(length(requested_end_usn) BETWEEN 1 AND 19),
               covered_until_usn TEXT NOT NULL
                 CHECK(length(covered_until_usn) BETWEEN 1 AND 19),
               is_complete INTEGER NOT NULL CHECK(is_complete IN (0, 1)),
               protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
               contract_version INTEGER NOT NULL CHECK(contract_version = 1),
               status TEXT NOT NULL CHECK(status IN (
                 'enrolled', 'checkpointed', 'superseded'
               )),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               checkpointed_unix_ms INTEGER CHECK(checkpointed_unix_ms >= 0), canonical_payload BLOB NOT NULL DEFAULT X'',
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(requested_start_usn NOT GLOB '*[^0-9]*'),
               CHECK(requested_end_usn NOT GLOB '*[^0-9]*'),
               CHECK(covered_until_usn NOT GLOB '*[^0-9]*'),
               CHECK(
                 CAST(requested_start_usn AS INTEGER) < CAST(requested_end_usn AS INTEGER)
                 AND CAST(requested_start_usn AS INTEGER) < CAST(covered_until_usn AS INTEGER)
                 AND CAST(covered_until_usn AS INTEGER) <= CAST(requested_end_usn AS INTEGER)
               ),
               CHECK(
                 is_complete = (
                   CAST(covered_until_usn AS INTEGER) = CAST(requested_end_usn AS INTEGER)
                 )
               ),
               CHECK(
                 (status = 'checkpointed' AND checkpointed_unix_ms IS NOT NULL)
                 OR
                 (status <> 'checkpointed' AND checkpointed_unix_ms IS NULL)
               ),
               FOREIGN KEY(root_id, root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
                 ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_queue_lineage (
               source_range_id TEXT NOT NULL,
               change_id INTEGER NOT NULL,
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               PRIMARY KEY(source_range_id, change_id),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_cross_root_lineage (
               id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               file_reference_version INTEGER NOT NULL
                 CHECK(file_reference_version IN (2, 3)),
               file_reference BLOB NOT NULL,
               previous_root_id TEXT NOT NULL,
               previous_root_generation INTEGER NOT NULL
                 CHECK(previous_root_generation > 0),
               previous_relative_path TEXT NOT NULL,
               current_root_id TEXT NOT NULL,
               current_root_generation INTEGER NOT NULL CHECK(current_root_generation > 0),
               current_relative_path TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'superseded')),
               created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0), journal_id TEXT, old_usn TEXT, new_usn TEXT, previous_carry_id TEXT,
               CHECK(previous_root_id <> current_root_id),
               CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
               CHECK(length(current_relative_path) BETWEEN 1 AND 32767),
               CHECK(instr(previous_relative_path, char(92)) = 0),
               CHECK(instr(current_relative_path, char(92)) = 0),
               CHECK(
                 (file_reference_version = 2 AND length(file_reference) = 8)
                 OR
                 (file_reference_version = 3 AND length(file_reference) = 16)
               ),
               FOREIGN KEY(previous_root_id, previous_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation),
               FOREIGN KEY(current_root_id, current_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
             );

CREATE TABLE library_persistent_journal_cross_root_ranges (
               lineage_id TEXT NOT NULL,
               source_range_id TEXT NOT NULL,
               participant_role TEXT NOT NULL CHECK(participant_role IN ('previous', 'current')),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               PRIMARY KEY(lineage_id, source_range_id),
               FOREIGN KEY(lineage_id)
                 REFERENCES library_persistent_journal_cross_root_lineage(id) ON DELETE CASCADE,
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_range_lifecycle (
               source_range_id TEXT PRIMARY KEY,
               lifecycle_state TEXT NOT NULL
                 CHECK(lifecycle_state IN ('pending', 'completed', 'superseded')),
               completed_unix_ms INTEGER CHECK(completed_unix_ms >= 0),
               updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
               CHECK(
                 (lifecycle_state = 'completed' AND completed_unix_ms IS NOT NULL)
                 OR
                 (lifecycle_state <> 'completed' AND completed_unix_ms IS NULL)
               ),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_pending_renames (
               carry_id TEXT PRIMARY KEY CHECK(length(carry_id) BETWEEN 1 AND 512),
               source_range_id TEXT NOT NULL,
               volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
               volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
               journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
               file_reference_version INTEGER NOT NULL
                 CHECK(file_reference_version IN (2, 3)),
               file_reference BLOB NOT NULL,
               old_usn TEXT NOT NULL CHECK(length(old_usn) BETWEEN 1 AND 19),
               previous_root_id TEXT NOT NULL,
               previous_root_generation INTEGER NOT NULL CHECK(previous_root_generation > 0),
               previous_relative_path TEXT NOT NULL,
               is_directory INTEGER NOT NULL CHECK(is_directory IN (0, 1)),
               enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
               CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
               CHECK(old_usn NOT GLOB '*[^0-9]*'),
               CHECK(length(previous_relative_path) BETWEEN 1 AND 32767),
               CHECK(instr(previous_relative_path, char(92)) = 0),
               CHECK(
                 (file_reference_version = 2 AND length(file_reference) = 8)
                 OR
                 (file_reference_version = 3 AND length(file_reference) = 16)
               ),
               FOREIGN KEY(source_range_id)
                 REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
               FOREIGN KEY(previous_root_id, previous_root_generation)
                 REFERENCES library_persistent_journal_root_state(root_id, root_generation)
             );

CREATE TABLE library_change_lane_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE library_change_queue_lanes (
               change_id INTEGER PRIMARY KEY,
               lane TEXT NOT NULL CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery')),
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );

CREATE TABLE library_recovery_authority_contract (
               singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
               complete INTEGER NOT NULL CHECK(complete = 1)
             );

CREATE TABLE library_recovery_authorities (
               change_id INTEGER PRIMARY KEY,
               run_id TEXT NOT NULL UNIQUE CHECK(length(run_id) BETWEEN 1 AND 512),
               root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
               root_generation INTEGER NOT NULL CHECK(root_generation > 0),
               reason TEXT NOT NULL CHECK(reason IN (
                 'existing_root_baseline', 'first_import_boundary', 'journal_gap',
                 'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
                 'containment_failure', 'broker_after_current_failure', 'watcher_uncovered_gap'
               )),
               opening_journal_id TEXT,
               opening_next_usn TEXT,
               authorized_unix_ms INTEGER NOT NULL,
               retired_unix_ms INTEGER,
               CHECK(
                 (reason IN ('existing_root_baseline', 'first_import_boundary')
                   AND opening_journal_id IS NOT NULL AND opening_next_usn IS NOT NULL)
                 OR
                 (reason NOT IN ('existing_root_baseline', 'first_import_boundary')
                   AND opening_journal_id IS NULL AND opening_next_usn IS NULL)
               ),
               CHECK(retired_unix_ms IS NULL OR retired_unix_ms >= authorized_unix_ms),
               FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
             );

CREATE TABLE library_persistent_journal_baselines (
       change_id INTEGER PRIMARY KEY,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
       volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
       root_reference_version INTEGER NOT NULL CHECK(root_reference_version IN (2, 3)),
       root_file_reference BLOB NOT NULL,
       journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
       opening_next_usn TEXT NOT NULL CHECK(length(opening_next_usn) BETWEEN 1 AND 19),
       closing_next_usn TEXT CHECK(length(closing_next_usn) BETWEEN 1 AND 19),
       protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       phase TEXT NOT NULL CHECK(phase IN ('inventory', 'replay', 'absence', 'completed')),
       authorized_unix_ms INTEGER NOT NULL CHECK(authorized_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= authorized_unix_ms),
       completed_unix_ms INTEGER CHECK(completed_unix_ms >= updated_unix_ms),
       CHECK(
         (root_reference_version = 2 AND length(root_file_reference) = 8)
         OR
         (root_reference_version = 3 AND length(root_file_reference) = 16)
       ),
       CHECK(journal_id <> '0' AND journal_id NOT GLOB '*[^0-9]*'),
       CHECK(opening_next_usn NOT GLOB '*[^0-9]*'),
       CHECK(closing_next_usn IS NULL OR closing_next_usn NOT GLOB '*[^0-9]*'),
       CHECK(
         (phase = 'inventory' AND closing_next_usn IS NULL)
         OR
         (phase <> 'inventory' AND closing_next_usn IS NOT NULL)
       ),
       CHECK(
         closing_next_usn IS NULL
         OR CAST(closing_next_usn AS INTEGER) >= CAST(opening_next_usn AS INTEGER)
       ),
       CHECK((phase = 'completed') = (completed_unix_ms IS NOT NULL)),
       FOREIGN KEY(change_id)
         REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE,
       FOREIGN KEY(root_id, root_generation)
         REFERENCES library_persistent_journal_root_state(root_id, root_generation)
         ON DELETE CASCADE
     );

CREATE TABLE library_recovery_execution_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     );

CREATE TABLE library_metadata_inventory_candidate_owners (
       run_id TEXT NOT NULL,
       candidate_key TEXT NOT NULL CHECK(length(candidate_key) BETWEEN 1 AND 1024),
       change_id INTEGER NOT NULL,
       candidate_role TEXT NOT NULL CHECK(candidate_role IN ('present', 'absence')),
       relative_path TEXT NOT NULL CHECK(length(relative_path) BETWEEN 1 AND 32767),
       previous_relative_path TEXT,
       owned_unix_ms INTEGER NOT NULL CHECK(owned_unix_ms >= 0),
       CHECK(instr(relative_path, char(92)) = 0),
       CHECK(previous_relative_path IS NULL OR (
         length(previous_relative_path) BETWEEN 1 AND 32767
         AND instr(previous_relative_path, char(92)) = 0
       )),
       PRIMARY KEY(run_id, candidate_key),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE RESTRICT
     );

CREATE TABLE library_metadata_inventory_frontier (
       run_id TEXT NOT NULL,
       ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
       relative_directory TEXT NOT NULL CHECK(length(relative_directory) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('pending', 'enumerating', 'completed')),
       directory_identity_scheme TEXT,
       directory_identity_value TEXT,
       resume_after_relative_path TEXT,
       enumerated_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(enumerated_entry_count >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
       CHECK(instr(relative_directory, char(92)) = 0),
       CHECK(resume_after_relative_path IS NULL OR (
         length(resume_after_relative_path) BETWEEN 1 AND 32767
         AND instr(resume_after_relative_path, char(92)) = 0
       )),
       CHECK(
         (directory_identity_scheme IS NULL AND directory_identity_value IS NULL)
         OR
         (length(directory_identity_scheme) BETWEEN 1 AND 128
           AND length(directory_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, ordinal),
       UNIQUE(run_id, relative_directory),
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_runs(id) ON DELETE CASCADE
     );

CREATE TABLE library_root_publication_namespace_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     );

CREATE TABLE library_root_publication_namespaces (
       root_id TEXT PRIMARY KEY,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
       identity_value TEXT NOT NULL CHECK(length(identity_value) = 49),
       authority_kind TEXT NOT NULL CHECK(authority_kind IN (
         'journal_v3_migration', 'metadata_inventory', 'foreground_scan'
       )),
       established_catalog_revision INTEGER NOT NULL
         CHECK(established_catalog_revision >= 0),
       established_unix_ms INTEGER NOT NULL CHECK(established_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= established_unix_ms),
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     );

CREATE TABLE library_scan_publication_namespace_bindings (
       scan_id TEXT PRIMARY KEY,
       root_id TEXT NOT NULL,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
       identity_value TEXT NOT NULL CHECK(length(identity_value) = 49),
       bound_unix_ms INTEGER NOT NULL CHECK(bound_unix_ms >= 0),
       FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     );

CREATE TABLE library_live_gap_recovery_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     );

CREATE TABLE library_live_gap_recovery_claims (
       gap_change_id INTEGER PRIMARY KEY,
       root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       consumer_kind TEXT NOT NULL CHECK(consumer_kind IN (
          'pending_journal', 'journal_source_range',
          'metadata_inventory_control', 'explicit_recovery_required',
          'foreground_scan'
        )),
       opening_volume_guid TEXT,
       opening_volume_serial TEXT,
       opening_root_reference_version INTEGER,
       opening_root_file_reference BLOB,
       opening_journal_id TEXT,
       opening_next_usn TEXT,
       protocol_version INTEGER,
       contract_version INTEGER,
       source_range_id TEXT,
       recovery_change_id INTEGER,
       foreground_scan_id TEXT,
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       consumed_unix_ms INTEGER,
       CHECK(
         (consumer_kind IN ('pending_journal', 'journal_source_range')
           AND opening_volume_guid IS NOT NULL
           AND opening_volume_serial IS NOT NULL
           AND opening_root_reference_version IN (2, 3)
           AND opening_root_file_reference IS NOT NULL
           AND opening_journal_id IS NOT NULL
           AND opening_next_usn IS NOT NULL
           AND protocol_version BETWEEN 1 AND 65535
           AND contract_version = 1)
         OR
         (consumer_kind = 'metadata_inventory_control'
           AND (
             (opening_volume_guid IS NULL
               AND opening_volume_serial IS NULL
               AND opening_root_reference_version IS NULL
               AND opening_root_file_reference IS NULL
               AND opening_journal_id IS NULL
               AND opening_next_usn IS NULL
               AND protocol_version IS NULL
               AND contract_version IS NULL)
             OR
             (opening_volume_guid IS NOT NULL
               AND opening_volume_serial IS NOT NULL
               AND opening_root_reference_version IN (2, 3)
               AND opening_root_file_reference IS NOT NULL
               AND opening_journal_id IS NOT NULL
               AND opening_next_usn IS NOT NULL
               AND protocol_version BETWEEN 1 AND 65535
               AND contract_version = 1)
           ))
         OR
          (consumer_kind IN ('explicit_recovery_required', 'foreground_scan')
            AND opening_volume_guid IS NULL
           AND opening_volume_serial IS NULL
           AND opening_root_reference_version IS NULL
           AND opening_root_file_reference IS NULL
           AND opening_journal_id IS NULL
           AND opening_next_usn IS NULL
           AND protocol_version IS NULL
           AND contract_version IS NULL)
       ),
       CHECK(
         opening_root_reference_version IS NULL
         OR (opening_root_reference_version = 2 AND length(opening_root_file_reference) = 8)
         OR (opening_root_reference_version = 3 AND length(opening_root_file_reference) = 16)
       ),
       CHECK(opening_volume_serial IS NULL OR (
         length(opening_volume_serial) BETWEEN 1 AND 20
         AND opening_volume_serial NOT GLOB '*[^0-9]*'
       )),
       CHECK(opening_journal_id IS NULL OR (
         length(opening_journal_id) BETWEEN 1 AND 20
         AND opening_journal_id <> '0'
         AND opening_journal_id NOT GLOB '*[^0-9]*'
       )),
       CHECK(opening_next_usn IS NULL OR (
         length(opening_next_usn) BETWEEN 1 AND 19
         AND opening_next_usn NOT GLOB '*[^0-9]*'
       )),
       CHECK(
          (consumer_kind = 'pending_journal'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NULL)
         OR
          (consumer_kind = 'journal_source_range'
            AND source_range_id IS NOT NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NOT NULL)
         OR
          (consumer_kind = 'metadata_inventory_control'
            AND source_range_id IS NULL AND recovery_change_id IS NOT NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NOT NULL)
         OR
          (consumer_kind = 'explicit_recovery_required'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NULL
            AND consumed_unix_ms IS NULL)
          OR
          (consumer_kind = 'foreground_scan'
            AND source_range_id IS NULL AND recovery_change_id IS NULL
            AND foreground_scan_id IS NOT NULL)
       ),
       CHECK(consumed_unix_ms IS NULL OR consumed_unix_ms >= created_unix_ms),
       FOREIGN KEY(gap_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
       FOREIGN KEY(source_range_id)
         REFERENCES library_persistent_journal_source_ranges(id) ON DELETE RESTRICT,
       FOREIGN KEY(recovery_change_id) REFERENCES library_change_queue(id) ON DELETE RESTRICT,
       FOREIGN KEY(foreground_scan_id) REFERENCES scan_runs(id) ON DELETE RESTRICT
      );

CREATE TABLE library_source_revision_metadata_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     );

CREATE TABLE "library_metadata_inventory_spools" (
    run_id TEXT NOT NULL PRIMARY KEY CHECK(length(run_id) BETWEEN 1 AND 256),
    authority_change_id INTEGER NOT NULL,
    root_id TEXT NOT NULL CHECK(length(root_id) BETWEEN 1 AND 512),
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    root_identity_scheme TEXT NOT NULL CHECK(length(root_identity_scheme) BETWEEN 1 AND 128),
    root_identity_value TEXT NOT NULL CHECK(length(root_identity_value) BETWEEN 1 AND 512),
    scope_kind TEXT NOT NULL CHECK(scope_kind IN ('root', 'subtree')),
    scope_relative_path TEXT NOT NULL CHECK(length(scope_relative_path) <= 32767),
    state TEXT NOT NULL CHECK(state IN ('enumerating', 'ready', 'retired')),
    created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
    updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= created_unix_ms),
    CHECK(instr(scope_relative_path, char(92)) = 0),
    CHECK(scope_kind = 'root' OR length(scope_relative_path) > 0)
);

CREATE TABLE "library_metadata_inventory_spool_directories" (
       run_id TEXT NOT NULL,
       ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
       relative_directory TEXT NOT NULL CHECK(length(relative_directory) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('pending', 'enumerating', 'resetting', 'completed')),
       directory_identity_scheme TEXT,
       directory_identity_value TEXT,
       source_entry_count INTEGER NOT NULL DEFAULT 0 CHECK(source_entry_count >= 0),
       created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= created_unix_ms),
       CHECK(instr(relative_directory, char(92)) = 0),
       CHECK(
         (directory_identity_scheme IS NULL AND directory_identity_value IS NULL)
         OR
         (length(directory_identity_scheme) BETWEEN 1 AND 128
           AND length(directory_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, relative_directory),
       UNIQUE(run_id, ordinal),
       FOREIGN KEY(run_id) REFERENCES "library_metadata_inventory_spools"(run_id)
         ON DELETE RESTRICT
     );

CREATE TABLE "library_metadata_inventory_spool_entries" (
       run_id TEXT NOT NULL,
       directory_relative_path TEXT,
       relative_path TEXT NOT NULL CHECK(length(relative_path) BETWEEN 1 AND 32767),
       entry_kind TEXT NOT NULL CHECK(entry_kind IN ('file', 'directory', 'other')),
       file_size INTEGER CHECK(file_size IS NULL OR file_size >= 0),
       modified_unix_ms INTEGER NOT NULL,
       file_identity_scheme TEXT,
       file_identity_value TEXT,
       placeholder_state TEXT NOT NULL CHECK(placeholder_state IN (
         'available', 'offline', 'recall_on_open', 'recall_on_data_access'
       )),
       is_reparse_point INTEGER NOT NULL CHECK(is_reparse_point IN (0, 1)),
       staged_unix_ms INTEGER NOT NULL CHECK(staged_unix_ms >= 0),
       source_revision_token TEXT,
       CHECK(directory_relative_path IS NULL OR instr(directory_relative_path, char(92)) = 0),
       CHECK(instr(relative_path, char(92)) = 0),
       CHECK(
         (entry_kind = 'file' AND file_size IS NOT NULL)
         OR
         (entry_kind <> 'file' AND file_size IS NULL)
       ),
       CHECK(
         (file_identity_scheme IS NULL AND file_identity_value IS NULL)
         OR
         (length(file_identity_scheme) BETWEEN 1 AND 128
           AND length(file_identity_value) BETWEEN 1 AND 512)
       ),
       PRIMARY KEY(run_id, relative_path),
       FOREIGN KEY(run_id, directory_relative_path)
         REFERENCES "library_metadata_inventory_spool_directories"(run_id, relative_directory)
         ON DELETE RESTRICT
     );

CREATE TABLE library_metadata_inventory_spool_contract (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    contract_version INTEGER NOT NULL CHECK(contract_version = 4),
    complete INTEGER NOT NULL CHECK(complete = 1)
);

CREATE TABLE scan_resume_pending_locations (
            scan_id TEXT NOT NULL,
            location_id TEXT NOT NULL,
            PRIMARY KEY(scan_id, location_id),
            FOREIGN KEY(scan_id, location_id)
                REFERENCES asset_locations(scan_id, location_id) ON DELETE CASCADE
        ) WITHOUT ROWID;

CREATE TABLE scan_directory_identities (
    scan_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
    identity_value TEXT NOT NULL CHECK(
        length(identity_value) = 49 AND substr(identity_value, 17, 1) = ':'
        AND substr(identity_value, 1, 16) NOT GLOB '*[^0-9a-f]*'
        AND substr(identity_value, 18, 32) NOT GLOB '*[^0-9a-f]*'
    ),
    PRIMARY KEY(scan_id, relative_path),
    FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
);

CREATE TABLE scan_directory_journal_authority (
    scan_id TEXT PRIMARY KEY NOT NULL,
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    journal_id BLOB NOT NULL CHECK(typeof(journal_id) = 'blob' AND length(journal_id) = 8
        AND journal_id != X'0000000000000000'),
    covered_until_usn INTEGER NOT NULL CHECK(covered_until_usn >= 0),
    FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE
);

CREATE TABLE library_journal_scope_recoveries (
    parent_change_id INTEGER PRIMARY KEY NOT NULL,
    current_change_id INTEGER NOT NULL UNIQUE,
    previous_change_id INTEGER UNIQUE,
    CHECK(parent_change_id != current_change_id),
    CHECK(previous_change_id IS NULL OR (previous_change_id != parent_change_id
        AND previous_change_id != current_change_id)),
    FOREIGN KEY(parent_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
    FOREIGN KEY(current_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE,
    FOREIGN KEY(previous_change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
);

CREATE TABLE library_media_admission_rechecks (
    root_id TEXT NOT NULL,
    root_generation INTEGER NOT NULL CHECK(root_generation > 0),
    relative_path TEXT NOT NULL,
    location_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    file_identity_scheme TEXT,
    file_identity_value TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending', 'queued', 'rejected')),
    change_id INTEGER,
    PRIMARY KEY(root_id, relative_path),
    CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
    FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE,
    FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE,
    FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE SET NULL
);

CREATE TABLE exact_analysis_runs (
        run_id TEXT PRIMARY KEY NOT NULL,
        scheme TEXT NOT NULL,
        engine_id TEXT NOT NULL,
        engine_version TEXT NOT NULL,
        parameters TEXT NOT NULL,
        started_unix_ms INTEGER NOT NULL
    );

CREATE TABLE exact_analysis_outcomes (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES exact_analysis_runs(run_id),
        phase TEXT NOT NULL CHECK(phase IN ('completed', 'cancelled', 'failed')),
        checked_files INTEGER NOT NULL CHECK(checked_files >= 0),
        fingerprinted_files INTEGER NOT NULL CHECK(fingerprinted_files >= 0),
        skipped_files INTEGER NOT NULL CHECK(skipped_files >= 0),
        bytes_read INTEGER NOT NULL CHECK(bytes_read >= 0),
        issue_code TEXT,
        finished_unix_ms INTEGER NOT NULL
    );

CREATE TABLE content_fingerprints (
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        scheme TEXT NOT NULL,
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length > 0),
        file_identity_scheme TEXT,
        file_identity_value TEXT,
        source_revision_token TEXT,
        run_id TEXT NOT NULL REFERENCES exact_analysis_runs(run_id),
        PRIMARY KEY(source_generation, scheme),
        CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL))
    );

CREATE TABLE gallery_content_fingerprints (
    source_generation INTEGER NOT NULL,
    scheme TEXT NOT NULL,
    PRIMARY KEY(source_generation, scheme),
    FOREIGN KEY(source_generation, scheme)
      REFERENCES content_fingerprints(source_generation, scheme)
);

CREATE TABLE exact_duplicate_decisions (
    scheme TEXT NOT NULL,
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
    preferred_location_id TEXT,
    preferred_asset_id TEXT,
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    updated_unix_ms INTEGER NOT NULL,
    CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)),
    PRIMARY KEY(scheme, digest, byte_length)
);

CREATE TABLE gallery_retained_fingerprints (
    source_generation INTEGER NOT NULL CHECK(source_generation > 0),
    evidence_source_generation INTEGER NOT NULL,
    scheme TEXT NOT NULL,
    PRIMARY KEY(source_generation, scheme),
    CHECK(source_generation <> evidence_source_generation),
    FOREIGN KEY(evidence_source_generation, scheme)
      REFERENCES gallery_content_fingerprints(source_generation, scheme) ON DELETE CASCADE
);

CREATE TABLE library_metadata_fallbacks (
    change_id INTEGER PRIMARY KEY,
    admitted_unix_ms INTEGER NOT NULL CHECK(admitted_unix_ms >= 0),
    FOREIGN KEY(change_id) REFERENCES library_recovery_authorities(change_id) ON DELETE CASCADE
);

CREATE TABLE organization_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0)
    );

CREATE TABLE organization_albums (
        album_id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 128),
        revision INTEGER NOT NULL CHECK(revision > 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
    );

CREATE TABLE organization_records (
        record_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT,
        disposition TEXT NOT NULL CHECK(disposition IN ('pending', 'deferred', 'organized', 'ignored')),
        favorite INTEGER NOT NULL CHECK(favorite IN (0, 1)),
        organization_revision INTEGER NOT NULL CHECK(organization_revision >= 0),
        favorite_revision INTEGER NOT NULL CHECK(favorite_revision >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
        unassociated_since_unix_ms INTEGER CHECK(unassociated_since_unix_ms >= 0),
        CHECK((disposition = 'organized') = (album_id IS NOT NULL)),
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_bindings (
        record_id TEXT NOT NULL,
        asset_id TEXT NOT NULL,
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
        file_identity_scheme TEXT,
        file_identity_value TEXT,
        source_revision_token TEXT,
        fingerprint_scheme TEXT,
        digest TEXT CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
        CHECK((fingerprint_scheme IS NULL) = (digest IS NULL)),
        PRIMARY KEY(asset_id, source_generation),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    );

CREATE TABLE organization_sources (
        root_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        PRIMARY KEY(root_id, record_id),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    );

CREATE TABLE organization_conflicts (
        record_id TEXT NOT NULL,
        album_id TEXT NOT NULL,
        decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0),
        PRIMARY KEY(record_id, album_id),
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_operations (
        operation_id TEXT PRIMARY KEY NOT NULL,
        request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
        operation_kind TEXT NOT NULL CHECK(operation_kind IN ('favorite', 'organization', 'album', 'undo', 'restore', 'cleanup')),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0)
    );

CREATE TABLE organization_operation_changes (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        previous_album_id TEXT,
        previous_disposition TEXT CHECK(previous_disposition IN ('pending', 'deferred', 'organized', 'ignored')),
        previous_favorite INTEGER CHECK(previous_favorite IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        CHECK((field = 'favorite' AND previous_favorite IS NOT NULL AND previous_disposition IS NULL
                 AND previous_album_id IS NULL)
           OR (field = 'organization' AND previous_favorite IS NULL AND previous_disposition IS NOT NULL
                 AND (previous_disposition = 'organized') = (previous_album_id IS NOT NULL))),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE
    );

CREATE TABLE organization_record_groups (
        record_id TEXT PRIMARY KEY NOT NULL,
        group_id TEXT NOT NULL,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(group_id) REFERENCES organization_records(record_id)
    );

CREATE TABLE organization_album_retirements (
        album_id TEXT PRIMARY KEY NOT NULL,
        revision INTEGER NOT NULL CHECK(revision > 0),
        retired_unix_ms INTEGER NOT NULL CHECK(retired_unix_ms >= 0),
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_album_operation_changes (
        operation_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT NOT NULL,
        previous_name TEXT,
        previous_retired INTEGER NOT NULL CHECK(previous_retired IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_operation_conflicts (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        album_id TEXT NOT NULL,
        decision_revision INTEGER NOT NULL CHECK(decision_revision >= 0),
        PRIMARY KEY(operation_id, record_id, album_id),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(record_id) REFERENCES organization_records(record_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_operation_targets (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE
    );

CREATE TABLE organization_admissions (
           admission_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(admission_id > 0),
           source_generation INTEGER NOT NULL UNIQUE CHECK(source_generation > 0)
         );

CREATE TABLE organization_admission_roots (
           root_key INTEGER PRIMARY KEY CHECK(root_key > 0),
           root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE
         );

CREATE TABLE organization_admission_sources (
           admission_id INTEGER NOT NULL REFERENCES organization_admissions(admission_id) ON DELETE CASCADE,
           root_key INTEGER NOT NULL REFERENCES organization_admission_roots(root_key) ON DELETE CASCADE,
           PRIMARY KEY(admission_id, root_key)
         ) WITHOUT ROWID;

CREATE TABLE organization_review_scopes (
       slot TEXT PRIMARY KEY NOT NULL CHECK(slot IN ('active', 'bookmark')),
       collection TEXT NOT NULL CHECK(collection IN ('library', 'favorites', 'album')),
       album_id TEXT,
       root_id TEXT,
       folder_relative_path TEXT,
       include_descendants INTEGER NOT NULL CHECK(include_descendants IN (0, 1)),
       search_text TEXT NOT NULL CHECK(length(search_text) <= 512),
       queue TEXT NOT NULL CHECK(queue IN ('pending', 'deferred', 'ignored')),
       CHECK((collection = 'album') = (album_id IS NOT NULL))
     );

CREATE TABLE organization_review_session (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       scope_slot TEXT NOT NULL DEFAULT 'active' CHECK(scope_slot = 'active')
         REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
       session_id TEXT NOT NULL,
       position_sequence INTEGER NOT NULL CHECK(position_sequence >= 0),
       checkpoint_digest TEXT CHECK(length(checkpoint_digest) = 64)
     );

CREATE TABLE organization_review_bookmark (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       scope_slot TEXT NOT NULL DEFAULT 'bookmark' CHECK(scope_slot = 'bookmark')
         REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
       record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
       admission_id INTEGER NOT NULL CHECK(admission_id > 0),
       root_id TEXT NOT NULL,
       location_id TEXT NOT NULL,
       after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)),
       updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0)
     );

CREATE TABLE organization_review_operations (
       operation_id TEXT PRIMARY KEY NOT NULL
         REFERENCES organization_operations(operation_id) ON DELETE CASCADE,
       session_id TEXT NOT NULL,
       position_sequence INTEGER NOT NULL CHECK(position_sequence > 0),
       committed_sequence INTEGER NOT NULL CHECK(committed_sequence = position_sequence + 1)
     );

CREATE TABLE organization_archive_restores (
          archive_id TEXT PRIMARY KEY NOT NULL,
          checksum TEXT NOT NULL CHECK(length(checksum) = 64 AND checksum NOT GLOB '*[^0-9a-f]*'),
          operation_id TEXT NOT NULL UNIQUE REFERENCES organization_operations(operation_id),
          revision_base INTEGER NOT NULL CHECK(revision_base > 0),
          source_revision INTEGER NOT NULL CHECK(source_revision >= 0),
          restored_unix_ms INTEGER NOT NULL CHECK(restored_unix_ms >= 0)
        );

CREATE TABLE organization_restored_fields (
          record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
          field TEXT NOT NULL CHECK(field IN ('organization', 'favorite')),
          applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
          PRIMARY KEY(record_id, field)
        ) WITHOUT ROWID;

CREATE TABLE organization_archive_generations (
          archive_id TEXT NOT NULL REFERENCES organization_archive_restores(archive_id),
          source_generation INTEGER NOT NULL CHECK(source_generation > 0),
          local_generation INTEGER NOT NULL UNIQUE CHECK(local_generation > 0),
          PRIMARY KEY(archive_id, source_generation)
        ) WITHOUT ROWID;

CREATE TABLE organization_recovery_roots (
          root_id TEXT PRIMARY KEY NOT NULL,
          archive_id TEXT NOT NULL REFERENCES organization_archive_restores(archive_id),
          original_root_id TEXT NOT NULL, path TEXT NOT NULL,
          identity_scheme TEXT, identity_value TEXT,
          CHECK((identity_scheme IS NULL) = (identity_value IS NULL)),
          UNIQUE(archive_id, original_root_id)
        );

CREATE TABLE organization_recovery_versions (
          location_id TEXT PRIMARY KEY NOT NULL, asset_id TEXT NOT NULL, root_id TEXT NOT NULL,
          source_generation INTEGER NOT NULL CHECK(source_generation > 0),
          byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
          file_identity_scheme TEXT, file_identity_value TEXT, source_revision_token TEXT,
          fingerprint_scheme TEXT, digest TEXT,
          native_copy_count INTEGER NOT NULL CHECK(native_copy_count > 0),
          CHECK((file_identity_scheme IS NULL) = (file_identity_value IS NULL)),
          CHECK((fingerprint_scheme IS NULL) = (digest IS NULL)),
          CHECK(digest IS NULL OR (length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'))
        );

CREATE TABLE organization_recovery_admissions (
          source_generation INTEGER PRIMARY KEY CHECK(source_generation > 0),
          admission_id INTEGER NOT NULL UNIQUE CHECK(admission_id > 0)
        );

CREATE TABLE organization_recovery_admission_sources (
          source_generation INTEGER NOT NULL REFERENCES organization_recovery_admissions(source_generation) ON DELETE CASCADE,
          root_id TEXT NOT NULL, PRIMARY KEY(source_generation, root_id)
        ) WITHOUT ROWID;

CREATE TABLE organization_recovery_admission_links (
          source_generation INTEGER PRIMARY KEY REFERENCES organization_admissions(source_generation) ON DELETE CASCADE,
          recovered_generation INTEGER NOT NULL REFERENCES organization_recovery_admissions(source_generation) ON DELETE CASCADE
        );

CREATE TABLE organization_recovery_duplicate_decisions (
          scheme TEXT NOT NULL, digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
          byte_length INTEGER NOT NULL CHECK(byte_length >= 0), preferred_location_id TEXT, preferred_asset_id TEXT,
          ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)), updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          unassociated_since_unix_ms INTEGER NOT NULL CHECK(unassociated_since_unix_ms >= 0),
          CHECK((preferred_location_id IS NULL) = (preferred_asset_id IS NULL)), PRIMARY KEY(scheme, digest, byte_length)
        ) WITHOUT ROWID;

CREATE TABLE organization_recovery_duplicate_sources (
          scheme TEXT NOT NULL, digest TEXT NOT NULL, byte_length INTEGER NOT NULL, root_id TEXT NOT NULL,
          FOREIGN KEY(scheme, digest, byte_length) REFERENCES organization_recovery_duplicate_decisions(scheme, digest, byte_length) ON DELETE CASCADE,
          PRIMARY KEY(scheme, digest, byte_length, root_id)
        ) WITHOUT ROWID;

CREATE TABLE organization_unassociated_records (
           record_id TEXT PRIMARY KEY NOT NULL,
           group_id TEXT NOT NULL,
           FOREIGN KEY(record_id) REFERENCES organization_record_groups(record_id) ON DELETE CASCADE
         ) WITHOUT ROWID;

CREATE TABLE organization_history_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        cursor INTEGER NOT NULL CHECK(cursor >= 0),
        started_revision INTEGER NOT NULL CHECK(started_revision >= 0)
    );

CREATE TABLE organization_history_entries (
        position INTEGER PRIMARY KEY CHECK(position > 0),
        operation_id TEXT UNIQUE NOT NULL,
        FOREIGN KEY(operation_id) REFERENCES organization_operations(operation_id) ON DELETE CASCADE
    );

CREATE TABLE organization_field_authority (
        operation_id TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('favorite', 'organization')),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        previous_authority INTEGER NOT NULL CHECK(previous_authority >= 0 AND previous_authority < applied_revision),
        applied_authority INTEGER NOT NULL CHECK(applied_authority >= 0 AND applied_authority <= applied_revision),
        PRIMARY KEY(operation_id, record_id, field),
        FOREIGN KEY(operation_id, record_id, field)
            REFERENCES organization_operation_changes(operation_id, record_id, field) ON DELETE CASCADE
    );

CREATE TABLE organization_album_authority (
        operation_id TEXT PRIMARY KEY NOT NULL,
        album_id TEXT NOT NULL,
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        previous_authority INTEGER NOT NULL CHECK(previous_authority >= 0 AND previous_authority < applied_revision),
        applied_authority INTEGER NOT NULL CHECK(applied_authority >= 0 AND applied_authority <= applied_revision),
        FOREIGN KEY(operation_id) REFERENCES organization_album_operation_changes(operation_id) ON DELETE CASCADE,
        FOREIGN KEY(album_id) REFERENCES organization_albums(album_id)
    );

CREATE TABLE organization_review_sort (
           scope_slot TEXT PRIMARY KEY NOT NULL
             REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
           sort_key TEXT NOT NULL CHECK(sort_key IN (
             'capture_time', 'created_time', 'modified_time', 'file_name', 'library_admission')),
           sort_direction TEXT NOT NULL CHECK(sort_direction IN ('ascending', 'descending'))
         );

CREATE TABLE organization_review_cursor (
           singleton INTEGER PRIMARY KEY CHECK(singleton = 1)
             REFERENCES organization_review_bookmark(singleton) ON DELETE CASCADE,
           primary_missing INTEGER NOT NULL CHECK(primary_missing IN (0, 1)),
           primary_text TEXT NOT NULL,
           primary_number INTEGER NOT NULL
         );

CREATE TABLE library_sidebar_root_order (
           root_id TEXT PRIMARY KEY NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
           position INTEGER NOT NULL CHECK(typeof(position) = 'integer' AND position >= 0)
         );

CREATE TABLE organization_sidebar_album_order (
           album_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_albums(album_id) ON DELETE CASCADE,
           position INTEGER NOT NULL CHECK(typeof(position) = 'integer' AND position >= 0)
         );

CREATE TABLE organization_admission_clock (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    restored_prefix INTEGER NOT NULL CHECK (restored_prefix >= 0)
);

CREATE VIRTUAL TABLE gallery_search_terms USING fts5(
    absolute_path, relative_path, tokenize='trigram', content='', contentless_delete=1,
    detail=none
);

CREATE TABLE classification_categories (
        category_id TEXT PRIMARY KEY NOT NULL CHECK(length(category_id) BETWEEN 1 AND 64),
        label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 80),
        revision INTEGER NOT NULL CHECK(revision >= 0)
    );

CREATE TABLE classification_category_versions (
        category_id TEXT NOT NULL REFERENCES classification_categories(category_id),
        semantic_revision TEXT NOT NULL CHECK(length(semantic_revision) = 64),
        descriptions TEXT NOT NULL CHECK(json_valid(descriptions) AND json_type(descriptions) = 'array'
            AND json_array_length(descriptions) BETWEEN 1 AND 8 AND length(CAST(descriptions AS BLOB)) <= 65536),
        PRIMARY KEY(category_id, semantic_revision)
    ) WITHOUT ROWID;

CREATE TABLE classification_rules (
        rules_revision TEXT PRIMARY KEY NOT NULL CHECK(length(rules_revision) = 64)
    );

CREATE TABLE classification_rule_members (
        rules_revision TEXT NOT NULL REFERENCES classification_rules(rules_revision),
        category_id TEXT NOT NULL,
        semantic_revision TEXT NOT NULL,
        PRIMARY KEY(rules_revision, category_id),
        FOREIGN KEY(category_id, semantic_revision)
            REFERENCES classification_category_versions(category_id, semantic_revision)
    ) WITHOUT ROWID;

CREATE TABLE classification_runs (
        run_id TEXT PRIMARY KEY NOT NULL CHECK(length(run_id) BETWEEN 1 AND 128),
        primary_rules TEXT NOT NULL REFERENCES classification_rules(rules_revision),
        configuration_revision TEXT NOT NULL CHECK(length(configuration_revision) = 64),
        payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB)) <= 4096),
        UNIQUE(run_id, configuration_revision)
    );

CREATE TABLE classification_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        active_rules TEXT REFERENCES classification_rules(rules_revision),
        configuration_revision TEXT CHECK(configuration_revision IS NULL OR length(configuration_revision) = 64),
        active_run_id TEXT,
        CHECK(active_run_id IS NULL OR configuration_revision IS NOT NULL),
        FOREIGN KEY(active_run_id, configuration_revision) REFERENCES classification_runs(run_id, configuration_revision)
    );

CREATE TABLE classification_predictions (
        publication_id INTEGER PRIMARY KEY AUTOINCREMENT,
        prediction_id TEXT UNIQUE NOT NULL CHECK(length(prediction_id) BETWEEN 1 AND 128),
        run_id TEXT NOT NULL,
        configuration_revision TEXT NOT NULL,
        fingerprint_scheme TEXT NOT NULL,
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
        payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB)) <= 131072),
        FOREIGN KEY(run_id, configuration_revision) REFERENCES classification_runs(run_id, configuration_revision)
    );

CREATE TABLE classification_decisions (
        record_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        category_id TEXT REFERENCES classification_categories(category_id),
        choice TEXT CHECK(choice IS NULL OR (json_valid(choice) AND length(CAST(choice AS BLOB)) <= 4096)),
        choice_revision INTEGER NOT NULL CHECK(choice_revision >= 0),
        choice_restored INTEGER NOT NULL CHECK(choice_restored IN (0, 1)),
        deferred INTEGER NOT NULL CHECK(deferred IN (0, 1)),
        review_revision INTEGER NOT NULL CHECK(review_revision >= 0),
        review_restored INTEGER NOT NULL CHECK(review_restored IN (0, 1)),
        CHECK((category_id IS NULL) = (choice IS NULL)),
        CHECK(choice IS NULL OR choice_revision > 0),
        CHECK(deferred = 0 OR review_revision > 0)
    );

CREATE TABLE classification_operations (
        operation_id TEXT PRIMARY KEY NOT NULL CHECK(length(operation_id) BETWEEN 1 AND 128),
        request_digest TEXT NOT NULL CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
        operation_kind TEXT NOT NULL CHECK(operation_kind IN ('configure', 'category', 'defer', 'undo', 'restore', 'continuation')),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        changed_groups INTEGER NOT NULL CHECK(changed_groups >= 0),
        created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
        restored INTEGER NOT NULL DEFAULT 0 CHECK(restored IN (0, 1))
    );

CREATE TABLE classification_operation_changes (
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        field TEXT NOT NULL CHECK(field IN ('category', 'review')),
        previous_payload TEXT CHECK(previous_payload IS NULL OR (json_valid(previous_payload)
            AND length(CAST(previous_payload AS BLOB)) <= 4096)),
        previous_revision INTEGER NOT NULL CHECK(previous_revision >= 0),
        previous_restored INTEGER NOT NULL CHECK(previous_restored IN (0, 1)),
        applied_revision INTEGER NOT NULL CHECK(applied_revision > 0),
        PRIMARY KEY(operation_id, record_id, field),
        CHECK(field <> 'review' OR (previous_payload IS NOT NULL AND previous_payload IN ('true', 'false')))
    ) WITHOUT ROWID;

CREATE TABLE classification_operation_targets (
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        record_id TEXT NOT NULL,
        field TEXT NOT NULL CHECK(field IN ('category', 'review')),
        PRIMARY KEY(operation_id, record_id, field)
    ) WITHOUT ROWID;

CREATE TABLE classification_review_bookmark (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        record_id TEXT NOT NULL REFERENCES organization_records(record_id) ON DELETE CASCADE,
        scope TEXT NOT NULL CHECK(json_valid(scope) AND length(CAST(scope AS BLOB)) <= 8192),
        admission_id INTEGER NOT NULL CHECK(admission_id > 0),
        root_id TEXT NOT NULL,
        location_id TEXT NOT NULL,
        primary_missing INTEGER NOT NULL CHECK(primary_missing IN (0, 1)),
        primary_text TEXT NOT NULL CHECK(length(primary_text) <= 32768),
        primary_number INTEGER NOT NULL,
        after_current INTEGER NOT NULL CHECK(after_current IN (0, 1)),
        revision INTEGER NOT NULL CHECK(revision > 0)
    );

CREATE TABLE classification_review_session (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        session_id TEXT NOT NULL CHECK(length(session_id) BETWEEN 1 AND 128),
        position_sequence INTEGER NOT NULL CHECK(position_sequence >= 0),
        scope TEXT NOT NULL CHECK(json_valid(scope) AND length(CAST(scope AS BLOB)) <= 8192),
        checkpoint_digest TEXT CHECK(checkpoint_digest IS NULL OR length(checkpoint_digest) = 64)
    );

CREATE TABLE classification_review_operations (
        operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
        session_id TEXT NOT NULL CHECK(length(session_id) BETWEEN 1 AND 128),
        position_sequence INTEGER NOT NULL CHECK(position_sequence > 0),
        committed_sequence INTEGER NOT NULL CHECK(committed_sequence = position_sequence + 1)
    );

CREATE TABLE classification_dispatch_state (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
        revision INTEGER NOT NULL CHECK(revision >= 0),
        epoch INTEGER NOT NULL CHECK(epoch >= 0),
        mode TEXT NOT NULL CHECK(mode IN ('disabled', 'automatic', 'paused', 'stopped')),
        run_id TEXT REFERENCES classification_runs(run_id),
        session_id TEXT CHECK(session_id IS NULL OR length(session_id) BETWEEN 1 AND 128),
        source_cursor INTEGER NOT NULL CHECK(source_cursor >= 0),
        lease_clock INTEGER NOT NULL CHECK(lease_clock >= 0),
        fairness_cursor INTEGER NOT NULL CHECK(fairness_cursor BETWEEN 0 AND 6),
        CHECK(mode = 'disabled' OR run_id IS NOT NULL)
    ) STRICT;

CREATE TABLE classification_stopped_runs (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_runs(run_id)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE classification_work_items (
        work_id INTEGER PRIMARY KEY,
        created_run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
        configuration_revision TEXT NOT NULL CHECK(length(configuration_revision) = 64),
        kind TEXT NOT NULL CHECK(kind IN ('source', 'content')),
        source_generation INTEGER,
        fingerprint_scheme TEXT,
        digest TEXT,
        byte_length INTEGER,
        priority INTEGER NOT NULL CHECK(priority BETWEEN 0 AND 2),
        phase TEXT NOT NULL CHECK(phase IN ('queued', 'running', 'waiting', 'done', 'failed', 'retired')),
        failure_count INTEGER NOT NULL CHECK(failure_count BETWEEN 0 AND 3),
        ready_unix_ms INTEGER NOT NULL CHECK(ready_unix_ms >= 0),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        lease_id INTEGER UNIQUE,
        lease_epoch INTEGER,
        lease_asset_id TEXT,
        lease_location_id TEXT,
        lease_source_generation INTEGER,
        CHECK((kind = 'source' AND source_generation IS NOT NULL AND source_generation > 0 AND fingerprint_scheme IS NULL
               AND digest IS NULL AND byte_length IS NULL)
           OR (kind = 'content' AND source_generation IS NULL AND fingerprint_scheme IS NOT NULL
               AND length(fingerprint_scheme) BETWEEN 1 AND 128 AND digest IS NOT NULL
               AND length(digest) = 64 AND byte_length IS NOT NULL AND byte_length > 0)),
        CHECK((phase = 'running' AND lease_id IS NOT NULL AND lease_id > 0
               AND lease_epoch IS NOT NULL AND lease_epoch >= 0
               AND lease_asset_id IS NOT NULL AND lease_location_id IS NOT NULL
               AND lease_source_generation IS NOT NULL
               AND length(lease_asset_id) BETWEEN 1 AND 128
               AND length(lease_location_id) BETWEEN 1 AND 128 AND lease_source_generation > 0
               AND (kind = 'content' OR lease_source_generation = source_generation))
           OR (phase <> 'running' AND lease_id IS NULL AND lease_epoch IS NULL
               AND lease_asset_id IS NULL AND lease_location_id IS NULL AND lease_source_generation IS NULL))
    ) STRICT;

CREATE TABLE classification_analysis_setups (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_runs(run_id),
        profile_payload TEXT CHECK(profile_payload IS NULL OR
            length(CAST(profile_payload AS BLOB)) BETWEEN 1 AND 65536)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE classification_dispatch_admission (
        singleton INTEGER PRIMARY KEY CHECK(singleton = 1)
            REFERENCES classification_dispatch_state(singleton),
        backlog_ceiling INTEGER NOT NULL CHECK(backlog_ceiling >= 0),
        arrival_cursor INTEGER NOT NULL CHECK(arrival_cursor >= backlog_ceiling)
    ) STRICT;

CREATE TABLE classification_unavailable_sources (
        failure_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(failure_id > 0),
        work_id INTEGER NOT NULL REFERENCES classification_work_items(work_id) ON DELETE CASCADE,
        root_id TEXT NOT NULL REFERENCES library_roots(id) ON DELETE CASCADE,
        root_generation INTEGER NOT NULL CHECK(root_generation > 0),
        asset_id TEXT NOT NULL CHECK(length(asset_id) BETWEEN 1 AND 128),
        location_id TEXT NOT NULL CHECK(length(location_id) BETWEEN 1 AND 128),
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        UNIQUE(work_id, source_generation, location_id)
    ) STRICT;

CREATE TABLE classification_source_publications (
            publication_id INTEGER PRIMARY KEY AUTOINCREMENT CHECK(publication_id > 0),
            root_id TEXT NOT NULL UNIQUE REFERENCES library_roots(id) ON DELETE CASCADE,
            root_generation INTEGER NOT NULL CHECK(root_generation > 0),
            scan_id TEXT NOT NULL,
            after_source_generation INTEGER NOT NULL DEFAULT 0 CHECK(after_source_generation >= 0)
        ) STRICT;

CREATE TABLE classification_managed_categories (
        category_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_categories(category_id),
        concept_id TEXT NOT NULL CHECK(length(concept_id) = 64 AND concept_id NOT GLOB '*[^0-9a-f]*'),
        purpose_category_id TEXT REFERENCES classification_categories(category_id),
        operation_id TEXT NOT NULL REFERENCES classification_operations(operation_id),
        admitted_label TEXT NOT NULL CHECK(length(admitted_label) BETWEEN 1 AND 80),
        admitted_semantic_revision TEXT NOT NULL CHECK(length(admitted_semantic_revision) = 64),
        protected INTEGER NOT NULL CHECK(protected IN (0, 1)),
        FOREIGN KEY(category_id, admitted_semantic_revision)
            REFERENCES classification_category_versions(category_id, semantic_revision)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE classification_maintenance_evidence (
    operation_id TEXT PRIMARY KEY NOT NULL REFERENCES classification_operations(operation_id),
    run_id TEXT NOT NULL REFERENCES classification_runs(run_id),
    evidence_revision TEXT NOT NULL CHECK(length(evidence_revision) = 64),
    payload TEXT NOT NULL CHECK(json_valid(payload)
        AND (json_extract(payload, '$.version') IS 1
            OR (json_extract(payload, '$.version') IS 2
                AND json_extract(payload, '$.kind') IS 'equivalent_category_retirement'
                AND json_type(payload, '$.retirements') IS 'array'
                AND json_array_length(payload, '$.retirements') BETWEEN 1 AND 64)
            OR (json_extract(payload, '$.version') IS 3
                AND json_extract(payload, '$.kind') IS 'purpose_category_merge'
                AND json_type(payload, '$.merge') IS 'object'
                AND json_type(payload, '$.merge.category') IS 'text'
                AND json_type(payload, '$.merge.purpose') IS 'text'))
        AND length(CAST(payload AS BLOB)) BETWEEN 1 AND 524288)
) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_runs (
        run_id TEXT PRIMARY KEY NOT NULL CHECK(length(run_id) BETWEEN 1 AND 128),
        source_run_id TEXT NOT NULL REFERENCES exact_analysis_runs(run_id),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        engine_id TEXT NOT NULL CHECK(length(engine_id) BETWEEN 1 AND 128),
        engine_version TEXT NOT NULL CHECK(length(engine_version) BETWEEN 1 AND 128),
        parameters TEXT NOT NULL CHECK(length(parameters) BETWEEN 1 AND 4096),
        started_unix_ms INTEGER NOT NULL CHECK(started_unix_ms >= 0)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_evidence (
        evidence_id INTEGER PRIMARY KEY NOT NULL CHECK(evidence_id > 0),
        run_id TEXT NOT NULL REFERENCES similarity_runs(run_id),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
        disposition TEXT NOT NULL CHECK(disposition IN ('ready', 'insufficient_structure', 'decode_failed')),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        UNIQUE(scheme, fingerprint_scheme, digest, byte_length),
        CHECK((disposition = 'decode_failed') = (issue_code IS NOT NULL))
    ) STRICT;

CREATE TABLE similarity_groups (
        group_id TEXT PRIMARY KEY NOT NULL CHECK(length(group_id) = 64 AND group_id NOT GLOB '*[^0-9a-f]*'),
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        reference_scheme TEXT NOT NULL CHECK(length(reference_scheme) BETWEEN 1 AND 128),
        reference_digest TEXT NOT NULL CHECK(length(reference_digest) = 64 AND reference_digest NOT GLOB '*[^0-9a-f]*'),
        reference_byte_length INTEGER NOT NULL CHECK(reference_byte_length BETWEEN 1 AND 268435456),
        UNIQUE(scheme, reference_scheme, reference_digest, reference_byte_length)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_references (
        group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
        evidence_id INTEGER NOT NULL UNIQUE REFERENCES similarity_evidence(evidence_id)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_members (
        group_id TEXT NOT NULL REFERENCES similarity_references(group_id),
        evidence_id INTEGER NOT NULL REFERENCES similarity_evidence(evidence_id),
        distance INTEGER NOT NULL CHECK(distance BETWEEN 0 AND 10),
        run_id TEXT NOT NULL REFERENCES similarity_runs(run_id),
        PRIMARY KEY(group_id, evidence_id)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_exclusions (
        group_id TEXT NOT NULL REFERENCES similarity_groups(group_id),
        fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
        digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
        byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
        exclusion_id TEXT NOT NULL UNIQUE CHECK(length(exclusion_id) BETWEEN 1 AND 128),
        PRIMARY KEY(group_id, fingerprint_scheme, digest, byte_length)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_source_failures (
        source_generation INTEGER NOT NULL CHECK(source_generation > 0),
        root_id TEXT NOT NULL,
        root_generation INTEGER NOT NULL CHECK(root_generation > 0),
        location_id TEXT NOT NULL,
        scheme TEXT NOT NULL CHECK(length(scheme) BETWEEN 1 AND 128),
        issue_code TEXT NOT NULL CHECK(length(issue_code) BETWEEN 1 AND 128),
        retry_unix_ms INTEGER CHECK(retry_unix_ms IS NULL OR retry_unix_ms >= 0),
        PRIMARY KEY(source_generation, root_id, root_generation, location_id, scheme)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_match_work (
        evidence_id INTEGER PRIMARY KEY NOT NULL REFERENCES similarity_evidence(evidence_id),
        phase TEXT NOT NULL CHECK(phase IN ('references', 'peer', 'members', 'complete')),
        reference_revision INTEGER NOT NULL CHECK(reference_revision >= 0),
        query_variant INTEGER NOT NULL CHECK(query_variant BETWEEN 0 AND 7),
        band INTEGER NOT NULL CHECK(band BETWEEN 0 AND 3),
        neighbor INTEGER NOT NULL CHECK(neighbor BETWEEN 0 AND 136),
        after_evidence_id INTEGER NOT NULL CHECK(after_evidence_id >= 0),
        after_group_id TEXT NOT NULL CHECK(length(after_group_id) <= 64),
        after_variant INTEGER NOT NULL CHECK(after_variant BETWEEN -1 AND 7),
        found_reference INTEGER NOT NULL CHECK(found_reference IN (0, 1))
    ) STRICT;

CREATE TABLE similarity_dispatch (
        singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
        active_run_id TEXT REFERENCES similarity_runs(run_id),
        lease_until_unix_ms INTEGER,
        reference_revision INTEGER NOT NULL CHECK(reference_revision >= 0),
        CHECK((active_run_id IS NULL) = (lease_until_unix_ms IS NULL)),
        CHECK(lease_until_unix_ms IS NULL OR lease_until_unix_ms >= 0)
    ) STRICT;

CREATE TABLE similarity_outcomes (
        run_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_runs(run_id),
        phase TEXT NOT NULL CHECK(phase IN ('completed', 'cancelled', 'failed')),
        checked_sources INTEGER NOT NULL CHECK(checked_sources >= 0),
        analyzed_contents INTEGER NOT NULL CHECK(analyzed_contents >= 0),
        skipped_sources INTEGER NOT NULL CHECK(skipped_sources >= 0),
        issue_code TEXT CHECK(issue_code IS NULL OR length(issue_code) BETWEEN 1 AND 128),
        finished_unix_ms INTEGER NOT NULL CHECK(finished_unix_ms >= 0)
    ) STRICT, WITHOUT ROWID;

CREATE TABLE "similarity_variants" (
    evidence_id INTEGER NOT NULL REFERENCES similarity_evidence(evidence_id),
    variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
    word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
    word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
    word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
    word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
    confirmation BLOB CHECK(confirmation IS NULL OR length(confirmation) = 32),
    PRIMARY KEY(evidence_id, variant),
    UNIQUE(evidence_id, word0, word1, word2, word3, confirmation)
) STRICT, WITHOUT ROWID;

CREATE TABLE "similarity_reference_variants" (
    group_id TEXT NOT NULL REFERENCES similarity_references(group_id),
    variant INTEGER NOT NULL CHECK(variant BETWEEN 0 AND 7),
    word0 INTEGER NOT NULL CHECK(word0 BETWEEN 0 AND 65535),
    word1 INTEGER NOT NULL CHECK(word1 BETWEEN 0 AND 65535),
    word2 INTEGER NOT NULL CHECK(word2 BETWEEN 0 AND 65535),
    word3 INTEGER NOT NULL CHECK(word3 BETWEEN 0 AND 65535),
    confirmation BLOB CHECK(confirmation IS NULL OR length(confirmation) = 32),
    PRIMARY KEY(group_id, variant)
) STRICT, WITHOUT ROWID;

CREATE TABLE similarity_preferences (
    group_id TEXT NOT NULL REFERENCES similarity_groups(group_id),
    fingerprint_scheme TEXT NOT NULL CHECK(length(fingerprint_scheme) BETWEEN 1 AND 128),
    digest TEXT NOT NULL CHECK(length(digest) = 64 AND digest NOT GLOB '*[^0-9a-f]*'),
    byte_length INTEGER NOT NULL CHECK(byte_length BETWEEN 1 AND 268435456),
    PRIMARY KEY(group_id, fingerprint_scheme, digest, byte_length)
) STRICT, WITHOUT ROWID;

CREATE TABLE "similarity_decisions" (
    group_id TEXT PRIMARY KEY NOT NULL REFERENCES similarity_groups(group_id),
    ignored INTEGER NOT NULL CHECK(ignored IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision >= 0)
) STRICT, WITHOUT ROWID;

CREATE TABLE organization_review_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version = 2),
    payload TEXT NOT NULL CHECK(length(CAST(payload AS BLOB)) <= 4096 AND json_valid(payload))
);

CREATE TABLE organization_album_icons (
    album_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_albums(album_id) ON DELETE CASCADE,
    icon TEXT NOT NULL CHECK(icon IN ('album', 'travel', 'nature', 'pets', 'people', 'food',
        'art', 'work', 'study', 'games', 'music', 'memories'))
);

CREATE TABLE file_operation_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    authorization TEXT NOT NULL CHECK(length(authorization) <= 524288 AND json_valid(authorization)),
    version INTEGER NOT NULL CHECK(version >= 0),
    phase TEXT NOT NULL CHECK(phase IN ('prepared', 'executing', 'name_occupied', 'copied',
        'removing_source', 'native_completed', 'published', 'failed', 'cancelled', 'uncertain')),
    UNIQUE(batch_id, ordinal)
);

CREATE TABLE file_operation_events (
    item_id INTEGER NOT NULL REFERENCES file_operation_items(id),
    version INTEGER NOT NULL CHECK(version >= 0),
    snapshot TEXT NOT NULL CHECK(length(snapshot) <= 524288 AND json_valid(snapshot)
        AND json_extract(snapshot, '$.version') = version),
    recorded_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(item_id, version)
);

CREATE TABLE file_operation_exports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_id TEXT NOT NULL UNIQUE CHECK(length(batch_id) BETWEEN 1 AND 256),
    authorized_unix_ms INTEGER NOT NULL,
    action TEXT NOT NULL CHECK(action IN ('copy', 'move')),
    rule TEXT NOT NULL CHECK(rule IN ('direct', 'category', 'album', 'date', 'source', 'album_year')),
    selected_files INTEGER NOT NULL CHECK(selected_files > 0),
    root_path TEXT NOT NULL CHECK(length(root_path) BETWEEN 1 AND 65536),
    identity_scheme TEXT NOT NULL CHECK(length(identity_scheme) BETWEEN 1 AND 256),
    identity_value TEXT NOT NULL CHECK(length(identity_value) BETWEEN 1 AND 4096),
    report_state TEXT NOT NULL DEFAULT 'pending' CHECK(report_state IN ('pending', 'not_needed', 'written', 'failed')),
    report_path TEXT,
    report_error_code TEXT,
    report_error_message TEXT,
    CHECK((report_state = 'written') = (report_path IS NOT NULL)),
    CHECK((report_state = 'failed') = (report_error_code IS NOT NULL AND report_error_message IS NOT NULL))
);

CREATE TABLE file_operation_export_skips (
    batch_id TEXT NOT NULL REFERENCES file_operation_exports(batch_id),
    ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
    source_path TEXT NOT NULL CHECK(length(source_path) BETWEEN 1 AND 65536),
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 4096),
    PRIMARY KEY(batch_id, ordinal)
) WITHOUT ROWID;

CREATE TABLE organization_review_group_filters (
    scope_slot TEXT PRIMARY KEY NOT NULL REFERENCES organization_review_scopes(slot) ON DELETE CASCADE,
    group_filter TEXT NOT NULL CHECK(group_filter IN ('unrestricted', 'exact', 'similar'))
) STRICT;

CREATE UNIQUE INDEX scan_runs_one_active_root
               ON scan_runs(root_id) WHERE status IN ('running', 'paused');

CREATE INDEX asset_locations_active_root
               ON asset_locations(root_id, scan_id, relative_path, location_id);

CREATE INDEX asset_locations_file_identity
               ON asset_locations(scan_id, file_identity_scheme, file_identity_value);

CREATE INDEX asset_locations_active_file_identity
               ON asset_locations(file_identity_scheme, file_identity_value, scan_id, location_id);

CREATE INDEX asset_locations_location_id
               ON asset_locations(location_id, scan_id);

CREATE INDEX asset_locations_asset_id
               ON asset_locations(asset_id);

CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (COALESCE(capture_local_time, file_local_time) IS NULL),
                 IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );

CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (file_local_time IS NULL), IFNULL(file_local_time, ''),
                 modified_unix_ms,
                 root_id, location_id, scan_id
               );

CREATE INDEX asset_locations_gallery_modified
               ON asset_locations(modified_unix_ms, root_id, location_id, scan_id);

CREATE INDEX asset_locations_gallery_name
               ON asset_locations(natural_name_key, root_id, location_id, scan_id);

CREATE INDEX asset_locations_parent_folder
               ON asset_locations(root_id, parent_relative_path, scan_id, location_id);

CREATE UNIQUE INDEX scan_issues_identity
               ON scan_issues(scan_id, IFNULL(path, ''), code, message);

CREATE INDEX scan_directory_frontier_order
               ON scan_directory_frontier(scan_id, id);

CREATE INDEX preview_artifact_locations_location
               ON preview_artifact_locations(location_id, artifact_key);

CREATE INDEX preview_artifacts_reclamation
               ON preview_artifacts(lifecycle_state, last_used_unix_ms, artifact_key);

CREATE INDEX preview_artifacts_compatibility
               ON preview_artifacts(
                 source_file_size, source_modified_unix_ms,
                 algorithm_id, algorithm_version, orientation_contract, size_bucket
             );

CREATE INDEX asset_locations_root_relative
               ON asset_locations(root_id, relative_path, scan_id, location_id);

CREATE INDEX library_change_catch_up_handoffs_asset
               ON library_change_catch_up_handoffs(
                 asset_id, catch_up_source, catch_up_watermark
               );

CREATE INDEX library_change_catch_up_handoffs_preview
               ON library_change_catch_up_handoffs(
                 preview_path, preview_status, catch_up_source, catch_up_watermark
               );

CREATE INDEX scan_run_catch_up_lineage_evidence
               ON scan_run_catch_up_lineage(
                 catch_up_source, catch_up_watermark, scan_id
               );

CREATE INDEX library_change_scan_handoff_lineage_evidence
               ON library_change_scan_handoff_lineage(
                 catch_up_source, catch_up_watermark, batch_id
               );

CREATE INDEX library_change_scan_handoff_items_identity
               ON library_change_scan_handoff_items(
                 file_identity_scheme, file_identity_value, batch_id
               );

CREATE INDEX library_change_scan_handoff_items_asset
               ON library_change_scan_handoff_items(asset_id, batch_id);

CREATE INDEX library_change_scan_handoff_items_preview
               ON library_change_scan_handoff_items(
                 preview_path, preview_status, batch_id
               );

CREATE INDEX library_change_queue_eligible
               ON library_change_queue(
                 root_id, root_generation, status, ready_unix_ms, next_retry_unix_ms, id
               );

CREATE INDEX library_change_queue_lease_expiry
               ON library_change_queue(status, lease_expires_unix_ms, id);

CREATE INDEX library_change_queue_active_path
               ON library_change_queue(
                 root_id, root_generation, status, relative_path, scope, id
               );

CREATE INDEX library_change_queue_cleanup
               ON library_change_queue(status, updated_unix_ms, id);

CREATE INDEX library_change_queue_catch_up_lineage_evidence
               ON library_change_queue_catch_up_lineage(
                 catch_up_source, catch_up_watermark, change_id
               );

CREATE UNIQUE INDEX library_metadata_inventory_runs_one_active_root
               ON library_metadata_inventory_runs(root_id)
               WHERE status IN ('running', 'comparing');

CREATE INDEX library_metadata_inventory_runs_cleanup
               ON library_metadata_inventory_runs(status, updated_unix_ms, id);

CREATE INDEX library_metadata_inventory_entries_compare
               ON library_metadata_inventory_entries(run_id, comparison_status, relative_path);

CREATE INDEX library_metadata_inventory_entries_identity
               ON library_metadata_inventory_entries(
                 run_id, file_identity_scheme, file_identity_value, relative_path
               );

CREATE INDEX library_metadata_inventory_entries_previous
               ON library_metadata_inventory_entries(
                 run_id, candidate_previous_relative_path, relative_path
               );

CREATE UNIQUE INDEX library_change_root_state_generation_identity
               ON library_change_root_state(root_id, generation);

CREATE INDEX library_persistent_journal_root_state_continuity
               ON library_persistent_journal_root_state(
                 continuity_state, capability_state, updated_unix_ms, root_id
               );

CREATE INDEX library_persistent_journal_checkpoints_volume
               ON library_persistent_journal_checkpoints(
                 volume_guid, journal_id, continuity_state, root_id
               );

CREATE INDEX library_persistent_journal_source_ranges_root
               ON library_persistent_journal_source_ranges(
                 root_id, root_generation, status, enrolled_unix_ms, id
               );

CREATE INDEX library_persistent_journal_source_ranges_volume
               ON library_persistent_journal_source_ranges(
                 volume_guid, journal_id, requested_start_usn, id
               );

CREATE INDEX library_persistent_journal_queue_lineage_change
               ON library_persistent_journal_queue_lineage(change_id, source_range_id);

CREATE INDEX library_persistent_journal_cross_root_lineage_previous
               ON library_persistent_journal_cross_root_lineage(
                 previous_root_id, previous_root_generation, status, id
               );

CREATE INDEX library_persistent_journal_cross_root_lineage_current
               ON library_persistent_journal_cross_root_lineage(
                 current_root_id, current_root_generation, status, id
               );

CREATE INDEX library_persistent_journal_cross_root_ranges_source
               ON library_persistent_journal_cross_root_ranges(source_range_id, lineage_id);

CREATE INDEX library_persistent_journal_range_lifecycle_state
               ON library_persistent_journal_range_lifecycle(
                 lifecycle_state, updated_unix_ms, source_range_id
               );

CREATE INDEX library_persistent_journal_pending_renames_volume
               ON library_persistent_journal_pending_renames(
                 volume_guid, journal_id, old_usn, carry_id
               );

CREATE INDEX library_persistent_journal_pending_renames_source
               ON library_persistent_journal_pending_renames(source_range_id, carry_id);

CREATE INDEX library_change_queue_lanes_eligible
               ON library_change_queue_lanes(lane, change_id);

CREATE INDEX library_recovery_authorities_root
               ON library_recovery_authorities(
                 root_id, root_generation, retired_unix_ms, change_id
               );

CREATE UNIQUE INDEX library_persistent_journal_baselines_root
       ON library_persistent_journal_baselines(root_id, root_generation)
       WHERE phase <> 'completed';

CREATE INDEX library_metadata_inventory_candidate_owners_change
       ON library_metadata_inventory_candidate_owners(change_id, run_id, candidate_key);

CREATE INDEX library_metadata_inventory_frontier_state
       ON library_metadata_inventory_frontier(run_id, state, ordinal);

CREATE UNIQUE INDEX library_scan_publication_namespace_root
       ON library_scan_publication_namespace_bindings(root_id, root_generation);

CREATE INDEX library_live_gap_recovery_claims_root
       ON library_live_gap_recovery_claims(
         root_id, root_generation, consumer_kind, gap_change_id
       );

CREATE INDEX library_metadata_inventory_spool_directories_state
       ON library_metadata_inventory_spool_directories(run_id, state, ordinal);

CREATE INDEX library_metadata_inventory_spool_entries_order
       ON library_metadata_inventory_spool_entries(run_id, relative_path);

CREATE UNIQUE INDEX library_metadata_inventory_spools_active_authority
      ON library_metadata_inventory_spools(authority_change_id) WHERE state <> 'retired';

CREATE INDEX library_metadata_inventory_spools_retirement
      ON library_metadata_inventory_spools(state, run_id);

CREATE INDEX library_metadata_inventory_spool_entries_directory
      ON library_metadata_inventory_spool_entries(run_id, directory_relative_path);

CREATE INDEX library_metadata_inventory_spool_entries_initial
      ON library_metadata_inventory_spool_entries(run_id, relative_path) WHERE directory_relative_path IS NULL;

CREATE INDEX library_change_queue_active_origin
    ON library_change_queue(root_id, root_generation, origin, status, id)
    WHERE status IN ('pending', 'leased', 'retry_wait');

CREATE INDEX scan_directory_identities_by_identity
    ON scan_directory_identities(scan_id, identity_value, relative_path);

CREATE INDEX library_media_admission_rechecks_pending
    ON library_media_admission_rechecks(state, root_id, relative_path)
    WHERE state <> 'rejected';

CREATE INDEX library_change_queue_superseded_by
         ON library_change_queue(superseded_by_change_id);

CREATE INDEX library_media_admission_rechecks_change
         ON library_media_admission_rechecks(change_id);

CREATE INDEX library_media_admission_rechecks_asset
         ON library_media_admission_rechecks(asset_id);

CREATE INDEX content_fingerprints_content
        ON content_fingerprints(scheme, digest, byte_length, source_generation);

CREATE INDEX asset_locations_exact_size
        ON asset_locations(file_size, location_id, root_id, scan_id);

CREATE INDEX asset_locations_exact_source
        ON asset_locations(source_generation, location_id, root_id, scan_id);

CREATE INDEX asset_locations_pending_metadata
    ON asset_locations(root_id, scan_id, relative_path)
    WHERE metadata_engine_id = 'ame-invalidated-media-metadata';

CREATE INDEX gallery_retained_fingerprint_evidence
    ON gallery_retained_fingerprints(evidence_source_generation, scheme);

CREATE INDEX organization_bindings_record
        ON organization_bindings(record_id);

CREATE INDEX organization_bindings_content
        ON organization_bindings(fingerprint_scheme, digest, byte_length) WHERE digest IS NOT NULL;

CREATE INDEX organization_sources_record
        ON organization_sources(record_id, root_id);

CREATE INDEX organization_record_group_members
        ON organization_record_groups(group_id, record_id);

CREATE INDEX asset_locations_organization_root_generation
       ON asset_locations(root_id, scan_id, source_generation);

CREATE INDEX organization_admission_sources_root
           ON organization_admission_sources(root_key, admission_id);

CREATE INDEX organization_bindings_native_version ON organization_bindings(
       file_identity_scheme, file_identity_value, source_revision_token, byte_length, record_id);

CREATE INDEX organization_recovery_versions_native ON organization_recovery_versions(
          file_identity_scheme, file_identity_value, source_revision_token, byte_length);

CREATE INDEX organization_recovery_versions_content ON organization_recovery_versions(
          fingerprint_scheme, digest, byte_length);

CREATE INDEX organization_recovery_versions_generation ON organization_recovery_versions(source_generation);

CREATE INDEX organization_recovery_admission_sources_root ON organization_recovery_admission_sources(root_id, source_generation);

CREATE INDEX organization_unassociated_records_group
         ON organization_unassociated_records(group_id, record_id);

CREATE UNIQUE INDEX organization_field_authority_revision
        ON organization_field_authority(record_id, field, applied_revision);

CREATE UNIQUE INDEX organization_album_authority_revision
        ON organization_album_authority(album_id, applied_revision);

CREATE INDEX library_sidebar_root_position ON library_sidebar_root_order(position, root_id);

CREATE INDEX organization_sidebar_album_position ON organization_sidebar_album_order(position, album_id);

CREATE INDEX exact_duplicate_preferred_location
    ON exact_duplicate_decisions(preferred_location_id, preferred_asset_id)
    WHERE preferred_location_id IS NOT NULL;

CREATE UNIQUE INDEX asset_locations_gallery_search ON asset_locations(gallery_search_id);

CREATE INDEX classification_predictions_content
        ON classification_predictions(fingerprint_scheme, digest, byte_length, configuration_revision, publication_id DESC);

CREATE INDEX classification_decisions_category
        ON classification_decisions(category_id, record_id) WHERE category_id IS NOT NULL;

CREATE UNIQUE INDEX classification_work_source
        ON classification_work_items(configuration_revision, source_generation) WHERE kind = 'source';

CREATE UNIQUE INDEX classification_work_content
        ON classification_work_items(configuration_revision, fingerprint_scheme, digest, byte_length) WHERE kind = 'content';

CREATE INDEX classification_work_ready
        ON classification_work_items(configuration_revision, phase, priority, kind, ready_unix_ms, work_id);

CREATE UNIQUE INDEX classification_work_single_claim
        ON classification_work_items((1)) WHERE phase = 'running';

CREATE INDEX classification_unavailable_root
        ON classification_unavailable_sources(root_id, root_generation, failure_id);

CREATE INDEX asset_locations_classification_publication
         ON asset_locations(root_id, scan_id, source_generation);

CREATE INDEX similarity_members_content ON similarity_members(evidence_id, group_id);

CREATE INDEX similarity_work_pending ON similarity_match_work(phase, evidence_id);

CREATE INDEX similarity_source_retries ON similarity_source_failures(retry_unix_ms) WHERE retry_unix_ms IS NOT NULL;

CREATE INDEX similarity_variants_band0 ON similarity_variants(word0, evidence_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_variants_band1 ON similarity_variants(word1, evidence_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_variants_band2 ON similarity_variants(word2, evidence_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_variants_band3 ON similarity_variants(word3, evidence_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_references_band0 ON similarity_reference_variants(word0, group_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_references_band1 ON similarity_reference_variants(word1, group_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_references_band2 ON similarity_reference_variants(word2, group_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX similarity_references_band3 ON similarity_reference_variants(word3, group_id, variant) WHERE confirmation IS NOT NULL;

CREATE INDEX file_operation_pending ON file_operation_items(id)
    WHERE phase IN ('prepared', 'executing', 'name_occupied', 'copied', 'removing_source', 'native_completed', 'uncertain');

CREATE INDEX file_operation_removal_source ON file_operation_items(
    json_extract(authorization, '$.catalog.location_id'),
    json_extract(authorization, '$.catalog.source_generation'))
    WHERE phase IN ('executing', 'native_completed', 'uncertain')
      AND json_extract(authorization, '$.action') IN ('recycle', 'delete_permanently');

CREATE TRIGGER library_persistent_journal_source_range_id_insert
          BEFORE INSERT ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END;

CREATE TRIGGER library_persistent_journal_source_range_id_update
          BEFORE UPDATE OF id ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END;

CREATE TRIGGER library_change_queue_lane_insert_guard
               BEFORE INSERT ON library_change_queue_lanes
               WHEN NEW.lane <> (
                 SELECT CASE origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 FROM library_change_queue WHERE id = NEW.change_id
               )
               BEGIN
                 SELECT RAISE(ABORT, 'change queue lane does not match origin');
               END;

CREATE TRIGGER library_change_queue_lane_update_guard
               BEFORE UPDATE OF lane, change_id ON library_change_queue_lanes
               WHEN NEW.lane <> (
                 SELECT CASE origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 FROM library_change_queue WHERE id = NEW.change_id
               )
               BEGIN
                 SELECT RAISE(ABORT, 'change queue lane does not match origin');
               END;

CREATE TRIGGER library_change_queue_lane_insert
               AFTER INSERT ON library_change_queue
               BEGIN
                 INSERT INTO library_change_queue_lanes(change_id, lane)
                 VALUES (
                   NEW.id,
                   CASE NEW.origin
                     WHEN 'live_notification' THEN 'p0_live'
                     WHEN 'startup_catch_up' THEN 'p1_journal'
                     ELSE 'p2_recovery'
                   END
                 );
               END;

CREATE TRIGGER library_change_queue_lane_origin_update
               AFTER UPDATE OF origin ON library_change_queue
               BEGIN
                 UPDATE library_change_queue_lanes
                 SET lane = CASE NEW.origin
                   WHEN 'live_notification' THEN 'p0_live'
                   WHEN 'startup_catch_up' THEN 'p1_journal'
                   ELSE 'p2_recovery'
                 END
                 WHERE change_id = NEW.id;
               END;

CREATE TRIGGER library_recovery_authority_insert_guard
               BEFORE INSERT ON library_recovery_authorities
               WHEN NOT EXISTS (
                 SELECT 1
                 FROM library_change_queue AS queue
                 JOIN library_change_queue_lanes AS lanes ON lanes.change_id = queue.id
                 WHERE queue.id = NEW.change_id
                   AND queue.root_id = NEW.root_id
                   AND queue.root_generation = NEW.root_generation
                   AND lanes.lane = 'p2_recovery'
               )
               BEGIN
                 SELECT RAISE(ABORT, 'recovery authority does not match P2 queue work');
               END;

CREATE TRIGGER library_recovery_authority_update_guard
               BEFORE UPDATE OF change_id, root_id, root_generation, reason,
                                opening_journal_id, opening_next_usn, authorized_unix_ms
               ON library_recovery_authorities
               BEGIN
                 SELECT RAISE(ABORT, 'recovery authority identity is immutable');
               END;

CREATE TRIGGER library_persistent_journal_baseline_insert_guard
       BEFORE INSERT ON library_persistent_journal_baselines
       WHEN NOT EXISTS (
         SELECT 1 FROM library_recovery_authorities AS authority
         WHERE authority.change_id = NEW.change_id
           AND authority.root_id = NEW.root_id
           AND authority.root_generation = NEW.root_generation
           AND (
             (authority.reason IN ('existing_root_baseline', 'first_import_boundary')
               AND authority.opening_journal_id = NEW.journal_id
               AND authority.opening_next_usn = NEW.opening_next_usn)
             OR
             (authority.reason IN (
                'watcher_uncovered_gap', 'journal_gap', 'journal_reset', 'journal_trim',
                'journal_reconstruction_failure', 'containment_failure',
                'broker_after_current_failure'
              )
               AND authority.opening_journal_id IS NULL
               AND authority.opening_next_usn IS NULL)
           )
           AND authority.retired_unix_ms IS NULL
       )
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline does not match recovery authority');
       END;

CREATE TRIGGER library_persistent_journal_baseline_update_guard
       BEFORE UPDATE OF change_id, root_id, root_generation, volume_guid, volume_serial,
                        root_reference_version, root_file_reference, journal_id,
                        opening_next_usn, protocol_version, contract_version,
                        authorized_unix_ms
       ON library_persistent_journal_baselines
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline identity is immutable');
       END;

CREATE TRIGGER library_metadata_inventory_candidate_owner_insert_guard
       BEFORE INSERT ON library_metadata_inventory_candidate_owners
       WHEN NOT EXISTS (
         SELECT 1
         FROM library_metadata_inventory_runs AS run
         JOIN library_change_queue AS queue ON queue.id = NEW.change_id
         WHERE run.id = NEW.run_id
           AND queue.root_id = run.root_id
           AND queue.root_generation = run.root_generation
           AND queue.scope = 'path'
           AND queue.relative_path = NEW.relative_path
           AND queue.previous_relative_path IS NEW.previous_relative_path
       )
       BEGIN
         SELECT RAISE(ABORT, 'inventory candidate owner does not match path work');
       END;

CREATE TRIGGER library_metadata_inventory_candidate_owner_update_guard
       BEFORE UPDATE OF run_id, candidate_key, candidate_role, relative_path,
                        previous_relative_path, owned_unix_ms
       ON library_metadata_inventory_candidate_owners
       BEGIN
         SELECT RAISE(ABORT, 'inventory candidate owner identity is immutable');
       END;

CREATE TRIGGER library_live_gap_recovery_claim_insert_guard
       BEFORE INSERT ON library_live_gap_recovery_claims
       WHEN NOT EXISTS (
         SELECT 1
         FROM library_change_queue AS gap
         JOIN library_change_queue_lanes AS lane ON lane.change_id = gap.id
         WHERE gap.id = NEW.gap_change_id
           AND gap.root_id = NEW.root_id
           AND gap.root_generation = NEW.root_generation
           AND gap.intent_kind = 'freshness_unknown'
           AND gap.scope = 'root' AND gap.relative_path = ''
           AND gap.previous_relative_path IS NULL
           AND (
              (NEW.consumer_kind IN (
                 'pending_journal', 'journal_source_range', 'metadata_inventory_control'
               )
                AND gap.origin = 'live_notification' AND lane.lane = 'p0_live')
             OR
             (NEW.consumer_kind = 'explicit_recovery_required'
               AND gap.origin = 'startup_catch_up' AND lane.lane = 'p1_journal'
                AND gap.last_failure_code = 'live_gap_v30_explicit_recovery_required')
            )
            AND NEW.consumer_kind <> 'foreground_scan'
        )
       BEGIN
         SELECT RAISE(ABORT, 'live gap claim does not match durable gap work');
       END;

CREATE TRIGGER library_live_gap_recovery_claim_identity_update_guard
       BEFORE UPDATE OF gap_change_id, root_id, root_generation,
                        opening_volume_guid, opening_volume_serial,
                        opening_root_reference_version, opening_root_file_reference,
                        opening_journal_id, opening_next_usn,
                        protocol_version, contract_version, created_unix_ms
       ON library_live_gap_recovery_claims
       BEGIN
         SELECT RAISE(ABORT, 'live gap claim identity is immutable');
       END;

CREATE TRIGGER asset_locations_source_generation_insert_guard
             BEFORE INSERT ON asset_locations
             WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
             BEGIN
               SELECT RAISE(ABORT, 'asset location source generation is required');
             END;

CREATE TRIGGER asset_locations_source_generation_update_guard
             BEFORE UPDATE OF source_generation ON asset_locations
             WHEN NEW.source_generation IS NULL OR NEW.source_generation <= 0
             BEGIN
               SELECT RAISE(ABORT, 'asset location source generation is required');
             END;

CREATE TRIGGER library_metadata_inventory_spool_binding_update_guard
       BEFORE UPDATE OF run_id, authority_change_id, root_id, root_generation,
                        root_identity_scheme, root_identity_value,
                        scope_kind, scope_relative_path, created_unix_ms
       ON library_metadata_inventory_spools
       BEGIN
         SELECT RAISE(ABORT, 'inventory spool authority binding is immutable');
     END;

CREATE TRIGGER library_metadata_inventory_spool_directory_complete_guard
       BEFORE UPDATE OF directory_identity_scheme, directory_identity_value,
                        source_entry_count
       ON library_metadata_inventory_spool_directories
       WHEN OLD.state = 'completed'
       BEGIN
         SELECT RAISE(ABORT, 'completed inventory spool directory is immutable');
       END;

CREATE TRIGGER library_metadata_inventory_spool_retired_guard
     BEFORE UPDATE OF state ON library_metadata_inventory_spools
     WHEN OLD.state = 'retired' AND NEW.state <> 'retired'
     BEGIN SELECT RAISE(ABORT, 'retired inventory storage cannot regain execution'); END;

CREATE TRIGGER library_metadata_inventory_spool_run_delete
     BEFORE DELETE ON library_metadata_inventory_runs
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE run_id = OLD.id AND state <> 'retired'; END;

CREATE TRIGGER library_metadata_inventory_spool_run_terminal
     AFTER UPDATE OF status ON library_metadata_inventory_runs
     WHEN NEW.status IN ('failed', 'cancelled', 'superseded')
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE run_id = NEW.id AND state <> 'retired'; END;

CREATE TRIGGER library_metadata_inventory_spool_authority_delete
     BEFORE DELETE ON library_recovery_authorities
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE authority_change_id = OLD.change_id AND state <> 'retired'; END;

CREATE TRIGGER library_metadata_inventory_spool_authority_retired
     AFTER UPDATE OF retired_unix_ms ON library_recovery_authorities
     WHEN NEW.retired_unix_ms IS NOT NULL
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE authority_change_id = NEW.change_id AND state <> 'retired'; END;

CREATE TRIGGER scan_resume_pending_insert_guard
         BEFORE INSERT ON scan_resume_pending_locations
         WHEN NOT EXISTS (
             SELECT 1 FROM scan_runs AS scan
             JOIN library_roots AS root ON root.id = scan.root_id
             JOIN library_change_root_state AS state ON state.root_id = root.id
             WHERE scan.id = NEW.scan_id AND scan.scan_owner = 'foreground'
               AND scan.status = 'running' AND root.active_scan_id IS NULL
               AND state.is_active = 1 AND state.generation = scan.root_generation_at_start
         )
         BEGIN SELECT RAISE(ABORT, 'retained import has no current resume authority'); END;

CREATE TRIGGER scan_resume_pending_update_guard
         BEFORE UPDATE ON scan_resume_pending_locations
         BEGIN SELECT RAISE(ABORT, 'retained import membership is immutable'); END;

CREATE TRIGGER scan_resume_publication_guard
         BEFORE UPDATE OF status ON scan_runs
         WHEN NEW.status = 'completed' AND EXISTS (
             SELECT 1 FROM scan_resume_pending_locations WHERE scan_id = NEW.id
         )
         BEGIN SELECT RAISE(ABORT, 'retained import namespace has not been reconciled'); END;

CREATE TRIGGER library_metadata_fallback_insert_guard
    BEFORE INSERT ON library_metadata_fallbacks
    WHEN NOT EXISTS (
      SELECT 1 FROM library_recovery_authorities AS authority
      JOIN library_change_queue AS control ON control.id = authority.change_id
      JOIN library_persistent_journal_checkpoints AS checkpoint
        ON checkpoint.root_id = authority.root_id
       AND checkpoint.root_generation = authority.root_generation
      WHERE authority.change_id = NEW.change_id AND authority.retired_unix_ms IS NULL
        AND control.scope = 'root' AND control.origin = 'metadata_inventory'
        AND control.status IN ('pending', 'leased', 'retry_wait')
        AND authority.reason IN (
          'journal_gap', 'journal_reset', 'journal_trim', 'journal_reconstruction_failure',
          'containment_failure', 'broker_after_current_failure', 'existing_root_baseline', 'watcher_uncovered_gap'
        )
        AND checkpoint.continuity_state = 'recovery_required'
        AND checkpoint.last_failure_code IS NOT NULL
        AND NEW.admitted_unix_ms >= authority.authorized_unix_ms
    )
    BEGIN SELECT RAISE(ABORT, 'metadata fallback requires a failed journal and root recovery'); END;

CREATE TRIGGER library_metadata_fallback_update_guard
    BEFORE UPDATE ON library_metadata_fallbacks
    BEGIN SELECT RAISE(ABORT, 'metadata fallback admission is immutable'); END;

CREATE TRIGGER organization_unassociated_member_insert
         AFTER INSERT ON organization_record_groups
         BEGIN
           INSERT INTO organization_unassociated_records(record_id, group_id)
           SELECT NEW.record_id, NEW.group_id FROM organization_records
           WHERE record_id = NEW.record_id AND unassociated_since_unix_ms IS NOT NULL;
         END;

CREATE TRIGGER organization_unassociated_member_move
         AFTER UPDATE OF group_id ON organization_record_groups
         WHEN NEW.group_id <> OLD.group_id
         BEGIN
           UPDATE organization_unassociated_records SET group_id = NEW.group_id
           WHERE record_id = NEW.record_id;
         END;

CREATE TRIGGER organization_unassociated_record_change
         AFTER UPDATE OF unassociated_since_unix_ms ON organization_records
         WHEN NEW.unassociated_since_unix_ms IS NOT OLD.unassociated_since_unix_ms
         BEGIN
           DELETE FROM organization_unassociated_records
           WHERE record_id = NEW.record_id AND NEW.unassociated_since_unix_ms IS NULL;
           INSERT OR IGNORE INTO organization_unassociated_records(record_id, group_id)
           SELECT record_id, group_id FROM organization_record_groups
           WHERE record_id = NEW.record_id AND NEW.unassociated_since_unix_ms IS NOT NULL;
         END;

CREATE TRIGGER organization_capture_field_authority
        AFTER INSERT ON organization_operation_changes
        WHEN NEW.applied_revision = (SELECT revision + 1 FROM organization_state) BEGIN
        INSERT INTO organization_field_authority
            (operation_id, record_id, field, applied_revision, previous_authority, applied_authority)
        SELECT NEW.operation_id, NEW.record_id, NEW.field, NEW.applied_revision,
            COALESCE(prior.applied_authority,
                CASE NEW.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END),
            NEW.applied_revision
        FROM organization_records AS record
        LEFT JOIN organization_field_authority AS prior ON prior.record_id = record.record_id
            AND prior.field = NEW.field AND prior.applied_revision =
                CASE NEW.field WHEN 'favorite' THEN record.favorite_revision ELSE record.organization_revision END
        WHERE record.record_id = NEW.record_id;
    END;

CREATE TRIGGER organization_capture_album_authority
        AFTER INSERT ON organization_album_operation_changes
        WHEN NEW.applied_revision = (SELECT revision + 1 FROM organization_state) BEGIN
        INSERT INTO organization_album_authority
            (operation_id, album_id, applied_revision, previous_authority, applied_authority)
        SELECT NEW.operation_id, NEW.album_id, NEW.applied_revision,
            CASE WHEN NEW.previous_name IS NULL THEN 0 ELSE COALESCE(prior.applied_authority, album.revision) END,
            NEW.applied_revision
        FROM organization_albums AS album
        LEFT JOIN organization_album_authority AS prior ON prior.album_id = album.album_id
            AND prior.applied_revision = album.revision
        WHERE album.album_id = NEW.album_id;
    END;

CREATE TRIGGER library_sidebar_root_append AFTER INSERT ON library_roots BEGIN
           INSERT INTO library_sidebar_root_order(root_id, position)
             SELECT NEW.id, COALESCE(MAX(position) + 1, 0) FROM library_sidebar_root_order;
         END;

CREATE TRIGGER organization_sidebar_album_append AFTER INSERT ON organization_albums BEGIN
           INSERT INTO organization_sidebar_album_order(album_id, position)
             SELECT NEW.album_id, COALESCE(MAX(position) + 1, 0) FROM organization_sidebar_album_order;
         END;

CREATE TRIGGER gallery_search_insert
AFTER INSERT ON asset_locations BEGIN
    UPDATE asset_locations SET gallery_search_id = (
      SELECT COALESCE(MAX(gallery_search_id), 0) + 1 FROM asset_locations
    ) WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
    INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
      SELECT gallery_search_id, NEW.absolute_path, NEW.relative_path FROM asset_locations
      WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
END;

CREATE TRIGGER gallery_search_update
AFTER UPDATE OF absolute_path, relative_path ON asset_locations
WHEN OLD.absolute_path IS NOT NEW.absolute_path OR OLD.relative_path IS NOT NEW.relative_path
BEGIN
    UPDATE gallery_search_terms SET absolute_path = NEW.absolute_path, relative_path = NEW.relative_path
      WHERE rowid = NEW.gallery_search_id;
END;

CREATE TRIGGER gallery_search_delete
BEFORE DELETE ON asset_locations BEGIN
    DELETE FROM gallery_search_terms WHERE rowid = OLD.gallery_search_id;
END;

CREATE TRIGGER gallery_search_identity
BEFORE UPDATE OF gallery_search_id ON asset_locations
WHEN OLD.gallery_search_id IS NOT NULL AND OLD.gallery_search_id IS NOT NEW.gallery_search_id
BEGIN SELECT RAISE(ABORT, 'catalog search identity is immutable'); END;

CREATE TRIGGER file_operation_authorization_immutable
    BEFORE UPDATE OF id, batch_id, ordinal, authorization ON file_operation_items
    BEGIN SELECT RAISE(ABORT, 'file operation authorization is immutable'); END;

CREATE TRIGGER file_operation_events_immutable
    BEFORE UPDATE ON file_operation_events
    BEGIN SELECT RAISE(ABORT, 'file operation events are immutable'); END;

CREATE TRIGGER file_operation_events_retained
    BEFORE DELETE ON file_operation_events
    BEGIN SELECT RAISE(ABORT, 'file operation history is durable'); END;

CREATE TRIGGER file_operation_export_authority_immutable
    BEFORE UPDATE OF batch_id, authorized_unix_ms, action, rule, selected_files, root_path, identity_scheme, identity_value
    ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export authority is immutable'); END;

CREATE TRIGGER file_operation_exports_retained
    BEFORE DELETE ON file_operation_exports BEGIN SELECT RAISE(ABORT, 'export history is durable'); END;

CREATE TRIGGER file_operation_export_skips_immutable
    BEFORE UPDATE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are immutable'); END;

CREATE TRIGGER file_operation_export_skips_retained
    BEFORE DELETE ON file_operation_export_skips BEGIN SELECT RAISE(ABORT, 'export skips are durable'); END;

CREATE VIEW gallery_display_fingerprints AS
    SELECT f.source_generation, f.source_generation AS evidence_source_generation,
           f.scheme, f.digest, f.byte_length, f.file_identity_scheme, f.file_identity_value,
           f.source_revision_token, 0 AS is_pending
    FROM content_fingerprints AS f
    JOIN gallery_content_fingerprints AS published
      ON published.source_generation = f.source_generation AND published.scheme = f.scheme
    UNION ALL
    SELECT retained.source_generation, f.source_generation,
           f.scheme, f.digest, f.byte_length, f.file_identity_scheme, f.file_identity_value,
           f.source_revision_token, 1
    FROM gallery_retained_fingerprints AS retained
    JOIN content_fingerprints AS f
      ON f.source_generation = retained.evidence_source_generation AND f.scheme = retained.scheme
    WHERE NOT EXISTS (
      SELECT 1 FROM gallery_content_fingerprints AS current
      WHERE current.source_generation = retained.source_generation AND current.scheme = retained.scheme
    );

CREATE VIEW organization_queue_versions AS
          SELECT CASE WHEN history.admission_id < current.admission_id
            THEN history.admission_id ELSE current.admission_id END AS admission_id,
            current.source_generation
          FROM organization_admissions AS current
          LEFT JOIN organization_recovery_admission_links AS linked
            ON linked.source_generation = current.source_generation
          LEFT JOIN organization_recovery_admissions AS history
            ON history.source_generation = linked.recovered_generation;

CREATE VIEW classification_human_projection AS
         SELECT group_id,
           CASE WHEN local_choices > 0 THEN local_choices ELSE restored_choices END AS choice_count,
           CASE WHEN local_choices > 0 THEN local_category ELSE restored_category END AS category_id,
           COALESCE(local_deferred, restored_deferred, 0) AS deferred
         FROM (
           SELECT member.group_id,
             COUNT(DISTINCT CASE WHEN decision.choice_restored = 0 THEN decision.category_id END) AS local_choices,
             COUNT(DISTINCT CASE WHEN decision.choice_restored = 1 THEN decision.category_id END) AS restored_choices,
             MIN(CASE WHEN decision.choice_restored = 0 THEN decision.category_id END) AS local_category,
             MIN(CASE WHEN decision.choice_restored = 1 THEN decision.category_id END) AS restored_category,
             MAX(CASE WHEN decision.review_restored = 0 AND decision.review_revision > 0
                 THEN decision.deferred END) AS local_deferred,
             MAX(CASE WHEN decision.review_restored = 1 AND decision.review_revision > 0
                 THEN decision.deferred END) AS restored_deferred
           FROM organization_record_groups AS member
           JOIN classification_decisions AS decision USING(record_id)
           GROUP BY member.group_id
         );

CREATE VIEW classification_prediction_projection AS
         SELECT prediction.publication_id, prediction.fingerprint_scheme, prediction.digest,
           prediction.byte_length,
           (SELECT json_extract(score.value, '$.category')
            FROM json_each(COALESCE(
              json_extract(prediction.payload, '$.refined.categories'),
              json_extract(prediction.payload, '$.initial.categories'))) AS score
            ORDER BY json_extract(score.value, '$.similarity') DESC,
              json_extract(score.value, '$.category') COLLATE BINARY LIMIT 1) AS category_id,
           CASE json_extract(prediction.payload, '$.review')
             WHEN 'routine_sampling' THEN 'suggested'
             ELSE 'needs_review'
           END AS review
         FROM classification_predictions AS prediction
         JOIN classification_state AS state
           ON state.singleton = 1 AND state.configuration_revision = prediction.configuration_revision
         JOIN classification_runs AS run ON run.run_id = prediction.run_id
           AND run.primary_rules = state.active_rules;

INSERT INTO "catalog_state" VALUES (0, 1);
INSERT INTO "classification_dispatch_state" VALUES (1, 0, 0, 'disabled', NULL, NULL, 0, 0, 0);
INSERT INTO "classification_dispatch_admission" VALUES (1, 0, 0);
INSERT INTO "classification_state" VALUES (1, 0, NULL, NULL, NULL);
INSERT INTO "library_change_lane_contract" VALUES (1, 1);
INSERT INTO "library_change_preview_repair_contract" VALUES (1, 1);
INSERT INTO "library_change_queue_contract" VALUES (1, 1, 1, 1, 1, 1, 1);
INSERT INTO "library_live_gap_recovery_contract" VALUES (1, 1, 1);
INSERT INTO "library_metadata_inventory_contract" VALUES (1, 1);
INSERT INTO "library_metadata_inventory_spool_contract" VALUES (1, 4, 1);
INSERT INTO "library_persistent_journal_contract" VALUES (1, 1, 1);
INSERT INTO "library_recovery_authority_contract" VALUES (1, 1);
INSERT INTO "library_recovery_execution_contract" VALUES (1, 1, 1);
INSERT INTO "library_root_publication_namespace_contract" VALUES (1, 1, 1);
INSERT INTO "library_source_revision_metadata_contract" VALUES (1, 1, 1);
INSERT INTO "library_terminal_media_evidence_contract" VALUES (1, 1);
INSERT INTO "organization_admission_clock" VALUES (1, 0);
INSERT INTO "organization_history_state" VALUES (1, 0, 0);
INSERT INTO "organization_state" VALUES (1, 0);
INSERT INTO "schema_info" VALUES (83);
INSERT INTO "similarity_dispatch" VALUES (1, NULL, NULL, 0);
