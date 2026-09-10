use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::{Connection, Params, StatementStatus, params};

use crate::domain::{
    AssetLocationView, DiscoveredFile, ExpectedFileState, PreviewStatus, ScanRequest,
};
use crate::ports::CatalogRepository;

use super::{
    FileIdentityEvidence, RejectedInputValidationRoster, SqliteCatalog, StagedValidationRoster,
    load_identity_page,
};

const SCAN: &str = "keyset-scan";
const ROOT: &str = "keyset-root";
const PAGE: u32 = 128;
const EMPTY_PAGE_VM_BUDGET: usize = 128;

#[test]
fn publication_keysets_seek_without_losing_reordered_duplicate_or_nullable_inputs() {
    assert_eq!(
        rusqlite::version(),
        "3.53.2",
        "exercise the pinned bundled SQLite"
    );
    let storage = tempfile::tempdir().expect("owned catalog storage");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("real schema");
    begin(&mut catalog, SCAN, ROOT);
    let mut locations = (0..768).map(location).collect::<Vec<_>>();
    for index in [0, 255, 256, 511, 512, 767] {
        let mut duplicate = locations[index].clone();
        duplicate.location_id = format!("duplicate-{index:04}");
        duplicate.relative_path = format!("duplicate-{index:04}.png");
        duplicate.absolute_path = format!("fixture/{}", duplicate.relative_path);
        locations.push(duplicate);
    }
    let mut unidentified = location(768);
    unidentified.file_identity = None;
    locations.push(unidentified);
    for location in locations.iter().rev() {
        catalog
            .stage_location(SCAN, ROOT, location)
            .expect("real staging port");
    }
    catalog
        .flush_pending_locations()
        .expect("staged observations");
    begin(&mut catalog, "other-scan", "other-root");
    let mut outside = location(999);
    outside.root_id = "other-root".to_owned();
    outside.scan_id = "other-scan".to_owned();
    catalog
        .stage_location("other-scan", "other-root", &outside)
        .expect("other projection");
    catalog
        .flush_pending_locations()
        .expect("other projection staged");

    let transaction = catalog
        .connection
        .unchecked_transaction()
        .expect("read-only page snapshot");
    let mut identities = Vec::new();
    let mut after = None;
    loop {
        let page = load_identity_page(&transaction, SCAN, after.as_ref()).expect("identity page");
        assert!(page.len() <= PAGE as usize);
        if page.is_empty() {
            break;
        }
        after = page.last().cloned();
        identities.extend(page);
    }
    let mut expected = locations
        .iter()
        .filter_map(|item| item.file_identity.clone())
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| (&left.scheme, &left.value).cmp(&(&right.scheme, &right.value)));
    expected.dedup();
    assert_eq!(expected.len(), 768);
    assert_eq!(identities, expected);
    let last = after.expect("multiple identity pages");
    let (empty, operations) = measure_vm(&catalog.connection, || {
        load_identity_page(&transaction, SCAN, Some(&last)).expect("identity cursor seek")
    });
    assert!(empty.is_empty());
    let old = legacy_vm_steps(&catalog.connection,
        "SELECT DISTINCT file_identity_scheme, file_identity_value FROM asset_locations
         WHERE scan_id = ?1 AND file_identity_scheme IS NOT NULL
           AND (?2 IS NULL OR file_identity_scheme > ?2 OR (file_identity_scheme = ?2 AND file_identity_value > ?3))
         ORDER BY file_identity_scheme, file_identity_value LIMIT ?4",
        params![SCAN, last.scheme, last.value, PAGE]);
    assert_seek_budget("identity", operations, old);
    drop(transaction);

    let positive =
        StagedValidationRoster::capture(&mut catalog, SCAN).expect("real positive roster");
    let negative = RejectedInputValidationRoster::create(&catalog).expect("real negative roster");
    for item in locations.iter().rev() {
        negative
            .record(
                &catalog,
                &DiscoveredFile {
                    source_root_path: "fixture".to_owned(),
                    absolute_path: item.absolute_path.clone(),
                    relative_path: item.relative_path.clone(),
                    file_size: item.file_size,
                    created_unix_ms: None,
                    modified_unix_ms: item.modified_unix_ms,
                    file_identity: item.file_identity.clone(),
                    source_revision: item.source_revision.clone(),
                    source_generation: 0,
                    issues: Vec::new(),
                },
            )
            .expect("negative observation");
    }
    assert_eq!(positive.total_items(), locations.len() as u64);
    let mut positive_ids = Vec::new();
    let mut after = None;
    loop {
        let page = positive
            .load_window(&catalog, after.as_deref(), PAGE)
            .expect("positive page");
        assert!(page.len() <= PAGE as usize);
        if page.is_empty() {
            break;
        }
        after = page.last().map(|item| item.0.clone());
        for (id, path, state) in page {
            let expected = locations
                .iter()
                .find(|item| item.location_id == id)
                .expect("known location");
            assert_eq!(path, expected.relative_path);
            assert_state(&state, expected);
            positive_ids.push(id);
        }
    }
    let mut expected_ids = locations
        .iter()
        .map(|item| item.location_id.clone())
        .collect::<Vec<_>>();
    expected_ids.sort();
    assert_eq!(positive_ids, expected_ids);
    let last = after.expect("multiple positive pages");
    let (empty, operations) = measure_vm(&catalog.connection, || {
        positive
            .load_window(&catalog, Some(&last), PAGE)
            .expect("positive cursor seek")
    });
    assert!(empty.is_empty());
    let table = temp_table(&catalog.connection, "ame_scan_validation_%");
    let old = legacy_vm_steps(
        &catalog.connection,
        &format!(
            "SELECT location_id, relative_path, absolute_path, file_size, modified_unix_ms,
                file_identity_scheme, file_identity_value, source_revision_token
         FROM temp.{table} WHERE (?1 IS NULL OR location_id > ?1) ORDER BY location_id LIMIT ?2"
        ),
        params![last, PAGE],
    );
    assert_seek_budget("positive", operations, old);
    assert!(
        positive
            .load_window(&catalog, None, 0)
            .expect("zero limit preserved")
            .is_empty()
    );

    let mut negative_paths = Vec::new();
    let mut after = None;
    loop {
        let page = negative
            .load_window(&catalog, after.as_deref(), PAGE)
            .expect("negative page");
        assert!(page.len() <= PAGE as usize);
        if page.is_empty() {
            break;
        }
        after = page.last().map(|item| item.0.clone());
        for (path, state) in page {
            let expected = locations
                .iter()
                .find(|item| item.relative_path == path)
                .expect("known rejected source");
            assert_state(&state, expected);
            negative_paths.push(path);
        }
    }
    let mut expected_paths = locations
        .iter()
        .map(|item| item.relative_path.clone())
        .collect::<Vec<_>>();
    expected_paths.sort();
    assert_eq!(negative_paths, expected_paths);
    let last = after.expect("multiple negative pages");
    let (empty, operations) = measure_vm(&catalog.connection, || {
        negative
            .load_window(&catalog, Some(&last), PAGE)
            .expect("negative cursor seek")
    });
    assert!(empty.is_empty());
    let table = temp_table(&catalog.connection, "ame_rejected_input_validation_%");
    let old = legacy_vm_steps(&catalog.connection, &format!(
        "SELECT relative_path, absolute_path, file_size, modified_unix_ms,
                identity_scheme, identity_value, source_revision
         FROM temp.{table} WHERE (?1 IS NULL OR relative_path > ?1) ORDER BY relative_path LIMIT ?2"),
        params![last, PAGE]);
    assert_seek_budget("negative", operations, old);
    assert!(
        negative
            .load_window(&catalog, None, 0)
            .expect("zero limit preserved")
            .is_empty()
    );
}

#[test]
fn directory_entry_keyset_seeks_after_completed_enumeration_without_losing_paths() {
    let storage = tempfile::tempdir().expect("owned catalog storage");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("real schema");
    begin(&mut catalog, SCAN, ROOT);
    assert_eq!(
        catalog.claim_next_directory(SCAN).expect("claim directory"),
        Some(String::new())
    );
    let paths = (0..768)
        .map(|index| format!("图片-{index:04}.png"))
        .collect::<Vec<_>>();
    assert_eq!(
        catalog
            .load_directory_entry_window(SCAN, "", None, PAGE)
            .expect_err("not ready")
            .code,
        "catalog_scan_directory_not_ready"
    );
    let reversed = paths.iter().rev().cloned().collect::<Vec<_>>();
    catalog
        .stage_directory_entries(SCAN, "", &reversed)
        .expect("real staged directory");
    catalog
        .stage_directory_entries(SCAN, "", &paths[..3])
        .expect("duplicate staging");
    catalog
        .complete_directory_enumeration(SCAN, "")
        .expect("ready directory");
    let mut found = Vec::new();
    let mut after = None;
    loop {
        let page = catalog
            .load_directory_entry_window(SCAN, "", after.as_deref(), PAGE)
            .expect("directory page");
        assert!(page.len() <= PAGE as usize);
        if page.is_empty() {
            break;
        }
        after = page.last().cloned();
        found.extend(page);
    }
    assert_eq!(found, paths);
    let last = after.expect("multiple directory pages");
    let (empty, operations) = measure_vm(&catalog.connection, || {
        catalog
            .load_directory_entry_window(SCAN, "", Some(&last), PAGE)
            .expect("directory cursor seek")
    });
    assert!(empty.is_empty());
    let old = legacy_vm_steps(
        &catalog.connection,
        "SELECT relative_path FROM scan_directory_entries
         WHERE scan_id = ?1 AND directory_relative_path = ?2
           AND (?3 IS NULL OR relative_path > ?3) ORDER BY relative_path LIMIT ?4",
        params![SCAN, "", last, PAGE],
    );
    assert_seek_budget("directory", operations, old);
    assert_eq!(
        catalog
            .load_directory_entry_window(SCAN, "", None, 0)
            .expect_err("zero limit refused")
            .code,
        "directory_entry_window_invalid"
    );
    assert_eq!(
        catalog
            .load_directory_entry_window(SCAN, "other", None, PAGE)
            .expect_err("wrong directory refused")
            .code,
        "catalog_scan_directory_not_ready"
    );
}

fn measure_vm<T>(connection: &Connection, operation: impl FnOnce() -> T) -> (T, usize) {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&count);
    connection
        .progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .expect("per-instruction production-query observer");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    let cleanup = connection.progress_handler(0, None::<fn() -> bool>);
    match result {
        Ok(result) => {
            cleanup.expect("remove query observer");
            (result, count.load(Ordering::Relaxed))
        }
        Err(error) => std::panic::resume_unwind(error),
    }
}

fn legacy_vm_steps(connection: &Connection, sql: &str, parameters: impl Params) -> usize {
    let mut statement = connection.prepare(sql).expect("original nullable-OR query");
    let mut rows = statement.query(parameters).expect("original query binding");
    assert!(rows.next().expect("original exhausted page").is_none());
    drop(rows);
    usize::try_from(statement.get_status(StatementStatus::VmStep)).expect("non-negative VM steps")
}

fn assert_seek_budget(owner: &str, production_operations: usize, legacy_steps: usize) {
    assert!(
        production_operations > 0 && production_operations <= EMPTY_PAGE_VM_BUDGET,
        "{owner} exhausted cursor must seek, not visit the whole prefix: {production_operations}"
    );
    assert!(
        legacy_steps > EMPTY_PAGE_VM_BUDGET,
        "{owner} original OR must fail the same seek budget: {legacy_steps}"
    );
    println!(
        "keyset owner={owner} production_progress_ops={production_operations} legacy_vm_steps={legacy_steps}"
    );
}

fn temp_table(connection: &Connection, pattern: &str) -> String {
    let names = connection
        .prepare("SELECT name FROM sqlite_temp_master WHERE type = 'table' AND name LIKE ?1")
        .expect("real roster table lookup")
        .query_map([pattern], |row| row.get::<_, String>(0))
        .expect("roster names")
        .collect::<Result<Vec<_>, _>>()
        .expect("roster table");
    assert_eq!(names.len(), 1);
    names[0].clone()
}

fn assert_state(actual: &ExpectedFileState, expected: &AssetLocationView) {
    assert_eq!(actual.absolute_path, expected.absolute_path);
    assert_eq!(actual.file_size, expected.file_size);
    assert_eq!(actual.modified_unix_ms, expected.modified_unix_ms);
    assert_eq!(actual.file_identity, expected.file_identity);
    assert_eq!(actual.source_revision, expected.source_revision);
}

fn begin(catalog: &mut SqliteCatalog, scan_id: &str, root_id: &str) {
    catalog
        .begin_scan(
            &ScanRequest {
                scan_id: scan_id.to_owned(),
                root_path: root_id.to_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            root_id,
            root_id,
        )
        .expect("begin real catalog scan");
}

fn location(index: usize) -> AssetLocationView {
    let name = format!("图片-{index:04}.png");
    AssetLocationView {
        asset_id: format!("asset-{index:04}"),
        location_id: format!("location-{index:04}"),
        root_id: ROOT.to_owned(),
        scan_id: SCAN.to_owned(),
        absolute_path: format!("fixture/{name}"),
        display_path: name.clone(),
        relative_path: name,
        preview_path: String::new(),
        file_size: 42,
        created_unix_ms: None,
        modified_unix_ms: 42,
        file_identity: Some(FileIdentityEvidence {
            scheme: format!("scheme-{}", index / 256),
            value: format!("{:04}", index % 256),
        }),
        source_revision: None,
        source_generation: 0,
        width: 8,
        height: 8,
        preview_status: PreviewStatus::Pending,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "fixture".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    }
}
