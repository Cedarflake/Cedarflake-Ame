use rusqlite::{Connection, OptionalExtension};

use crate::domain::ScanError;

use super::super::database_error;
use super::super::schema_shape::normalize_schema_sql;

pub(in super::super) const PERSISTENT_JOURNAL_CANONICAL_TABLE_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_contract",
        r#"CREATE TABLE library_persistent_journal_contract (
          singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          complete INTEGER NOT NULL CHECK(complete = 1)
        )"#,
    ),
    (
        "library_persistent_journal_root_state",
        r#"CREATE TABLE library_persistent_journal_root_state (
          root_id TEXT NOT NULL,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 0 AND 65535),
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
        )"#,
    ),
    (
        "library_persistent_journal_checkpoints",
        r#"CREATE TABLE library_persistent_journal_checkpoints (
          root_id TEXT PRIMARY KEY,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          root_reference_version INTEGER NOT NULL CHECK(root_reference_version IN (2, 3)),
          root_file_reference BLOB NOT NULL,
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          next_unread_usn TEXT NOT NULL CHECK(length(next_unread_usn) BETWEEN 1 AND 19),
          captured_exclusive_end TEXT NOT NULL
            CHECK(length(captured_exclusive_end) BETWEEN 1 AND 19),
          covered_catalog_revision INTEGER NOT NULL CHECK(covered_catalog_revision >= 0),
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
            OR (next_unread_usn = captured_exclusive_end AND last_failure_code IS NULL)
          ),
          FOREIGN KEY(root_id, root_generation)
            REFERENCES library_persistent_journal_root_state(root_id, root_generation)
            ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_source_ranges",
        r#"CREATE TABLE library_persistent_journal_source_ranges (
          id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
          root_id TEXT NOT NULL,
          root_generation INTEGER NOT NULL CHECK(root_generation > 0),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          requested_start_usn TEXT NOT NULL CHECK(length(requested_start_usn) BETWEEN 1 AND 19),
          requested_end_usn TEXT NOT NULL CHECK(length(requested_end_usn) BETWEEN 1 AND 19),
          covered_until_usn TEXT NOT NULL CHECK(length(covered_until_usn) BETWEEN 1 AND 19),
          is_complete INTEGER NOT NULL CHECK(is_complete IN (0, 1)),
          protocol_version INTEGER NOT NULL CHECK(protocol_version BETWEEN 1 AND 65535),
          contract_version INTEGER NOT NULL CHECK(contract_version = 1),
          status TEXT NOT NULL CHECK(status IN ('enrolled', 'checkpointed', 'superseded')),
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          checkpointed_unix_ms INTEGER CHECK(checkpointed_unix_ms >= 0),
          canonical_payload BLOB NOT NULL DEFAULT X'',
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
        )"#,
    ),
    (
        "library_persistent_journal_queue_lineage",
        r#"CREATE TABLE library_persistent_journal_queue_lineage (
          source_range_id TEXT NOT NULL,
          change_id INTEGER NOT NULL,
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          PRIMARY KEY(source_range_id, change_id),
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE,
          FOREIGN KEY(change_id) REFERENCES library_change_queue(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_cross_root_lineage",
        r#"CREATE TABLE library_persistent_journal_cross_root_lineage (
          id TEXT PRIMARY KEY CHECK(length(id) BETWEEN 1 AND 512),
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          file_reference_version INTEGER NOT NULL CHECK(file_reference_version IN (2, 3)),
          file_reference BLOB NOT NULL,
          previous_root_id TEXT NOT NULL,
          previous_root_generation INTEGER NOT NULL CHECK(previous_root_generation > 0),
          previous_relative_path TEXT NOT NULL,
          current_root_id TEXT NOT NULL,
          current_root_generation INTEGER NOT NULL CHECK(current_root_generation > 0),
          current_relative_path TEXT NOT NULL,
          status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'superseded')),
          created_unix_ms INTEGER NOT NULL CHECK(created_unix_ms >= 0),
          updated_unix_ms INTEGER NOT NULL CHECK(updated_unix_ms >= 0),
          journal_id TEXT,
          old_usn TEXT,
          new_usn TEXT,
          previous_carry_id TEXT,
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
        )"#,
    ),
    (
        "library_persistent_journal_cross_root_ranges",
        r#"CREATE TABLE library_persistent_journal_cross_root_ranges (
          lineage_id TEXT NOT NULL,
          source_range_id TEXT NOT NULL,
          participant_role TEXT NOT NULL CHECK(participant_role IN ('previous', 'current')),
          enrolled_unix_ms INTEGER NOT NULL CHECK(enrolled_unix_ms >= 0),
          PRIMARY KEY(lineage_id, source_range_id),
          FOREIGN KEY(lineage_id)
            REFERENCES library_persistent_journal_cross_root_lineage(id) ON DELETE CASCADE,
          FOREIGN KEY(source_range_id)
            REFERENCES library_persistent_journal_source_ranges(id) ON DELETE CASCADE
        )"#,
    ),
    (
        "library_persistent_journal_range_lifecycle",
        r#"CREATE TABLE library_persistent_journal_range_lifecycle (
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
        )"#,
    ),
    (
        "library_persistent_journal_pending_renames",
        r#"CREATE TABLE library_persistent_journal_pending_renames (
          carry_id TEXT PRIMARY KEY CHECK(length(carry_id) BETWEEN 1 AND 512),
          source_range_id TEXT NOT NULL,
          volume_guid TEXT NOT NULL CHECK(length(volume_guid) BETWEEN 1 AND 512),
          volume_serial TEXT NOT NULL CHECK(length(volume_serial) BETWEEN 1 AND 20),
          journal_id TEXT NOT NULL CHECK(length(journal_id) BETWEEN 1 AND 20),
          file_reference_version INTEGER NOT NULL CHECK(file_reference_version IN (2, 3)),
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
        )"#,
    ),
];

pub(in super::super) const PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_source_range_id_insert",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_insert
          BEFORE INSERT ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
    (
        "library_persistent_journal_source_range_id_update",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_update
          BEFORE UPDATE OF id ON library_persistent_journal_source_ranges
          WHEN NEW.id IS NULL OR typeof(NEW.id) <> 'text'
            OR length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
];

pub(in super::super) const PERSISTENT_JOURNAL_LEGACY_V24_TRIGGER_DDL: &[(&str, &str)] = &[
    (
        "library_persistent_journal_source_range_id_insert",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_insert
          BEFORE INSERT ON library_persistent_journal_source_ranges
          WHEN length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
    (
        "library_persistent_journal_source_range_id_update",
        r#"CREATE TRIGGER library_persistent_journal_source_range_id_update
          BEFORE UPDATE OF id ON library_persistent_journal_source_ranges
          WHEN length(NEW.id) <> 64 OR NEW.id GLOB '*[^0-9a-f]*'
          BEGIN
            SELECT RAISE(ABORT, 'invalid persistent journal source range id');
          END"#,
    ),
];

pub(super) fn matches(connection: &Connection) -> Result<bool, ScanError> {
    for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TABLE_DDL {
        let Some(sql) = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?
        else {
            return Ok(false);
        };
        if normalize_schema_sql(&sql) != normalize_schema_sql(expected) {
            return Ok(false);
        }
    }
    for (name, expected) in PERSISTENT_JOURNAL_CANONICAL_TRIGGER_DDL {
        let Some(sql) = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                [name],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?
        else {
            return Ok(false);
        };
        if normalize_schema_sql(&sql) != normalize_schema_sql(expected) {
            return Ok(false);
        }
    }
    Ok(true)
}
