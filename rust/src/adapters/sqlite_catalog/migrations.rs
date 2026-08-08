use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;

use super::{SCHEMA_VERSION, database_error, natural_name_key, parent_relative_path};

pub(super) fn migrate_schema(connection: &mut Connection) -> Result<(), ScanError> {
    let has_schema_info: bool = connection
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM sqlite_master
               WHERE type = 'table' AND name = 'schema_info'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;

    if !has_schema_info {
        let transaction = connection.transaction().map_err(database_error)?;
        create_schema_v13(&transaction)?;
        return transaction.commit().map_err(database_error);
    }

    loop {
        let version: i64 = connection
            .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
                row.get(0)
            })
            .map_err(database_error)?;
        match version {
            SCHEMA_VERSION => return Ok(()),
            1 => migrate_v1_to_v2(connection)?,
            2 => migrate_v2_to_v3(connection)?,
            3 => migrate_v3_to_v4(connection)?,
            4 => migrate_v4_to_v5(connection)?,
            5 => migrate_v5_to_v6(connection)?,
            6 => migrate_v6_to_v7(connection)?,
            7 => migrate_v7_to_v8(connection)?,
            8 => migrate_v8_to_v9(connection)?,
            9 => migrate_v9_to_v10(connection)?,
            10 => migrate_v10_to_v11(connection)?,
            11 => migrate_v11_to_v12(connection)?,
            12 => migrate_v12_to_v13(connection)?,
            _ => {
                return Err(ScanError::new(
                    "catalog_schema_unsupported",
                    format!("Expected catalog schema {SCHEMA_VERSION}, found {version}"),
                ));
            }
        }
    }
}

fn create_schema_v13(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    transaction
        .execute_batch(
            "CREATE TABLE schema_info (
               version INTEGER NOT NULL
             );
             INSERT INTO schema_info(version) VALUES (13);
             CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision) VALUES (0);
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
               file_identity_value TEXT,
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
             CREATE TABLE scan_issues (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               path TEXT,
               code TEXT NOT NULL,
               message TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );
             CREATE UNIQUE INDEX scan_issues_identity
               ON scan_issues(scan_id, IFNULL(path, ''), code, message);
             CREATE TABLE scan_directory_frontier (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               UNIQUE(scan_id, relative_path)
             );
             CREATE INDEX scan_directory_frontier_order
               ON scan_directory_frontier(scan_id, id);
             CREATE TABLE scan_directory_entries (
               scan_id TEXT NOT NULL,
               directory_relative_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(scan_id, directory_relative_path, relative_path),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );",
        )
        .map_err(database_error)
}

fn migrate_v1_to_v2(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations RENAME TO asset_locations_v1;
             CREATE TABLE assets (
               id TEXT PRIMARY KEY,
               created_unix_ms INTEGER NOT NULL
             );
             INSERT INTO assets(id, created_unix_ms)
               SELECT 'legacy:' || locations.scan_id || ':' || locations.location_id,
                      scans.started_unix_ms
               FROM asset_locations_v1 AS locations
               JOIN scan_runs AS scans ON scans.id = locations.scan_id;
             CREATE TABLE asset_locations (
               scan_id TEXT NOT NULL,
               asset_id TEXT NOT NULL,
               location_id TEXT NOT NULL,
               root_id TEXT NOT NULL,
               absolute_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               preview_path TEXT NOT NULL,
               file_size INTEGER NOT NULL,
               modified_unix_ms INTEGER NOT NULL,
               width INTEGER NOT NULL,
               height INTEGER NOT NULL,
               PRIMARY KEY(scan_id, location_id),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               FOREIGN KEY(asset_id) REFERENCES assets(id),
               FOREIGN KEY(root_id) REFERENCES library_roots(id)
             );
             INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, modified_unix_ms, width, height
             )
               SELECT scan_id, 'legacy:' || scan_id || ':' || location_id,
                      location_id, root_id, absolute_path, relative_path,
                      preview_path, file_size, modified_unix_ms, width, height
               FROM asset_locations_v1;
             DROP TABLE asset_locations_v1;
             CREATE INDEX asset_locations_active_root
               ON asset_locations(root_id, scan_id, relative_path, location_id);
             UPDATE schema_info SET version = 2;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v2_to_v3(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE TABLE catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT COUNT(*) FROM scan_runs WHERE status = 'completed';
             UPDATE schema_info SET version = 3;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v3_to_v4(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN max_items INTEGER;
             ALTER TABLE scan_runs ADD COLUMN max_entries INTEGER;
             ALTER TABLE scan_runs ADD COLUMN preview_edge INTEGER NOT NULL DEFAULT 512;
             ALTER TABLE scan_runs ADD COLUMN last_visited_relative_path TEXT;
             ALTER TABLE scan_runs ADD COLUMN visited_entries INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE scan_runs ADD COLUMN accepted_items INTEGER NOT NULL DEFAULT 0;
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms)
               WHERE status = 'running';
             CREATE UNIQUE INDEX scan_issues_identity
               ON scan_issues(scan_id, IFNULL(path, ''), code, message);
             UPDATE schema_info SET version = 4;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v4_to_v5(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN current_directory_relative_path TEXT;
             CREATE TABLE scan_directory_frontier (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               scan_id TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id),
               UNIQUE(scan_id, relative_path)
             );
             CREATE INDEX scan_directory_frontier_order
               ON scan_directory_frontier(scan_id, id);
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms)
               WHERE status IN ('running', 'paused');
             UPDATE schema_info SET version = 5;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v5_to_v6(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE scan_runs ADD COLUMN current_directory_enumerated INTEGER NOT NULL
               DEFAULT 0 CHECK(current_directory_enumerated IN (0, 1));
             CREATE TABLE scan_directory_entries (
               scan_id TEXT NOT NULL,
               directory_relative_path TEXT NOT NULL,
               relative_path TEXT NOT NULL,
               PRIMARY KEY(scan_id, directory_relative_path, relative_path),
               FOREIGN KEY(scan_id) REFERENCES scan_runs(id)
             );
             UPDATE scan_runs
               SET status = 'interrupted_unrecoverable',
                   completed_unix_ms = COALESCE(completed_unix_ms, started_unix_ms),
                   current_directory_relative_path = NULL,
                   current_directory_enumerated = 0,
                   last_visited_relative_path = NULL
               WHERE status IN ('running', 'paused');
             DELETE FROM scan_directory_frontier
               WHERE scan_id IN (
                 SELECT id FROM scan_runs WHERE status = 'interrupted_unrecoverable'
               );
             UPDATE schema_info SET version = 6;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v6_to_v7(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN preview_status TEXT NOT NULL
               DEFAULT 'ready' CHECK(preview_status IN ('pending', 'ready', 'failed'));
             ALTER TABLE asset_locations ADD COLUMN preview_issue_code TEXT;
             ALTER TABLE asset_locations ADD COLUMN preview_issue_message TEXT;
             UPDATE schema_info SET version = 7;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v7_to_v8(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN metadata_engine_id TEXT NOT NULL
               DEFAULT 'unknown';
             ALTER TABLE asset_locations ADD COLUMN metadata_engine_version TEXT NOT NULL
               DEFAULT '0';
             ALTER TABLE asset_locations ADD COLUMN capture_local_time TEXT;
             ALTER TABLE asset_locations ADD COLUMN capture_offset_minutes INTEGER;
             ALTER TABLE asset_locations ADD COLUMN capture_time_source TEXT
               CHECK(capture_time_source IS NULL OR capture_time_source IN (
                 'exif_original', 'exif_digitized', 'exif_datetime'
               ));
             ALTER TABLE asset_locations ADD COLUMN capture_raw_value TEXT;
             UPDATE schema_info SET version = 8;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v8_to_v9(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN file_identity_scheme TEXT;
             ALTER TABLE asset_locations ADD COLUMN file_identity_value TEXT;
             CREATE INDEX asset_locations_file_identity
               ON asset_locations(scan_id, file_identity_scheme, file_identity_value);
             UPDATE schema_info SET version = 9;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v9_to_v10(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_active_file_identity
               ON asset_locations(file_identity_scheme, file_identity_value, scan_id, location_id);
             CREATE INDEX asset_locations_location_id
               ON asset_locations(location_id, scan_id);
             CREATE INDEX asset_locations_asset_id
               ON asset_locations(asset_id);
             UPDATE schema_info SET version = 10;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v10_to_v11(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (capture_local_time IS NULL), IFNULL(capture_local_time, '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             UPDATE schema_info SET version = 11;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v11_to_v12(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN created_unix_ms INTEGER;
             ALTER TABLE asset_locations ADD COLUMN parent_relative_path TEXT NOT NULL DEFAULT '';
             ALTER TABLE asset_locations ADD COLUMN natural_name_key TEXT NOT NULL DEFAULT '';",
        )
        .map_err(database_error)?;
    let locations = {
        let mut statement = transaction
            .prepare("SELECT rowid, relative_path FROM asset_locations")
            .map_err(database_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(database_error)?;
        let mut locations = Vec::new();
        for row in rows {
            locations.push(row.map_err(database_error)?);
        }
        locations
    };
    for (row_id, relative_path) in locations {
        transaction
            .execute(
                "UPDATE asset_locations
                 SET parent_relative_path = ?1, natural_name_key = ?2
                 WHERE rowid = ?3",
                params![
                    parent_relative_path(&relative_path),
                    natural_name_key(&relative_path),
                    row_id,
                ],
            )
            .map_err(database_error)?;
    }
    transaction
        .execute_batch(
            "CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (created_unix_ms IS NULL), IFNULL(created_unix_ms, 0),
                 root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_modified
               ON asset_locations(modified_unix_ms, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_gallery_name
               ON asset_locations(natural_name_key, root_id, location_id, scan_id);
             CREATE INDEX asset_locations_parent_folder
               ON asset_locations(root_id, parent_relative_path, scan_id, location_id);
             UPDATE schema_info SET version = 12;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}

fn migrate_v12_to_v13(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection.transaction().map_err(database_error)?;
    transaction
        .execute_batch(
            "ALTER TABLE asset_locations ADD COLUMN file_local_time TEXT;
             UPDATE asset_locations
               SET file_local_time = strftime(
                 '%Y-%m-%dT%H:%M:%f',
                 COALESCE(created_unix_ms, modified_unix_ms) / 1000.0,
                 'unixepoch', 'localtime'
               );
             DROP INDEX asset_locations_gallery_time;
             DROP INDEX asset_locations_gallery_created;
             CREATE INDEX asset_locations_gallery_time
               ON asset_locations(
                 (COALESCE(capture_local_time, file_local_time) IS NULL),
                 IFNULL(COALESCE(capture_local_time, file_local_time), '') DESC,
                 modified_unix_ms DESC, root_id, location_id, scan_id
               );
             CREATE INDEX asset_locations_gallery_created
               ON asset_locations(
                 (file_local_time IS NULL), IFNULL(file_local_time, ''),
                 modified_unix_ms, root_id, location_id, scan_id
               );
             UPDATE schema_info SET version = 13;",
        )
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)
}
