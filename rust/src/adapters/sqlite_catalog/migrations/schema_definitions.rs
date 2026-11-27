pub(super) const CHANGE_LANE_INDEX_DDL: &str = "CREATE INDEX library_change_queue_lanes_eligible
       ON library_change_queue_lanes(lane, change_id)";

pub(super) const CHANGE_LANE_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_change_lane_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

pub(super) const CHANGE_LANE_QUEUE_TABLE_DDL: &str = "CREATE TABLE library_change_queue_lanes (
       change_id INTEGER PRIMARY KEY,
       lane TEXT NOT NULL CHECK(lane IN ('p0_live', 'p1_journal', 'p2_recovery')),
       FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
     )";

pub(super) const CHANGE_LANE_INSERT_TRIGGER_DDL: &str =
    "CREATE TRIGGER library_change_queue_lane_insert
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
       END";

pub(super) const CHANGE_LANE_ORIGIN_TRIGGER_DDL: &str =
    "CREATE TRIGGER library_change_queue_lane_origin_update
       AFTER UPDATE OF origin ON library_change_queue
       BEGIN
         UPDATE library_change_queue_lanes
         SET lane = CASE NEW.origin
           WHEN 'live_notification' THEN 'p0_live'
           WHEN 'startup_catch_up' THEN 'p1_journal'
           ELSE 'p2_recovery'
         END
         WHERE change_id = NEW.id;
       END";

pub(super) const CHANGE_LANE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_change_queue_lane_insert_guard
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
       END";

pub(super) const CHANGE_LANE_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_change_queue_lane_update_guard
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
       END";

pub(super) const RECOVERY_AUTHORITY_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_recovery_authority_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

pub(super) const RECOVERY_AUTHORITY_TABLE_DDL: &str = "CREATE TABLE library_recovery_authorities (
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
     )";

pub(super) const RECOVERY_AUTHORITY_ROOT_INDEX_DDL: &str =
    "CREATE INDEX library_recovery_authorities_root
       ON library_recovery_authorities(root_id, root_generation, retired_unix_ms, change_id)";

pub(super) const RECOVERY_AUTHORITY_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_recovery_authority_insert_guard
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
       END";

pub(super) const RECOVERY_AUTHORITY_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_recovery_authority_update_guard
       BEFORE UPDATE OF change_id, root_id, root_generation, reason,
                        opening_journal_id, opening_next_usn, authorized_unix_ms
       ON library_recovery_authorities
       BEGIN
         SELECT RAISE(ABORT, 'recovery authority identity is immutable');
       END";

pub(super) const PERSISTENT_JOURNAL_BASELINE_TABLE_DDL: &str =
    "CREATE TABLE library_persistent_journal_baselines (
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
     )";

pub(super) const PERSISTENT_JOURNAL_BASELINE_ROOT_INDEX_DDL: &str =
    "CREATE UNIQUE INDEX library_persistent_journal_baselines_root
       ON library_persistent_journal_baselines(root_id, root_generation)
       WHERE phase <> 'completed'";

pub(super) const PERSISTENT_JOURNAL_BASELINE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_persistent_journal_baseline_insert_guard
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
       END";

pub(super) const PERSISTENT_JOURNAL_BASELINE_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_persistent_journal_baseline_update_guard
       BEFORE UPDATE OF change_id, root_id, root_generation, volume_guid, volume_serial,
                        root_reference_version, root_file_reference, journal_id,
                        opening_next_usn, protocol_version, contract_version,
                        authorized_unix_ms
       ON library_persistent_journal_baselines
       BEGIN
         SELECT RAISE(ABORT, 'journal baseline identity is immutable');
       END";

pub(super) const METADATA_INVENTORY_RUN_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_runs (
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
     )";

pub(super) const RECOVERY_EXECUTION_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_recovery_execution_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

pub(super) const METADATA_INVENTORY_CANDIDATE_OWNER_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_candidate_owners (
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
     )";

pub(super) const METADATA_INVENTORY_CANDIDATE_CHANGE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_candidate_owners_change
       ON library_metadata_inventory_candidate_owners(change_id, run_id, candidate_key)";

pub(super) const METADATA_INVENTORY_CANDIDATE_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_candidate_owner_insert_guard
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
       END";

pub(super) const METADATA_INVENTORY_CANDIDATE_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_candidate_owner_update_guard
       BEFORE UPDATE OF run_id, candidate_key, candidate_role, relative_path,
                        previous_relative_path, owned_unix_ms
       ON library_metadata_inventory_candidate_owners
       BEGIN
         SELECT RAISE(ABORT, 'inventory candidate owner identity is immutable');
       END";

pub(super) const METADATA_INVENTORY_FRONTIER_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_frontier (
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
     )";

pub(super) const METADATA_INVENTORY_FRONTIER_STATE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_frontier_state
       ON library_metadata_inventory_frontier(run_id, state, ordinal)";

pub(super) const METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_directories (
       run_id TEXT NOT NULL,
       ordinal INTEGER NOT NULL CHECK(ordinal >= 0),
       relative_directory TEXT NOT NULL CHECK(length(relative_directory) <= 32767),
       state TEXT NOT NULL CHECK(state IN ('pending', 'enumerating', 'completed')),
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
       FOREIGN KEY(run_id) REFERENCES library_metadata_inventory_spools(run_id)
         ON DELETE CASCADE
     )";

pub(super) const METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_spool_directories_state
       ON library_metadata_inventory_spool_directories(run_id, state, ordinal)";

pub(super) const METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL: &str =
    "CREATE TABLE library_metadata_inventory_spool_entries (
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
         REFERENCES library_metadata_inventory_spool_directories(run_id, relative_directory)
         ON DELETE CASCADE
     )";

pub(super) const METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL: &str =
    "CREATE INDEX library_metadata_inventory_spool_entries_order
       ON library_metadata_inventory_spool_entries(run_id, relative_path)";

pub(super) const ROOT_PUBLICATION_NAMESPACE_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_root_publication_namespace_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

pub(super) const ROOT_PUBLICATION_NAMESPACE_TABLE_DDL: &str =
    "CREATE TABLE library_root_publication_namespaces (
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
     )";

pub(super) const SCAN_PUBLICATION_NAMESPACE_BINDING_TABLE_DDL: &str =
    "CREATE TABLE library_scan_publication_namespace_bindings (
       scan_id TEXT PRIMARY KEY,
       root_id TEXT NOT NULL,
       root_generation INTEGER NOT NULL CHECK(root_generation > 0),
       identity_scheme TEXT NOT NULL CHECK(identity_scheme = 'windows-file-id-128-v1'),
       identity_value TEXT NOT NULL CHECK(length(identity_value) = 49),
       bound_unix_ms INTEGER NOT NULL CHECK(bound_unix_ms >= 0),
       FOREIGN KEY(scan_id) REFERENCES scan_runs(id) ON DELETE CASCADE,
       FOREIGN KEY(root_id) REFERENCES library_roots(id) ON DELETE CASCADE
     )";

pub(super) const SCAN_PUBLICATION_NAMESPACE_ROOT_INDEX_DDL: &str =
    "CREATE UNIQUE INDEX library_scan_publication_namespace_root
       ON library_scan_publication_namespace_bindings(root_id, root_generation)";

pub(super) const LIVE_GAP_RECOVERY_CONTRACT_TABLE_DDL: &str =
    "CREATE TABLE library_live_gap_recovery_contract (
       singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
       contract_version INTEGER NOT NULL CHECK(contract_version = 1),
       complete INTEGER NOT NULL CHECK(complete = 1)
     )";

pub(super) const LIVE_GAP_RECOVERY_CLAIM_TABLE_DDL: &str =
    "CREATE TABLE library_live_gap_recovery_claims (
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
      )";

pub(super) const LIVE_GAP_RECOVERY_ROOT_INDEX_DDL: &str =
    "CREATE INDEX library_live_gap_recovery_claims_root
       ON library_live_gap_recovery_claims(
         root_id, root_generation, consumer_kind, gap_change_id
       )";

pub(super) const LIVE_GAP_RECOVERY_INSERT_GUARD_DDL: &str =
    "CREATE TRIGGER library_live_gap_recovery_claim_insert_guard
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
       END";

pub(super) const LIVE_GAP_RECOVERY_IDENTITY_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_live_gap_recovery_claim_identity_update_guard
       BEFORE UPDATE OF gap_change_id, root_id, root_generation,
                        opening_volume_guid, opening_volume_serial,
                        opening_root_reference_version, opening_root_file_reference,
                        opening_journal_id, opening_next_usn,
                        protocol_version, contract_version, created_unix_ms
       ON library_live_gap_recovery_claims
       BEGIN
         SELECT RAISE(ABORT, 'live gap claim identity is immutable');
       END";

pub(super) const METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_spool_binding_update_guard
       BEFORE UPDATE OF run_id, authority_change_id, root_id, root_generation,
                        root_identity_scheme, root_identity_value,
                        scope_kind, scope_relative_path, created_unix_ms
       ON library_metadata_inventory_spools
       BEGIN
         SELECT RAISE(ABORT, 'inventory spool authority binding is immutable');
     END";

pub(super) const METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL: &str =
    "CREATE TRIGGER library_metadata_inventory_spool_directory_complete_guard
       BEFORE UPDATE OF directory_identity_scheme, directory_identity_value,
                        source_entry_count
       ON library_metadata_inventory_spool_directories
       WHEN OLD.state = 'completed'
       BEGIN
         SELECT RAISE(ABORT, 'completed inventory spool directory is immutable');
       END";
