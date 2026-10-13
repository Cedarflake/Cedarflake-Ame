use rusqlite::{Connection, params};

use super::super::super::{
    ProductionSynchronization, new_production_synchronization_with_connection,
    poll_runtime_with_storage,
};
use super::*;
use crate::adapters::SqliteCatalog;
use crate::application::storage::StoragePaths;
use crate::journal_broker::{
    PersistentChangeJournalConnection, PersistentChangeJournalLiveOnlyReason,
};

struct CleanupDebtFixture {
    _directory: tempfile::TempDir,
    storage: StoragePaths,
    source_starts: Arc<AtomicUsize>,
}

impl CleanupDebtFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("isolated cleanup storage");
        let storage = StoragePaths {
            catalog_path: directory.path().join("catalog.sqlite3"),
            preview_root: directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: directory.path().join("settings.sqlite3"),
        };
        SqliteCatalog::open(storage.catalog_path.clone()).expect("initialize production schema");
        Self {
            _directory: directory,
            storage,
            source_starts: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn connection(&self) -> Connection {
        let connection = Connection::open(&self.storage.catalog_path).expect("owned metadata only");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .expect("keep production foreign keys enabled");
        connection
    }

    fn seed_orphans(&self) {
        let mut connection = self.connection();
        let transaction = connection
            .transaction()
            .expect("atomic historical debt fixture");
        {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO library_metadata_inventory_spool_entries(
                   run_id, relative_path, entry_kind, modified_unix_ms,
                   placeholder_state, is_reparse_point, staged_unix_ms
                 ) VALUES (?1, 'album', 'directory', 1, 'available', 0, 1)",
                )
                .expect("prepare historical NULL-parent observation");
            for index in 0..4_097 {
                statement
                    .execute([format!("historical-orphan-{index:05}")])
                    .expect("historical NULL-parent observation without a header");
            }
        }
        transaction
            .commit()
            .expect("persist historical cleanup debt");
        self.assert_valid_rootless_catalog();
    }

    fn seed_empty_retired_spools(&self) {
        let mut connection = self.connection();
        let transaction = connection
            .transaction()
            .expect("atomic retired metadata fixture");
        for index in 0..129 {
            let run_id = format!("retired-run-{index:03}");
            transaction
                .execute(
                    "INSERT INTO library_metadata_inventory_spools(
                   run_id, authority_change_id, root_id, root_generation,
                   root_identity_scheme, root_identity_value, scope_kind, scope_relative_path,
                   state, created_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, 'removed-root', 1, 'fixture', 'retired-identity',
                           'root', '', 'retired', 1, 1)",
                    params![run_id, index + 1],
                )
                .expect("retired header survives deleted execution owners");
            transaction
                .execute(
                    "INSERT INTO library_metadata_inventory_spool_directories(
                   run_id, ordinal, relative_directory, state, created_unix_ms, updated_unix_ms
                 ) VALUES (?1, 0, '', 'pending', 1, 1)",
                    [&run_id],
                )
                .expect("empty retired directory remains physically owned by the header");
        }
        transaction.commit().expect("persist retired metadata debt");
        self.assert_valid_rootless_catalog();
    }

    fn assert_valid_rootless_catalog(&self) {
        SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("fresh FULL open accepts persisted cleanup debt");
        let owners: (i64, i64, i64, i64, i64) = self
            .connection()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM library_roots),
                    (SELECT COUNT(*) FROM library_metadata_inventory_runs),
                    (SELECT COUNT(*) FROM library_recovery_authorities),
                    (SELECT COUNT(*) FROM library_change_queue),
                    (SELECT COUNT(*) FROM scan_runs)",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("no root path, scan or execution authority in the fixture");
        assert_eq!(owners, (0, 0, 0, 0, 0));
        assert_eq!(self.source_starts.load(Ordering::Relaxed), 0);
    }

    fn debt(&self) -> (i64, i64, i64) {
        self.connection()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries),
                    (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories),
                    (SELECT COUNT(*) FROM library_metadata_inventory_spools)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("count persisted debt")
    }

    fn runtime(&self) -> ProductionSynchronization {
        let source_starts = Arc::clone(&self.source_starts);
        new_production_synchronization_with_connection(
            Arc::new(move |_| {
                source_starts.fetch_add(1, Ordering::Relaxed);
                panic!("rootless cleanup cannot start a source watcher");
            }),
            PersistentChangeJournalConnection::LiveOnly(
                PersistentChangeJournalLiveOnlyReason::PortableDistribution,
            ),
        )
    }

    fn poll(&self, runtime: &mut ProductionSynchronization) {
        let snapshot = poll_runtime_with_storage(runtime, &self.storage)
            .expect("real production poll performs rootless maintenance");
        assert!(snapshot.roots.is_empty());
        assert!(runtime.live.is_none());
        assert!(runtime.journal.is_none());
        assert!(runtime.recovery.is_none());
        assert!(runtime.recovery_inventory_sources.is_empty());
        assert_eq!(self.source_starts.load(Ordering::Relaxed), 0);
    }
}

fn stop_without_draining(runtime: &mut ProductionSynchronization) {
    let deadline = Instant::now() + Duration::from_secs(2);
    runtime
        .finish_stopping_until(deadline)
        .expect("cleanup shares the production absolute stop deadline");
    runtime
        .finish_stopping_until(deadline)
        .expect("confirmed retirement does not require extending that deadline");
    assert!(runtime.inventory_cleanup.task.is_none());
    assert!(runtime.stop_requested.load(Ordering::Acquire));
}

fn assert_idle(fixture: &CleanupDebtFixture, runtime: &mut ProductionSynchronization) {
    fixture.poll(runtime);
    assert!(runtime.inventory_cleanup.task.is_none());
    assert!(runtime.inventory_cleanup.retry_delay.is_none());
    assert!(
        runtime.inventory_cleanup.next_check.is_some(),
        "completed has_more=false must schedule an idle recheck"
    );
    fixture.poll(runtime);
    assert!(
        runtime.inventory_cleanup.task.is_none(),
        "idle poll cannot start another batch"
    );
    fixture.assert_valid_rootless_catalog();
}

#[test]
fn bounded_cleanup_production_poll_runs_without_roots_and_rechecks_after_restart() {
    let fixture = CleanupDebtFixture::new();
    fixture.seed_orphans();
    assert_eq!(fixture.debt(), (4_097, 0, 0));
    let mut runtime = fixture.runtime();
    fixture.poll(&mut runtime);
    await_worker(&mut runtime.inventory_cleanup);
    assert_eq!(
        fixture.debt(),
        (1, 0, 0),
        "one production poll deletes only 4,096 entries"
    );
    runtime
        .inventory_cleanup
        .reap_finished(Instant::now())
        .expect("observe the first real batch");
    assert!(
        runtime.inventory_cleanup.next_check.is_none(),
        "has_more=true remains due"
    );
    stop_without_draining(&mut runtime);
    drop(runtime);
    assert_eq!(
        fixture.debt(),
        (1, 0, 0),
        "stop leaves the persisted remainder for another epoch"
    );
    fixture.assert_valid_rootless_catalog();

    let mut restarted = fixture.runtime();
    fixture.poll(&mut restarted);
    await_worker(&mut restarted.inventory_cleanup);
    assert_eq!(
        fixture.debt(),
        (0, 0, 0),
        "restart discovers debt without an in-memory notification"
    );
    assert_idle(&fixture, &mut restarted);
    stop_without_draining(&mut restarted);
}

#[test]
fn bounded_cleanup_production_poll_reclaims_empty_retired_metadata_in_bounded_batches() {
    let fixture = CleanupDebtFixture::new();
    fixture.seed_empty_retired_spools();
    assert_eq!(fixture.debt(), (0, 129, 129));
    let mut runtime = fixture.runtime();
    fixture.poll(&mut runtime);
    await_worker(&mut runtime.inventory_cleanup);
    assert_eq!(
        fixture.debt(),
        (0, 1, 1),
        "directory and header batches are each limited to 128"
    );
    fixture.assert_valid_rootless_catalog();
    fixture.poll(&mut runtime);
    await_worker(&mut runtime.inventory_cleanup);
    assert_eq!(
        fixture.debt(),
        (0, 0, 0),
        "metadata-only has_more schedules its next batch"
    );
    assert_idle(&fixture, &mut runtime);
    stop_without_draining(&mut runtime);
}
