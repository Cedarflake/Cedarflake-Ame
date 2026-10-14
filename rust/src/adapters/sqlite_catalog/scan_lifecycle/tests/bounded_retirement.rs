use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::adapters::SqliteCatalogSpaceMaintenance;
use crate::domain::ScanCheckpoint;
use crate::ports::{
    CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogRepository,
    CatalogSpaceRepository, RetainedScanRepository,
};

use super::{PublicationFixture, SqliteCatalog};

#[test]
fn cancellation_retires_authority_before_restart_safe_staging_cleanup() {
    let mut fixture = large_update(512);
    fixture
        .catalog
        .abandon_scan("cancelled", "cancelled", 3)
        .unwrap();
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        385
    );
    assert_eq!(fixture.count("asset_locations", "scan_id = 'active'"), 1);
    assert_eq!(
        fixture.count(
            "scan_runs",
            "id = 'cancelled' AND status = 'cancelled' AND issue_count = 3"
        ),
        1
    );
    reopen(&mut fixture);

    // A new task can use this root while the old task's derived rows await reclamation.
    fixture.begin_and_stage("next", "root");
    fixture
        .catalog
        .pause_scan("next", &ScanCheckpoint::default())
        .unwrap();
    let maintenance = maintenance(&fixture);
    let control = control();
    let mut remaining = 385;
    while remaining > 0 {
        let removed = completed(
            maintenance
                .try_discard_retired_scan_staging(&control)
                .unwrap(),
        );
        assert!((1..=128).contains(&removed));
        remaining -= removed as i64;
        assert_eq!(
            fixture.count("asset_locations", "scan_id = 'cancelled'"),
            remaining
        );
        assert_eq!(fixture.count("asset_locations", "scan_id = 'active'"), 1);
        assert_eq!(fixture.count("asset_locations", "scan_id = 'next'"), 1);
        reopen(&mut fixture);
    }
    assert_eq!(fixture.count("assets", "id = 'asset-cancelled'"), 0);
    assert_eq!(
        completed(
            maintenance
                .try_discard_retired_scan_staging(&control)
                .unwrap()
        ),
        0
    );
}

#[test]
fn retained_first_import_cancellation_clears_resume_authority_before_deferred_cleanup() {
    let mut fixture = PublicationFixture::new();
    fixture.begin_and_stage("cancelled", "root");
    duplicate_staging(&mut fixture, 512);
    fixture
        .catalog
        .connection
        .execute(
            "INSERT INTO scan_resume_pending_locations(scan_id, location_id)
         SELECT scan_id, location_id FROM asset_locations WHERE scan_id = 'cancelled'",
            [],
        )
        .unwrap();
    fixture
        .catalog
        .pause_scan("cancelled", &ScanCheckpoint::default())
        .unwrap();
    fixture.catalog.cancel_retained_scan("cancelled").unwrap();
    assert_eq!(
        fixture.count("scan_resume_pending_locations", "scan_id = 'cancelled'"),
        0
    );
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        385
    );
    reopen(&mut fixture);
    assert!(
        fixture
            .catalog
            .load_single_recoverable_foreground_scan()
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .catalog
            .load_single_paused_foreground_scan()
            .unwrap()
            .is_none()
    );
    let maintenance = maintenance(&fixture);
    drain_staging(&maintenance);
    assert_eq!(fixture.count("asset_locations", "scan_id = 'cancelled'"), 0);
    reopen(&mut fixture);
}

#[test]
fn interrupted_update_retirement_keeps_the_published_snapshot_while_cleanup_is_pending() {
    let mut fixture = large_update(512);
    assert!(
        fixture
            .catalog
            .load_single_recoverable_foreground_scan()
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture.count("scan_runs", "id = 'cancelled' AND status = 'failed'"),
        1
    );
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        385
    );
    assert_eq!(fixture.count("asset_locations", "scan_id = 'active'"), 1);
    reopen(&mut fixture);
    let maintenance = maintenance(&fixture);
    drain_staging(&maintenance);
    assert_eq!(fixture.count("asset_locations", "scan_id = 'active'"), 1);
    reopen(&mut fixture);
}

#[test]
fn failed_background_cleanup_preserves_terminal_state_and_retries_the_same_batch() {
    let mut fixture = large_update(512);
    fixture
        .catalog
        .abandon_scan("cancelled", "cancelled", 0)
        .unwrap();
    fixture
        .catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_retired_batch BEFORE DELETE ON asset_locations
         WHEN OLD.scan_id = 'cancelled'
         BEGIN SELECT RAISE(ABORT, 'injected retired batch failure'); END;",
        )
        .unwrap();
    let maintenance = maintenance(&fixture);
    let error = maintenance
        .try_discard_retired_scan_staging(&control())
        .err()
        .unwrap();
    assert!(error.message.contains("injected retired batch failure"));
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        385
    );
    assert_eq!(
        fixture.count("scan_runs", "id = 'cancelled' AND status = 'cancelled'"),
        1
    );
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_retired_batch")
        .unwrap();
    assert_eq!(
        completed(
            maintenance
                .try_discard_retired_scan_staging(&control())
                .unwrap()
        ),
        128
    );
    reopen(&mut fixture);
}

#[test]
fn background_cleanup_yields_to_cancellation_and_an_existing_writer() {
    let mut fixture = large_update(512);
    fixture
        .catalog
        .abandon_scan("cancelled", "cancelled", 0)
        .unwrap();
    let maintenance = maintenance(&fixture);
    let interrupted = control();
    interrupted.preempt();
    assert!(matches!(
        maintenance
            .try_discard_retired_scan_staging(&interrupted)
            .unwrap(),
        CatalogMaintenanceAttempt::Interrupted
    ));
    fixture
        .catalog
        .connection
        .execute_batch("BEGIN IMMEDIATE")
        .unwrap();
    assert!(matches!(
        maintenance
            .try_discard_retired_scan_staging(&control())
            .unwrap(),
        CatalogMaintenanceAttempt::Busy
    ));
    fixture
        .catalog
        .connection
        .execute_batch("ROLLBACK")
        .unwrap();
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        385
    );
    assert_eq!(
        completed(
            maintenance
                .try_discard_retired_scan_staging(&control())
                .unwrap()
        ),
        128
    );
}

#[test]
fn foreground_retirement_work_does_not_grow_with_the_remaining_staging() {
    let small = cancellation_work(512);
    let large = cancellation_work(4096);
    assert!(small > 0);
    assert!(
        large <= small * 2,
        "bounded cancellation work: small={small}, large={large}"
    );
}

fn cancellation_work(extra_locations: usize) -> usize {
    let mut fixture = large_update(extra_locations);
    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    fixture
        .catalog
        .connection
        .progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    fixture
        .catalog
        .abandon_scan("cancelled", "cancelled", 0)
        .unwrap();
    fixture
        .catalog
        .connection
        .progress_handler(0, None::<fn() -> bool>)
        .unwrap();
    assert_eq!(
        fixture.count("asset_locations", "scan_id = 'cancelled'"),
        extra_locations as i64 + 1 - 128
    );
    steps.load(Ordering::Relaxed)
}

fn large_update(extra_locations: usize) -> PublicationFixture {
    let mut fixture = PublicationFixture::new();
    fixture.publish("active", "root");
    fixture.begin_and_stage("cancelled", "root");
    duplicate_staging(&mut fixture, extra_locations);
    fixture
}

fn duplicate_staging(fixture: &mut PublicationFixture, extra_locations: usize) {
    let columns = fixture
        .catalog
        .connection
        .prepare("PRAGMA table_info(asset_locations)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let selections = columns
        .iter()
        .map(|column| match column.as_str() {
            "location_id" | "absolute_path" | "display_path" | "relative_path" => {
                format!("source.{column} || printf('-%06d', copies.number)")
            }
            _ => format!("source.{column}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    fixture
        .catalog
        .connection
        .execute(
            &format!(
                "WITH RECURSIVE copies(number) AS (
           SELECT 1 UNION ALL SELECT number + 1 FROM copies WHERE number < ?1
         ) INSERT INTO asset_locations SELECT {selections}
         FROM asset_locations AS source CROSS JOIN copies
         WHERE source.scan_id = 'cancelled' AND source.location_id = 'location-cancelled'"
            ),
            [extra_locations as i64],
        )
        .unwrap();
}

fn maintenance(fixture: &PublicationFixture) -> SqliteCatalogSpaceMaintenance {
    SqliteCatalogSpaceMaintenance::new(fixture.directory.path().join("catalog.sqlite3"))
}

fn control() -> CatalogMaintenanceControl {
    CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)))
}

fn drain_staging(maintenance: &SqliteCatalogSpaceMaintenance) {
    for _ in 0..8 {
        if completed(
            maintenance
                .try_discard_retired_scan_staging(&control())
                .unwrap(),
        ) == 0
        {
            return;
        }
    }
    panic!("the retained fixture exceeded its bounded cleanup batch count");
}

fn completed(attempt: CatalogMaintenanceAttempt<usize>) -> usize {
    match attempt {
        CatalogMaintenanceAttempt::Completed(count) => count,
        _ => panic!("bounded cleanup did not complete"),
    }
}

fn reopen(fixture: &mut PublicationFixture) {
    let path = fixture.directory.path().join("catalog.sqlite3");
    // Validate through a new connection while the fixture retains its observation connection.
    let reopened =
        SqliteCatalog::open(path).expect("terminal staging keeps the full catalog valid");
    fixture.catalog = reopened;
}
