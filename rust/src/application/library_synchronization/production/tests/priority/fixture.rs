use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) struct PriorityFixture<G> {
    pub(super) production: ProductionSynchronization,
    source_gate: Option<G>,
    pub(super) writer_stop: Arc<AtomicBool>,
    pub(super) writer: Option<JoinHandle<Result<(), ScanError>>>,
    runtime_drained: bool,
    runtime_stop_deadline: Option<Instant>,
}

impl<G> PriorityFixture<G> {
    pub(super) fn new(production: ProductionSynchronization, source_gate: G) -> Self {
        Self {
            production,
            source_gate: Some(source_gate),
            writer_stop: Arc::new(AtomicBool::new(false)),
            writer: None,
            runtime_drained: false,
            runtime_stop_deadline: None,
        }
    }

    pub(super) fn source_gate(&self) -> &G {
        self.source_gate
            .as_ref()
            .expect("active priority source gate")
    }

    pub(super) fn finish(&mut self) -> Result<(), String> {
        self.writer_stop.store(true, Ordering::Release);
        // The existing gate's Drop unblocks enumeration before runtime drain can join it.
        // Attempt every phase before returning an error, including during assertion unwind.
        let gate = catch_unwind(AssertUnwindSafe(|| drop(self.source_gate.take())));
        let runtime = if self.runtime_drained {
            Ok(Ok(()))
        } else {
            let deadline = *self
                .runtime_stop_deadline
                .get_or_insert_with(|| Instant::now() + Duration::from_secs(2));
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                self.production.finish_stopping_until(deadline)
            }));
            self.runtime_drained = matches!(&outcome, Ok(Ok(())));
            outcome
        };
        let writer = self.writer.take().map(JoinHandle::join);

        gate.map_err(|_| "priority source gate cleanup panicked".to_owned())?;
        runtime
            .map_err(|_| "priority runtime cleanup panicked".to_owned())?
            .map_err(|error| format!("priority runtime cleanup failed: {error:?}"))?;
        if let Some(result) = writer {
            result
                .map_err(|_| "priority writer panicked".to_owned())?
                .map_err(|error| format!("priority writer failed: {error:?}"))?;
        }
        Ok(())
    }
}

impl<G> Drop for PriorityFixture<G> {
    fn drop(&mut self) {
        if self.finish().is_err() && !self.runtime_drained {
            // Retry one recoverable drain panic without granting a new shutdown deadline.
            let _ = self.finish();
        }
    }
}

#[test]
fn priority_fixture_unwind_releases_gate_stops_runtime_and_joins_writer() {
    let directory = tempfile::tempdir().expect("priority cleanup directory");
    std::fs::write(directory.path().join("entry.jpg"), b"metadata only").expect("entry");
    let root_path = directory.path().to_string_lossy().into_owned();
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let writer_finished = Arc::new(AtomicBool::new(false));
    let writer_stop = Arc::new(AtomicBool::new(false));
    crate::adapters::reset_source_enumeration_instrumentation(&root_path);
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let source_gate = crate::adapters::gate_source_enumeration(&root_path);
        let mut fixture = PriorityFixture::new(
            runtime_with_close_probe(Arc::clone(&close_count)),
            source_gate,
        );
        fixture.writer_stop = Arc::clone(&writer_stop);
        let stopped = Arc::clone(&writer_stop);
        let closed = Arc::clone(&close_count);
        let finished = Arc::clone(&writer_finished);
        let worker_root = root_path.clone();
        fixture.writer = Some(thread::spawn(move || {
            let discovery = crate::adapters::PublicationGuardedFileDiscovery::
                new_metadata_inventory_source_guard(&worker_root, None)
                .expect("guarded priority cleanup source");
            discovery
                .streaming_metadata_inventory_entries_in_directory("")
                .expect("priority cleanup entries")
                .next()
                .expect("priority cleanup entry")
                .expect("read priority cleanup entry");
            assert!(
                stopped.load(Ordering::Acquire),
                "stop intent precedes gate release"
            );
            let deadline = Instant::now() + Duration::from_secs(5);
            while closed.load(Ordering::Acquire) == 0 {
                assert!(
                    Instant::now() < deadline,
                    "runtime must stop before writer join"
                );
                thread::yield_now();
            }
            finished.store(true, Ordering::Release);
            Ok(())
        }));
        assert!(
            fixture
                .source_gate()
                .wait_until_blocked(Duration::from_secs(5))
        );
        assert_eq!(crate::adapters::source_entry_read_count(&root_path), 0);
        panic!("injected priority assertion");
    }));

    assert_eq!(
        panic.expect_err("injected panic").downcast_ref::<&str>(),
        Some(&"injected priority assertion")
    );
    assert!(writer_stop.load(Ordering::Acquire));
    assert!(writer_finished.load(Ordering::Acquire));
    assert_eq!(close_count.load(Ordering::Acquire), 1);
    assert_eq!(crate::adapters::source_entry_read_count(&root_path), 1);
    std::fs::rename(
        directory.path().join("entry.jpg"),
        directory.path().join("released.jpg"),
    )
    .expect("source handles released");
}

#[test]
fn priority_fixture_finish_reports_writer_failure_after_cleanup() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut fixture = PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
    fixture.writer = Some(thread::spawn(|| {
        Err(ScanError::new(
            "priority_writer_injected_failure",
            "controlled writer failure",
        ))
    }));

    let failure = fixture
        .finish()
        .expect_err("writer failure must remain visible");
    assert!(failure.contains("priority_writer_injected_failure"));
    assert!(fixture.writer_stop.load(Ordering::Acquire));
    assert!(fixture.writer.is_none());
    assert!(fixture.source_gate.is_none());
    fixture.finish().expect("idempotent cleanup");
    drop(fixture);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_unwind_preserves_assertion_when_writer_panics() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let mut fixture =
            PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
        fixture.writer = Some(thread::spawn(|| panic!("injected writer panic")));
        panic!("original priority assertion");
    }));

    assert_eq!(
        panic.expect_err("original panic").downcast_ref::<&str>(),
        Some(&"original priority assertion")
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_retries_runtime_panic_with_the_original_deadline() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut fixture = PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
    fixture
        .production
        .inject_drain_panic(DrainPanicPoint::RequestStop, Some(1));

    let failure = fixture
        .finish()
        .expect_err("runtime panic must remain visible");
    assert!(failure.contains("runtime cleanup panicked"));
    assert!(!fixture.runtime_drained);
    assert_eq!(close_count.load(Ordering::Acquire), 0);
    let deadline = fixture.runtime_stop_deadline.expect("original deadline");
    fixture.finish().expect("retry the same runtime owner");
    assert!(fixture.runtime_drained);
    assert_eq!(fixture.runtime_stop_deadline, Some(deadline));
    drop(fixture);
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}

#[test]
fn priority_fixture_failure_diagnostics_match_the_current_catalog_schema() {
    let directory = tempfile::tempdir().expect("priority diagnostics directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    drop(SqliteCatalog::open(catalog_path.clone()).expect("priority diagnostics catalog"));
    let connection = rusqlite::Connection::open(catalog_path).expect("priority diagnostics reader");

    assert_eq!(
        priority_queue_progress(&connection).expect("queue diagnostics"),
        ""
    );
    assert_eq!(
        priority_inventory_progress(&connection, "unregistered-root")
            .expect("inventory diagnostics"),
        None
    );
}

#[test]
fn priority_fixture_unwind_retries_one_runtime_panic_without_replacing_the_assertion() {
    let close_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        let mut fixture =
            PriorityFixture::new(runtime_with_close_probe(Arc::clone(&close_count)), ());
        fixture
            .production
            .inject_drain_panic(DrainPanicPoint::RequestStop, Some(1));
        panic!("original priority assertion");
    }));

    assert_eq!(
        panic.expect_err("original panic").downcast_ref::<&str>(),
        Some(&"original priority assertion")
    );
    assert_eq!(close_count.load(Ordering::Acquire), 1);
}
