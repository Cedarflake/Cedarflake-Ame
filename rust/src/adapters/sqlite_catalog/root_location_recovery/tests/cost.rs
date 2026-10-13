use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::*;

thread_local! {
    static AFTER_PAGE: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

pub(super) fn observe_next_page(callback: impl FnOnce() + 'static) {
    AFTER_PAGE.with(|hook| *hook.borrow_mut() = Some(Box::new(callback)));
}

pub(crate) fn after_recovery_write_page(_root_id: &str) {
    AFTER_PAGE.with(|hook| {
        if let Some(callback) = hook.borrow_mut().take() {
            callback();
        }
    });
}

#[test]
fn root_location_recovery_preempts_partial_eighty_thousand_location_rebase_for_live_work() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_aliases(&catalog, &fixture);
    let mut live = catalog
        .validated_session()
        .open_in_lane(LibraryChangeLane::Live)
        .unwrap();
    let located = fixture.located();
    let (sender, receiver) = mpsc::sync_channel(1);
    let (outcome, live_elapsed) = thread::scope(|scope| {
        let recovery = scope.spawn(|| {
            observe_next_page(move || sender.try_send(()).unwrap());
            catalog.recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        });
        receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        let started = Instant::now();
        let transaction = live.begin_write_in_lane(LibraryChangeLane::Live).unwrap();
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .unwrap();
        transaction.commit().unwrap();
        (recovery.join().unwrap(), started.elapsed())
    });
    assert_eq!(
        outcome.unwrap_err().code,
        "root_location_recovery_preempted"
    );
    assert!(
        live_elapsed < Duration::from_secs(1),
        "Live write must preempt the rebase: {live_elapsed:?}"
    );
    let retained = catalog
        .load_incremental_catalog_root(&fixture.root.root_id)
        .unwrap()
        .unwrap();
    assert_eq!(retained.root_path, fixture.root.root_path);
    assert_eq!(retained.root_generation, fixture.root.root_generation);
    assert_paths(&catalog, &fixture, &fixture.root.root_path);
    drop(catalog);
    let mut catalog = fixture.catalog();
    let started = Instant::now();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    let elapsed = started.elapsed();
    assert_paths(&catalog, &fixture, located.path());
    eprintln!(
        "ROOT_LOCATION_REBASE rows=80002 live_wait_ms={} complete_elapsed_ms={}",
        live_elapsed.as_millis(),
        elapsed.as_millis()
    );
    drop(catalog);
    drop(fixture.catalog());
}

#[test]
fn root_location_recovery_rolls_back_inventory_pages_when_live_work_preempts() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    super::inventory::seed_inventory(&mut catalog, &fixture.root, 4_096);
    seed_aliases(&catalog, &fixture);
    drop(catalog);
    let mut catalog = fixture.catalog();
    let mut live = catalog
        .validated_session()
        .open_in_lane(LibraryChangeLane::Live)
        .unwrap();
    let located = fixture.located();
    let (sender, receiver) = mpsc::sync_channel(1);
    let (outcome, live_elapsed) = thread::scope(|scope| {
        let recovery = scope.spawn(|| {
            observe_next_page(move || sender.try_send(()).unwrap());
            catalog.recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        });
        receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        let started = Instant::now();
        let transaction = live.begin_write_in_lane(LibraryChangeLane::Live).unwrap();
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .unwrap();
        transaction.commit().unwrap();
        (recovery.join().unwrap(), started.elapsed())
    });
    assert_eq!(
        outcome.unwrap_err().code,
        "root_location_recovery_preempted"
    );
    assert!(
        live_elapsed < Duration::from_secs(1),
        "Live wait after logical cleanup: {live_elapsed:?}"
    );
    let entries: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = 'old-inventory'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(entries, 4_096);
    assert_paths(&catalog, &fixture, &fixture.root.root_path);
    drop(catalog);
    let mut catalog = fixture.catalog();
    let started = Instant::now();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    eprintln!(
        "ROOT_LOCATION_INVENTORY rows=4096 locations=80002 live_wait_ms={} complete_elapsed_ms={}",
        live_elapsed.as_millis(),
        started.elapsed().as_millis()
    );
    assert_paths(&catalog, &fixture, located.path());
    drop(catalog);
    drop(fixture.catalog());
}

fn seed_aliases(catalog: &SqliteCatalog, fixture: &Fixture) {
    let columns = {
        let mut statement = catalog
            .connection
            .prepare("PRAGMA table_info(asset_locations)")
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    let expressions = columns
        .iter()
        .map(|column| match column.as_str() {
            "location_id" => "'alias-' || printf('%06d', n)".to_owned(),
            "relative_path" => "'generated/' || printf('%06d', n) || '.png'".to_owned(),
            "absolute_path" => {
                "?2 || char(92) || 'generated' || char(92) || printf('%06d', n) || '.png'"
                    .to_owned()
            }
            _ => format!("source.{column}"),
        })
        .collect::<Vec<_>>();
    catalog
        .connection
        .execute(
            &format!(
        "WITH RECURSIVE numbers(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM numbers WHERE n < 80000)
         INSERT INTO asset_locations({})
         SELECT {} FROM numbers CROSS JOIN asset_locations AS source
         WHERE source.location_id = ?1 AND source.scan_id = ?3",
        columns.join(", "), expressions.join(", "),
    ),
            params![
                fixture.before[0].location_id,
                fixture.root.root_path,
                fixture.root.active_scan_id
            ],
        )
        .unwrap();
}

fn assert_paths(catalog: &SqliteCatalog, fixture: &Fixture, root_path: &str) {
    let incorrect: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM asset_locations WHERE root_id = ?1 AND scan_id = ?2
           AND substr(absolute_path, 1, length(?3)) <> ?3",
            params![fixture.root.root_id, fixture.root.active_scan_id, root_path],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(incorrect, 0);
}
