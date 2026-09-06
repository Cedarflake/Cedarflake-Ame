use std::fs;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tempfile::tempdir;

use crate::domain::{
    AssetLocationView, FileIdentityEvidence, JournalFileReference, JournalIdentifier, JournalUsn,
    LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy,
    LibraryChangeScope, LibraryRecoveryAuthorityReason, LibraryRootGeneration,
    PersistentJournalBaselineStartRequest, PersistentJournalVolumeIdentity, PreviewStatus,
    ScanRequest,
};
use crate::ports::{CatalogRepository, LibraryChangeQueue, PersistentJournalRepository};

use super::super::{SqliteCatalog, SqliteWriteAdmission, sqlite_write_priority};
use super::LibraryChangeLane;

const IDENTITY_COUNT: u64 = 50_000;
const ROOT: &str = "publication-benchmark-root";
const SCAN: &str = "publication-benchmark-first-import";

#[test]
#[ignore = "explicit bounded synthetic catalog publication performance evidence"]
fn benchmark_fifty_thousand_identity_publication_with_noop_polls() {
    let storage = tempdir().expect("owned publication benchmark storage");
    let source = storage.path().join("empty-source");
    fs::create_dir(&source).expect("empty synthetic source namespace");
    let path = storage.path().join("catalog.sqlite3");
    let fixture_started = Instant::now();
    let mut catalog = SqliteCatalog::open(path.clone()).expect("synthetic catalog");
    let root_path = source.to_string_lossy().into_owned();
    let request = ScanRequest {
        scan_id: SCAN.to_owned(),
        root_path: root_path.clone(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    catalog
        .begin_scan(&request, ROOT, &root_path)
        .expect("begin first import");
    catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: SCAN.to_owned(),
                root_id: ROOT.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                authority_reason: LibraryRecoveryAuthorityReason::FirstImportBoundary,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "synthetic-publication-volume".to_owned(),
                    volume_serial: 7,
                },
                root_file_reference: JournalFileReference::V2([1; 8]),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                opening_next_usn: JournalUsn::new(10).expect("opening boundary"),
                protocol_version: 1,
                contract_version: 1,
                authorized_unix_ms: 1_000,
            },
            LibraryChangeQueuePolicy::default(),
        )
        .expect("real first-import capture authority");
    for index in 0..IDENTITY_COUNT {
        catalog
            .stage_location(SCAN, ROOT, &location(index, &root_path))
            .expect("stage synthetic identity through catalog port");
    }
    catalog
        .flush_pending_locations()
        .expect("flush bounded staging batches");
    catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: ROOT.to_owned(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: "new-after-enumeration.png".to_owned(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            LibraryChangeQueuePolicy::default(),
        )
        .expect("retain real pending P0 intent");
    let staged: (i64, i64, i64, i64) = catalog.connection.query_row(
        "SELECT COUNT(*), COUNT(DISTINCT file_identity_value), COUNT(DISTINCT source_generation), MIN(source_generation) FROM asset_locations WHERE scan_id = ?1",
        [SCAN], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).expect("complete unique staged roster");
    assert_eq!(
        (
            u64::try_from(staged.0).expect("non-negative staged count"),
            u64::try_from(staged.1).expect("non-negative distinct identity count"),
            u64::try_from(staged.2).expect("non-negative distinct generation count"),
        ),
        (IDENTITY_COUNT, IDENTITY_COUNT, IDENTITY_COUNT)
    );
    assert!(
        staged.3 > 0,
        "stage_location must allocate real positive source generations"
    );
    assert_eq!(active_count(&catalog).expect("unpublished root"), 0);
    let p0_before = pending_p0(&catalog);
    assert_eq!(p0_before.len(), 1);
    let poller = SqliteCatalog::open(path.clone()).expect("independent production poll connection");
    let admission = Arc::clone(&catalog.write_admission);
    let initial_epoch = admission.completed_write_epoch();
    let initial_revision = catalog_revision(&catalog);
    let fixture_ms = fixture_started.elapsed().as_millis();
    let mut polling = PublicationPolling::start(poller, Arc::clone(&admission));
    let publication_started = Instant::now();
    // Catalog-only benchmark: no filesystem validation proof is invented for synthetic rows.
    let published = catalog.publish_scan(SCAN, ROOT, IDENTITY_COUNT, 0);
    let publication_ms = publication_started.elapsed().as_millis();
    let evidence = polling.finish();
    published.expect("one publication call must commit despite overlapping empty polls");
    let evidence = evidence.expect("joined production no-op poller");
    assert!(evidence.polls >= 3);
    assert!(
        evidence.overlapping >= 3,
        "the workload must overlap real publication, not merely poll before it"
    );
    assert_eq!(
        admission.completed_write_epoch(),
        initial_epoch + 1,
        "only the publication writer may acquire and retire a permit"
    );
    assert_eq!(catalog_revision(&catalog), initial_revision + 1);
    assert_eq!(
        active_count(&catalog).expect("published root"),
        IDENTITY_COUNT
    );
    assert_eq!(
        pending_p0(&catalog),
        p0_before,
        "first publication cannot consume newer P0"
    );
    drop(catalog);
    let reopened =
        SqliteCatalog::open(path.clone()).expect("reopen and prove complete persisted publication");
    assert_eq!(
        active_count(&reopened).expect("reopened root"),
        IDENTITY_COUNT
    );
    assert_eq!(pending_p0(&reopened), p0_before);
    let completed: (i64, i64) = reopened.connection.query_row(
        "SELECT COUNT(*), SUM(asset_count) FROM scan_runs WHERE id = ?1 AND status = 'completed'",
        [SCAN], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("single durable completion");
    assert_eq!(
        (
            u64::try_from(completed.0).expect("non-negative completed scan count"),
            u64::try_from(completed.1).expect("non-negative completed asset count"),
        ),
        (1, IDENTITY_COUNT),
    );
    drop(reopened);
    assert_eq!(
        fs::read_dir(&source)
            .expect("unchanged empty source")
            .count(),
        0
    );
    let catalog_bytes = [
        path.clone(),
        path.with_file_name("catalog.sqlite3-wal"),
        path.with_file_name("catalog.sqlite3-shm"),
    ]
    .into_iter()
    .filter(|file| file.exists())
    .map(|file| fs::metadata(file).expect("catalog bytes").len())
    .sum::<u64>();
    assert!(catalog_bytes > 0);
    println!(
        "AME_PUBLICATION_BENCHMARK identities=50000 staged=50000 published=50000 publication_calls=1 commits=1 polls={} overlapping_polls={} partial_observations=0 p0_pending=1 source_entries=0 fixture_ms={fixture_ms} publication_ms={publication_ms} catalog_bytes={catalog_bytes}",
        evidence.polls, evidence.overlapping
    );
    storage
        .close()
        .expect("release only owned generated catalog storage");
}

fn location(index: u64, root_path: &str) -> AssetLocationView {
    let name = format!("synthetic-{index:05}.png");
    AssetLocationView {
        asset_id: format!("asset-{index:05}"),
        location_id: format!("location-{index:05}"),
        root_id: ROOT.to_owned(),
        scan_id: SCAN.to_owned(),
        absolute_path: format!("{root_path}/{name}"),
        display_path: name.clone(),
        relative_path: name,
        preview_path: String::new(),
        file_size: 1,
        created_unix_ms: None,
        modified_unix_ms: 1_000,
        file_identity: Some(FileIdentityEvidence {
            scheme: "synthetic-publication-identity-v1".to_owned(),
            value: format!("{index:032x}"),
        }),
        source_revision: None,
        source_generation: 0,
        width: 128,
        height: 96,
        preview_status: PreviewStatus::Pending,
        preview_issue_code: None,
        preview_issue_message: None,
        metadata_engine_id: "synthetic-publication-metadata".to_owned(),
        metadata_engine_version: "1".to_owned(),
        capture_time: None,
    }
}

fn active_count(catalog: &SqliteCatalog) -> Result<u64, String> {
    catalog.connection.query_row(
        "SELECT COUNT(*) FROM asset_locations AS location JOIN library_roots AS root
         ON root.id = location.root_id AND root.active_scan_id = location.scan_id WHERE root.id = ?1",
        [ROOT], |row| row.get::<_, i64>(0),
    ).map_err(|error| error.to_string())
        .and_then(|count| u64::try_from(count).map_err(|error| error.to_string()))
}

fn catalog_revision(catalog: &SqliteCatalog) -> u64 {
    let revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("read-only catalog revision");
    u64::try_from(revision).expect("non-negative catalog revision")
}

fn pending_p0(catalog: &SqliteCatalog) -> Vec<(i64, String, String)> {
    let mut statement = catalog.connection.prepare(
        "SELECT change.id, change.status, change.relative_path FROM library_change_queue AS change
         JOIN library_change_queue_lanes AS lane ON lane.change_id = change.id
         WHERE change.root_id = ?1 AND lane.lane = 'p0_live' ORDER BY change.id"
    ).expect("P0 ownership query");
    let rows = statement
        .query_map([ROOT], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("P0 rows");
    rows.collect::<Result<Vec<_>, _>>()
        .expect("exact retained P0 state")
}

#[derive(Default)]
struct PollEvidence {
    polls: u64,
    overlapping: u64,
}

struct PublicationPolling {
    stop: mpsc::SyncSender<()>,
    worker: Option<JoinHandle<Result<PollEvidence, String>>>,
}

impl PublicationPolling {
    fn start(mut catalog: SqliteCatalog, admission: Arc<SqliteWriteAdmission>) -> Self {
        let (stop, cancelled) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let mut evidence = PollEvidence::default();
            loop {
                let before = publication_epoch(&admission);
                let completed = catalog
                    .finalize_ready_first_import_journal_baseline(3_000)
                    .map_err(|error| format!("no-op completion failed: {}", error.code))?;
                if completed {
                    return Err(
                        "unfinished first import unexpectedly finalized its journal baseline"
                            .to_owned(),
                    );
                }
                evidence.polls += 1;
                let after = publication_epoch(&admission);
                if before.is_some() && before == after {
                    evidence.overlapping += 1;
                }
                let count = active_count(&catalog)?;
                if count != 0 && count != IDENTITY_COUNT {
                    return Err(format!("partial publication visible: {count}"));
                }
                match cancelled.recv_timeout(Duration::from_millis(10)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    _ => return Ok(evidence),
                }
            }
        });
        Self {
            stop,
            worker: Some(worker),
        }
    }

    fn finish(&mut self) -> Result<PollEvidence, String> {
        let _ = self.stop.try_send(());
        self.worker
            .take()
            .expect("single poll owner")
            .join()
            .map_err(|_| "production poll worker panicked".to_owned())?
    }
}

impl Drop for PublicationPolling {
    fn drop(&mut self) {
        if self.worker.is_some() {
            if let Err(error) = self.finish() {
                eprintln!("publication benchmark cleanup: {error}");
            }
        }
    }
}

fn publication_epoch(admission: &SqliteWriteAdmission) -> Option<u64> {
    let state = admission
        .state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    (state.is_active
        && state.active_priority == sqlite_write_priority(LibraryChangeLane::Recovery)
        && state.active_preempt.is_some())
    .then_some(state.completed_write_epoch)
}
