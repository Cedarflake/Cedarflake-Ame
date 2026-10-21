use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rusqlite::params;

use crate::adapters::SqliteCatalog;

use super::APPEND_CHILD_DIRECTORIES_SQL;

#[test]
fn child_frontier_work_is_independent_of_other_directories_payload() {
    let small = frontier_steps(1_024);
    let large = frontier_steps(8_192);
    eprintln!("child frontier VM steps: small={small}, large={large}");
    assert!(
        large <= small * 2,
        "one directory must not scan the accumulated library: small={small}, large={large}"
    );
}

fn frontier_steps(unrelated_entries: u32) -> usize {
    let storage = tempfile::tempdir().expect("isolated catalog storage");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("production schema");
    let transaction = catalog
        .connection
        .transaction()
        .expect("fixture transaction");
    transaction
        .execute(
            "INSERT INTO library_metadata_inventory_spools(
               run_id, authority_change_id, root_id, root_generation,
               root_identity_scheme, root_identity_value, scope_kind, scope_relative_path,
               state, created_unix_ms, updated_unix_ms
             ) VALUES ('fixture', 1, 'removed-root', 1, 'fixture', 'identity', 'root', '',
                       'retired', 1, 1)",
            [],
        )
        .expect("owned source manifest");
    for (ordinal, directory) in [(0, "selected"), (1, "unrelated")] {
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state, source_entry_count,
                   created_unix_ms, updated_unix_ms
                 ) VALUES ('fixture', ?1, ?2, 'pending', 0, 1, 1)",
                params![ordinal, directory],
            )
            .expect("owned source directory");
    }
    {
        let mut statement = transaction
            .prepare(
                "INSERT INTO library_metadata_inventory_spool_entries(
                   run_id, directory_relative_path, relative_path, entry_kind, file_size,
                   modified_unix_ms, placeholder_state, is_reparse_point, staged_unix_ms
                 ) VALUES ('fixture', ?1, ?2, ?3, CASE WHEN ?3 = 'file' THEN 1 ELSE NULL END,
                           1, ?4, ?5, 1)",
            )
            .expect("fixture insertion");
        for index in 0..unrelated_entries {
            statement
                .execute(params![
                    "unrelated",
                    format!("unrelated/{index:05}.jpg"),
                    "file",
                    "available",
                    false
                ])
                .expect("unrelated payload");
        }
        for (path, kind, availability, reparse) in [
            ("selected/z", "directory", "available", false),
            ("selected/a", "directory", "available", false),
            ("selected/file.jpg", "file", "available", false),
            ("selected/offline", "directory", "offline", false),
            ("selected/link", "directory", "available", true),
        ] {
            statement
                .execute(params!["selected", path, kind, availability, reparse])
                .expect("selected source observation");
        }
    }
    transaction.commit().expect("fixture committed");

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
        .expect("observe production frontier query");
    let inserted = catalog
        .connection
        .execute(
            APPEND_CHILD_DIRECTORIES_SQL,
            params!["fixture", "selected", 2],
        )
        .expect("append only eligible children");
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire query observer");
    assert_eq!(inserted, 2);
    let mut query = catalog
        .connection
        .prepare(
            "SELECT relative_directory FROM library_metadata_inventory_spool_directories
             WHERE run_id = 'fixture' AND ordinal >= 2 ORDER BY ordinal",
        )
        .expect("read appended frontier");
    let paths = query
        .query_map([], |row| row.get::<_, String>(0))
        .expect("read child paths")
        .collect::<Result<Vec<_>, _>>()
        .expect("decode child paths");
    assert_eq!(paths, ["selected/a", "selected/z"]);
    steps.load(Ordering::Relaxed)
}
