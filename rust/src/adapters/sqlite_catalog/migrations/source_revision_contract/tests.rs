use rusqlite::Connection;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use super::super::{self as migrations, migrate_schema};

#[test]
fn current_v31_source_revision_structure_damage_still_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("DROP TRIGGER asset_locations_source_generation_update_guard;")
        .expect("damage source revision structure");

    super::reset_row_audit_count();
    let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

    assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
    assert_eq!(super::row_audit_count(), 0);
}

#[test]
fn current_v31_source_revision_marker_damage_still_fails_closed() {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch("DELETE FROM library_source_revision_metadata_contract;")
        .expect("damage source revision marker");

    migrations::reset_current_schema_row_audit_count();
    let error = migrate_schema(&mut connection).expect_err("reject damaged current catalog");

    assert_eq!(error.code, "catalog_source_revision_contract_unverifiable");
    assert_eq!(migrations::current_schema_row_audit_count(), 0);
}

#[test]
fn current_v31_repairs_precontract_metadata_for_every_hardlink_alias() {
    let mut connection = precontract_hardlink_catalog();

    super::reset_row_audit_count();
    migrations::reset_current_schema_row_audit_count();
    migrate_schema(&mut connection).expect("repair precontract v31 metadata");
    assert_eq!(
        super::row_audit_count(),
        3,
        "initial validation plus the repair transaction's before-and-after proof",
    );
    assert!(migrations::current_schema_row_audit_count() > 0);

    let repaired = connection
        .query_row(
            "SELECT COUNT(*),
                    SUM(metadata_engine_id = 'ame-invalidated-media-metadata'
                        AND metadata_engine_version = '0'),
                    SUM(capture_local_time IS NULL
                        AND capture_time_source IS NULL
                        AND capture_raw_value IS NULL),
                    SUM(preview_status = 'pending' AND preview_path = ''),
                    COUNT(DISTINCT source_generation),
                    COUNT(DISTINCT source_revision_token)
             FROM asset_locations",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .expect("repaired alias evidence");
    assert_eq!(repaired, (2, 2, 2, 2, 1, 1));
    assert!(
        super::metadata_contract_is_complete(&connection)
            .expect("repaired source metadata contract")
    );

    connection
        .execute_batch("PRAGMA query_only = ON")
        .expect("make repaired catalog read only");
    migrate_schema(&mut connection).expect("idempotently validate repaired v31 metadata");
}

#[test]
fn failed_contract_publication_rolls_back_all_alias_metadata_and_can_retry() {
    let mut connection = precontract_hardlink_catalog();
    connection
        .authorizer(Some(|context: AuthContext<'_>| {
            if matches!(
                context.action,
                AuthAction::Insert {
                    table_name: "library_source_revision_metadata_contract"
                }
            ) {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        }))
        .expect("reject marker publication after metadata invalidation");

    migrate_schema(&mut connection).expect_err("repair cannot publish its marker");
    connection
        .authorizer(None::<fn(AuthContext<'_>) -> Authorization>)
        .expect("allow a subsequent repair");

    let preserved_aliases = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations
             WHERE metadata_engine_id = 'legacy-metadata'
               AND metadata_engine_version = '7'
               AND capture_local_time = '2020-01-02T03:04:05'
               AND capture_time_source = 'exif_original'
               AND capture_raw_value = '2020:01:02 03:04:05'
               AND preview_status = 'ready' AND preview_path <> ''
               AND source_generation = 1
               AND source_revision_token =
                   'windows-file-change-time-100ns-v1:0000000000000042'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("inspect rolled-back aliases");
    assert_eq!(preserved_aliases, 2);
    let marker_exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master
             WHERE type = 'table' AND name = 'library_source_revision_metadata_contract')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .expect("inspect rolled-back marker creation");
    assert!(!marker_exists);

    migrate_schema(&mut connection).expect("retry compatibility repair");
    assert!(super::metadata_contract_is_complete(&connection).expect("completed marker"));
    let invalidated_aliases = connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations
             WHERE metadata_engine_id = 'ame-invalidated-media-metadata'
               AND capture_local_time IS NULL
               AND preview_status = 'pending' AND preview_path = ''",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("inspect repaired aliases");
    assert_eq!(invalidated_aliases, 2);
}

fn precontract_hardlink_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().expect("current catalog");
    migrate_schema(&mut connection).expect("create current catalog");
    connection
        .execute_batch(
            "UPDATE catalog_state SET next_source_generation = 2;
             INSERT INTO library_roots(id, path, active_scan_id, created_unix_ms)
               VALUES ('root-a', 'C:/source', 'published-scan', 1);
             INSERT INTO scan_runs(
               id, root_id, status, started_unix_ms, completed_unix_ms,
               asset_count, preview_edge
             ) VALUES ('published-scan', 'root-a', 'completed', 1, 2, 2, 256);
             INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES ('root-a', 1, 1, 2);
             INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES ('root-a', 1, 0, 1, 'unknown', 'baseline_required', 2);
             INSERT INTO assets(id, created_unix_ms)
               VALUES ('asset-a', 1), ('asset-b', 1);
             INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, modified_unix_ms, width, height, preview_status,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value,
               source_revision_token, source_generation
             ) VALUES
               ('published-scan', 'asset-a', 'location-a', 'root-a',
                'C:/source/a.bmp', 'a.bmp', 'C:/cache/a.jpg', 1024, 55, 8, 6, 'ready',
                'legacy-metadata', '7', '2020-01-02T03:04:05',
                'exif_original', '2020:01:02 03:04:05',
                'windows-file-id-128-v1',
                '0000000000000001:00000000000000000000000000000001',
                'windows-file-change-time-100ns-v1:0000000000000042', 1),
               ('published-scan', 'asset-b', 'location-b', 'root-a',
                'C:/source/b.bmp', 'b.bmp', 'C:/cache/b.jpg', 1024, 55, 8, 6, 'ready',
                'legacy-metadata', '7', '2020-01-02T03:04:05',
                'exif_original', '2020:01:02 03:04:05',
                'windows-file-id-128-v1',
                '0000000000000001:00000000000000000000000000000001',
                'windows-file-change-time-100ns-v1:0000000000000042', 1);
             DROP TABLE library_source_revision_metadata_contract;",
        )
        .expect("precontract v31 hardlink fixture");

    connection
}
