use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape::schema_object_sql_matches;
use super::{
    METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL, METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL,
};

const CONTRACT: &str = "CREATE TABLE library_metadata_inventory_spool_contract (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    contract_version INTEGER NOT NULL CHECK(contract_version = 4),
    complete INTEGER NOT NULL CHECK(complete = 1)
)";
const HEADER: &str = "CREATE TABLE \"library_metadata_inventory_spools\" (
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
)";
const GUARDS: &[(&str, &str)] = &[
    (
        "library_metadata_inventory_spool_retired_guard",
        "CREATE TRIGGER library_metadata_inventory_spool_retired_guard
     BEFORE UPDATE OF state ON library_metadata_inventory_spools
     WHEN OLD.state = 'retired' AND NEW.state <> 'retired'
     BEGIN SELECT RAISE(ABORT, 'retired inventory storage cannot regain execution'); END",
    ),
    (
        "library_metadata_inventory_spool_run_delete",
        "CREATE TRIGGER library_metadata_inventory_spool_run_delete
     BEFORE DELETE ON library_metadata_inventory_runs
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE run_id = OLD.id AND state <> 'retired'; END",
    ),
    (
        "library_metadata_inventory_spool_run_terminal",
        "CREATE TRIGGER library_metadata_inventory_spool_run_terminal
     AFTER UPDATE OF status ON library_metadata_inventory_runs
     WHEN NEW.status IN ('failed', 'cancelled', 'superseded')
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE run_id = NEW.id AND state <> 'retired'; END",
    ),
    (
        "library_metadata_inventory_spool_authority_delete",
        "CREATE TRIGGER library_metadata_inventory_spool_authority_delete
     BEFORE DELETE ON library_recovery_authorities
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE authority_change_id = OLD.change_id AND state <> 'retired'; END",
    ),
    (
        "library_metadata_inventory_spool_authority_retired",
        "CREATE TRIGGER library_metadata_inventory_spool_authority_retired
     AFTER UPDATE OF retired_unix_ms ON library_recovery_authorities
     WHEN NEW.retired_unix_ms IS NOT NULL
     BEGIN UPDATE library_metadata_inventory_spools SET state = 'retired'
       WHERE authority_change_id = NEW.change_id AND state <> 'retired'; END",
    ),
];
const INDEXES: &[(&str, &str)] = &[
    ("library_metadata_inventory_spools_active_authority", "CREATE UNIQUE INDEX library_metadata_inventory_spools_active_authority
      ON library_metadata_inventory_spools(authority_change_id) WHERE state <> 'retired'"),
    ("library_metadata_inventory_spools_retirement", "CREATE INDEX library_metadata_inventory_spools_retirement
      ON library_metadata_inventory_spools(state, run_id)"),
    ("library_metadata_inventory_spool_entries_directory", "CREATE INDEX library_metadata_inventory_spool_entries_directory
      ON library_metadata_inventory_spool_entries(run_id, directory_relative_path)"),
    ("library_metadata_inventory_spool_entries_initial", "CREATE INDEX library_metadata_inventory_spool_entries_initial
      ON library_metadata_inventory_spool_entries(run_id, relative_path) WHERE directory_relative_path IS NULL"),
];

fn directory_ddl() -> String {
    METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL
        .replace(
            "library_metadata_inventory_spool_directories",
            "\"library_metadata_inventory_spool_directories\"",
        )
        .replace(
            "library_metadata_inventory_spools",
            "\"library_metadata_inventory_spools\"",
        )
        .replace("ON DELETE CASCADE", "ON DELETE RESTRICT")
        .replace(
            "'pending', 'enumerating', 'completed'",
            "'pending', 'enumerating', 'resetting', 'completed'",
        )
}

fn entry_ddl() -> String {
    METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL
        .replace(
            "library_metadata_inventory_spool_entries",
            "\"library_metadata_inventory_spool_entries\"",
        )
        .replace(
            "library_metadata_inventory_spool_directories",
            "\"library_metadata_inventory_spool_directories\"",
        )
        .replace("ON DELETE CASCADE", "ON DELETE RESTRICT")
}

pub(super) fn schema_matches(connection: &Connection) -> Result<bool, ScanError> {
    for (kind, name, ddl) in [
        (
            "table",
            "library_metadata_inventory_spool_contract",
            CONTRACT,
        ),
        ("table", "library_metadata_inventory_spools", HEADER),
        (
            "table",
            "library_metadata_inventory_spool_directories",
            &directory_ddl(),
        ),
        (
            "table",
            "library_metadata_inventory_spool_entries",
            &entry_ddl(),
        ),
        (
            "index",
            "library_metadata_inventory_spool_directories_state",
            METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
        ),
        (
            "index",
            "library_metadata_inventory_spool_entries_order",
            METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
        ),
        (
            "trigger",
            "library_metadata_inventory_spool_binding_update_guard",
            METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
        ),
        (
            "trigger",
            "library_metadata_inventory_spool_directory_complete_guard",
            METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
        ),
    ] {
        if !schema_object_sql_matches(connection, kind, name, ddl)? {
            return Ok(false);
        }
    }
    for (name, ddl) in GUARDS {
        if !schema_object_sql_matches(connection, "trigger", name, ddl)? {
            return Ok(false);
        }
    }
    for (name, ddl) in INDEXES {
        if !schema_object_sql_matches(connection, "index", name, ddl)? {
            return Ok(false);
        }
    }
    Ok(true)
}
