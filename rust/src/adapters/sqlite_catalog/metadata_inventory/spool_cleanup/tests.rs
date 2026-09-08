use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::adapters::SqliteCatalog;

#[test]
fn bounded_cleanup_hint_does_not_scan_active_raw_payload() {
    use crate::adapters::sqlite_catalog::migrations::inventory_spool_rows::tests::fixture;
    let mut counts = Vec::new();
    for entries in [1_024, 8_192] {
        let connection = fixture(1, entries);
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&steps);
        connection
            .progress_handler(
                1,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .expect("observe actual candidate hint");
        assert!(!super::has_candidates(&connection).expect("no debt among active data"));
        connection
            .progress_handler(0, None::<fn() -> bool>)
            .expect("retire observer");
        counts.push(steps.load(Ordering::Relaxed));
    }
    eprintln!("bounded cleanup hint VM steps: {counts:?}");
    assert!(
        counts[1] <= counts[0] * 2,
        "idle cleanup cannot scan the active library: {counts:?}"
    );
}

#[test]
fn bounded_raw_cleanup_does_not_sort_or_visit_the_remaining_directory_payload() {
    assert_bounded(RawShape::Entries);
}

#[test]
fn bounded_raw_cleanup_does_not_visit_all_nonempty_directories() {
    assert_bounded(RawShape::Directories);
}

#[test]
fn bounded_raw_cleanup_does_not_visit_all_retired_headers() {
    assert_bounded(RawShape::Headers);
}

#[derive(Clone, Copy, Debug)]
enum RawShape {
    Entries,
    Directories,
    Headers,
}

fn assert_bounded(shape: RawShape) {
    let small = cleanup_steps(1_024, shape);
    let large = cleanup_steps(8_192, shape);
    eprintln!("bounded raw cleanup VM steps: small={small}, large={large}");
    assert!(
        large <= small * 2,
        "an eightfold retained payload must not expand a one-entry batch: small={small}, large={large}"
    );
}

fn cleanup_steps(entries: u32, shape: RawShape) -> usize {
    let storage = tempfile::tempdir().expect("isolated catalog storage");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("production schema");
    let transaction = catalog
        .connection
        .transaction()
        .expect("generated retired payload");
    for index in 0..entries {
        let run = if matches!(shape, RawShape::Headers) {
            format!("retired-{index:05}")
        } else {
            "retired".to_owned()
        };
        let directory = if matches!(shape, RawShape::Directories) {
            format!("directory-{index:05}")
        } else {
            String::new()
        };
        if index == 0 || matches!(shape, RawShape::Headers) {
            transaction.execute("INSERT INTO library_metadata_inventory_spools(
        run_id, authority_change_id, root_id, root_generation, root_identity_scheme, root_identity_value,
        scope_kind, scope_relative_path, state, created_unix_ms, updated_unix_ms)
        VALUES (?1, 1, 'removed-root', 1, 'fixture', 'identity', 'root', '', 'retired', 1, 1)", [&run]).expect("retired header");
        }
        if index == 0 || !matches!(shape, RawShape::Entries) {
            transaction.execute("INSERT INTO library_metadata_inventory_spool_directories(
        run_id, ordinal, relative_directory, state, directory_identity_scheme, directory_identity_value,
        source_entry_count, created_unix_ms, updated_unix_ms)
        VALUES (?1, ?2, ?3, 'completed', 'fixture', 'identity', 0, 1, 1)", rusqlite::params![run, index, directory]).expect("retired directory");
        }
        transaction
            .execute(
                "INSERT INTO library_metadata_inventory_spool_entries(
            run_id, directory_relative_path, relative_path, entry_kind, file_size, modified_unix_ms,
            placeholder_state, is_reparse_point, staged_unix_ms)
            VALUES (?1, ?2, ?3, 'file', 1, 1, 'available', 0, 1)",
                rusqlite::params![run, directory, format!("file-{index:05}.png")],
            )
            .expect("generated raw metadata only");
    }
    transaction.commit().expect("durable retired payload");
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
        .expect("count executed SQLite VM work");
    let transaction = catalog.connection.transaction().expect("one bounded batch");
    let result = super::cleanup_batch(&transaction, 1, 1);
    transaction.commit().expect("commit bounded cleanup");
    catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .expect("retire VM observer");
    assert_eq!(result.expect("one-entry cleanup"), 1);
    steps.load(Ordering::Relaxed)
}
