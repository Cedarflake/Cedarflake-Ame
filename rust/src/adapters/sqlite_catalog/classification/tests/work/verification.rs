use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::application::classification_verification::verify;
use crate::domain::exact_duplicates::{ContentFingerprint, FingerprintCandidate};
use crate::ports::exact_duplicates::{ExactContentReader, FingerprintReadLease};

use super::*;

#[derive(Clone, Copy)]
enum ReadEffect {
    None,
    Cancel,
    PauseDuringValidation,
    ChangedDuringValidation,
}

struct Reader {
    database: PathBuf,
    reads: Cell<usize>,
    released_after_checkpoints: Rc<RefCell<Vec<i64>>>,
    effect: ReadEffect,
}

struct SourceLease {
    fingerprint: ContentFingerprint,
    database: PathBuf,
    released_after_checkpoints: Rc<RefCell<Vec<i64>>>,
    effect: ReadEffect,
}

impl Reader {
    fn new(directory: &tempfile::TempDir, effect: ReadEffect) -> Self {
        Self {
            database: directory.path().join("catalog.sqlite3"),
            reads: Cell::new(0),
            released_after_checkpoints: Rc::default(),
            effect,
        }
    }
}

impl ExactContentReader for Reader {
    type Lease = SourceLease;

    fn read_fingerprint(
        &self,
        candidate: &FingerprintCandidate,
        cancelled: &AtomicBool,
    ) -> Result<Option<SourceLease>, ScanError> {
        self.reads.set(self.reads.get() + 1);
        if matches!(self.effect, ReadEffect::Cancel) {
            cancelled.store(true, Ordering::Release);
        }
        Ok(Some(SourceLease {
            fingerprint: ContentFingerprint {
                digest: "a".repeat(64),
                byte_length: candidate.location.file_size,
            },
            database: self.database.clone(),
            released_after_checkpoints: self.released_after_checkpoints.clone(),
            effect: self.effect,
        }))
    }
}

impl FingerprintReadLease for SourceLease {
    fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        match self.effect {
            ReadEffect::ChangedDuringValidation => {
                Err(ScanError::new("source_changed", "The source changed"))
            }
            ReadEffect::PauseDuringValidation => {
                let mut catalog = SqliteCatalog::open(self.database.clone())?;
                change(&mut catalog, ClassificationDispatchCommand::Pause);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl Drop for SourceLease {
    fn drop(&mut self) {
        let connection = rusqlite::Connection::open(&self.database).unwrap();
        let checkpoints = connection.query_row(
            "SELECT count(*) FROM classification_work_items WHERE kind = 'source' AND phase = 'done'",
            [], |row| row.get(0),
        ).unwrap();
        self.released_after_checkpoints
            .borrow_mut()
            .push(checkpoints);
    }
}

fn claim_source(
    catalog: &mut SqliteCatalog,
    session: &ClassificationRecordId,
) -> ClassificationWorkLease {
    request(
        catalog,
        session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    catalog
        .claim_classification_work(session, 10)
        .unwrap()
        .unwrap()
}

#[test]
fn singleton_verification_retains_the_source_through_group_and_work_publication() {
    let (directory, mut catalog) = fixture();
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_size = 24 WHERE location_id = 'a'",
            [],
        )
        .unwrap();
    assert!(
        catalog
            .load_fingerprint_candidates("")
            .unwrap()
            .iter()
            .all(|source| source.location.location_id != "a")
    );
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::None);
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).unwrap(),
        ClassificationVerificationResult::Completed
    );
    assert_eq!(reader.reads.get(), 1);
    assert_eq!(*reader.released_after_checkpoints.borrow(), [1]);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM gallery_content_fingerprints"
        ),
        1
    );
    let next = catalog
        .claim_classification_work(&session, 20)
        .unwrap()
        .unwrap();
    assert!(matches!(
        next.target,
        ClassificationWorkTarget::AnalyzeContent { .. }
    ));
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM exact_analysis_outcomes"),
        0
    );
}

#[test]
fn retained_exact_evidence_is_published_without_another_source_read() {
    let (directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::None);
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).unwrap(),
        ClassificationVerificationResult::Completed
    );
    assert_eq!(reader.reads.get(), 0);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM gallery_content_fingerprints"
        ),
        1
    );
    let run: String = catalog
        .connection
        .query_row("SELECT run_id FROM content_fingerprints", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(run, "organization-fixture");
}

#[test]
fn failed_source_checkpoint_rolls_back_fingerprint_group_membership_and_run() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::None);
    catalog.connection.execute_batch("CREATE TRIGGER reject_source_checkpoint BEFORE UPDATE OF phase ON classification_work_items
        WHEN NEW.phase = 'done' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).is_err());
    assert_eq!(*reader.released_after_checkpoints.borrow(), [0]);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM content_fingerprints"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM gallery_content_fingerprints"
        ),
        0
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM exact_analysis_runs"),
        1
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_source_checkpoint")
        .unwrap();
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).unwrap(),
        ClassificationVerificationResult::Completed
    );
    assert_eq!(*reader.released_after_checkpoints.borrow(), [0, 1]);
}

#[test]
fn pause_during_source_validation_fences_the_fingerprint_commit() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::PauseDuringValidation);
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).unwrap(),
        ClassificationVerificationResult::Superseded
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM content_fingerprints"),
        0
    );
    assert_eq!(*reader.released_after_checkpoints.borrow(), [0]);
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Paused
    );
}

#[test]
fn cancelled_hash_releases_the_source_without_publishing_or_finishing_the_claim() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::Cancel);
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false)).unwrap(),
        ClassificationVerificationResult::Cancelled
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM content_fingerprints"),
        0
    );
    assert_eq!(*reader.released_after_checkpoints.borrow(), [0]);
    assert!(catalog.yield_classification_work(&work).unwrap());
}

#[test]
fn changed_source_evidence_is_never_published() {
    let (directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    let work = claim_source(&mut catalog, &session);
    let reader = Reader::new(&directory, ReadEffect::ChangedDuringValidation);
    assert_eq!(
        verify(&mut catalog, &reader, &work, 20, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "source_changed"
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM content_fingerprints"),
        0
    );
    assert_eq!(*reader.released_after_checkpoints.borrow(), [0]);
}
