use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::adapters::SqliteCatalog;
use crate::ports::LibraryChangeQueue;

use super::*;

fn restore_v38(connection: &Connection) {
    for (name, _) in INDEXES {
        connection
            .execute_batch(&format!("DROP INDEX {name}"))
            .unwrap();
    }
    connection
        .execute_batch("UPDATE schema_info SET version = 38; PRAGMA user_version = 38;")
        .unwrap();
}

#[test]
fn v38_upgrade_preserves_rows_and_conflicting_index_rolls_back() {
    for conflict in [false, true] {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::migrate_schema(&mut connection).unwrap();
        restore_v38(&connection);
        connection
            .execute("INSERT INTO assets VALUES ('retained-asset', 123)", [])
            .unwrap();
        if conflict {
            connection
                .execute_batch(
                    "CREATE INDEX library_change_queue_superseded_by ON library_change_queue(root_id)",
                )
                .unwrap();
            assert_eq!(
                migrate(&mut connection).unwrap_err().code,
                "catalog_queue_retirement_index_invalid"
            );
        } else {
            migrate(&mut connection).unwrap();
            validate(&connection).unwrap();
            super::super::migrate_schema(&mut connection).unwrap();
        }
        let (version, created): (i64, i64) = connection
            .query_row(
                "SELECT version, created_unix_ms FROM schema_info, assets WHERE assets.id = 'retained-asset'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(version, if conflict { 38 } else { 39 });
        assert_eq!(created, 123);
        if conflict {
            let added: i64 = connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name IN (
                     'library_media_admission_rechecks_change', 'library_media_admission_rechecks_asset')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(added, 0);
        }
    }
}

#[test]
fn retiring_one_change_does_not_traverse_unrelated_queue_or_recheck_rows() {
    let mut counts = Vec::new();
    for entries in [64, 4096] {
        let directory = tempfile::tempdir().unwrap();
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        catalog
            .connection
            .execute_batch(
                "INSERT INTO library_roots(id,path,created_unix_ms) VALUES('root','C:/fixture',1);
             INSERT INTO library_change_root_state(root_id,generation,is_active,updated_unix_ms)
               VALUES('root',1,1,1);
             INSERT INTO assets(id,created_unix_ms) VALUES('retained-asset',1);",
            )
            .unwrap();
        catalog.connection.execute(
            "WITH RECURSIVE sequence(id) AS (
               SELECT 1 UNION ALL SELECT id + 1 FROM sequence WHERE id < ?1
             )
             INSERT INTO library_change_queue(
               id,root_id,root_generation,intent_kind,scope,relative_path,origin,
               first_observed_unix_ms,most_recent_observed_unix_ms,first_sequence,most_recent_sequence,
               coalesced_observation_count,status,ready_unix_ms,catalog_revision_at_enqueue,created_unix_ms,updated_unix_ms)
             SELECT id,'root',1,'reconcile','path','item-' || id || '.png','consistency_audit',
               1,1,'1','1',1,'completed',1,0,1,CASE WHEN id = 1 THEN 1 ELSE 1000000 END FROM sequence",
            [entries],
        ).unwrap();
        catalog
            .connection
            .execute_batch(
                "UPDATE library_change_queue SET superseded_by_change_id = 1 WHERE id = 2;
             INSERT INTO library_media_admission_rechecks(
               root_id,root_generation,relative_path,location_id,asset_id,state,change_id)
             SELECT 'root',1,relative_path,relative_path,'retained-asset','pending',id
             FROM library_change_queue;",
            )
            .unwrap();
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&steps);
        catalog
            .connection
            .progress_handler(
                1,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        assert_eq!(catalog.cleanup_terminal_library_changes(2, 1).unwrap(), 1);
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        counts.push(steps.load(Ordering::Relaxed));
        let retained: (i64, i64, bool, bool) = catalog.connection.query_row(
            "SELECT
             (SELECT count(*) FROM library_change_queue),
             (SELECT count(*) FROM library_media_admission_rechecks),
             (SELECT superseded_by_change_id IS NULL FROM library_change_queue WHERE id = 2),
             (SELECT change_id IS NULL FROM library_media_admission_rechecks WHERE relative_path = 'item-1.png')",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).unwrap();
        assert_eq!(retained, (entries - 1, entries, true, true));
    }
    assert!(
        counts[1] <= counts[0] * 2,
        "Bounded queue retirement must not walk unrelated history: {counts:?}"
    );
}
