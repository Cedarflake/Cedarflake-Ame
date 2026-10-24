use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;

struct HeldPreparation {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    dropped: Arc<AtomicUsize>,
    control: Arc<OrganizationBatchControl>,
}

impl Drop for HeldPreparation {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::AcqRel);
    }
}

impl OrganizationBatchRepository for HeldPreparation {
    fn append_ids(&mut self, _: &[String]) -> Result<(), ScanError> {
        self.control.check()
    }

    fn prepare(&mut self) -> Result<OrganizationBatchSummary, ScanError> {
        self.entered.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
        self.control.check()?;
        panic!("The held preparation must be cancelled before release");
    }

    fn apply(
        &mut self,
        _: &str,
        _: &OrganizationBatchAction,
    ) -> Result<OrganizationWriteReceipt, ScanError> {
        panic!("The cancelled session cannot apply a write");
    }

    fn refresh(&mut self) -> Result<OrganizationBatchSummary, ScanError> {
        self.prepare()
    }
}

#[test]
fn close_cancels_inflight_work_before_waiting_and_releases_the_session_capacity() {
    let control = Arc::new(OrganizationBatchControl::new());
    let dropped = Arc::new(AtomicUsize::new(0));
    let (entered, waiting) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let session = Arc::new(OrganizationBatchSession {
        inner: Mutex::new(Some(ActiveSelection {
            repository: Box::new(HeldPreparation {
                entered,
                release: released,
                dropped: Arc::clone(&dropped),
                control: Arc::clone(&control),
            }),
            _permit: SelectionPermit::acquire().unwrap(),
        })),
        control: Arc::clone(&control),
    });
    let second = SelectionPermit::acquire().unwrap();
    assert_eq!(
        SelectionPermit::acquire().err().unwrap().code,
        "organization_selection_busy"
    );
    let worker = Arc::clone(&session);
    let worker = std::thread::spawn(move || worker.prepare());
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let closer = Arc::clone(&session);
    let closer = std::thread::spawn(move || closer.close());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !control.cancelled.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "Close must signal cancellation while preparation holds the repository"
        );
        std::thread::yield_now();
    }
    release.send(()).unwrap();
    assert_eq!(
        worker.join().unwrap().unwrap_err().code,
        "organization_selection_cancelled"
    );
    closer.join().unwrap().unwrap();
    assert_eq!(dropped.load(Ordering::Acquire), 1);
    assert_eq!(session.progress().phase, OrganizationBatchPhase::Cancelled);
    assert!(session.append_ids(vec!["late".into()]).is_err());
    session.close().unwrap();
    assert_eq!(dropped.load(Ordering::Acquire), 1);
    let replacement = SelectionPermit::acquire().unwrap();
    drop(replacement);
    drop(second);
    drop(session);
    assert_eq!(OPEN_SELECTIONS.load(Ordering::Acquire), 0);
}
