use std::cell::Cell;
use std::sync::atomic::AtomicBool;

use super::*;

struct HeldSource<'a> {
    fingerprint: ContentFingerprint,
    gate: &'a BackgroundAnalysisGate,
    released: &'a Cell<bool>,
}

impl FingerprintReadLease for HeldSource<'_> {
    fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        Err(ScanError::new("source_changed", "The source changed"))
    }
}

impl Drop for HeldSource<'_> {
    fn drop(&mut self) {
        assert!(self.gate.state.lock().unwrap().active.is_some());
        self.released.set(true);
    }
}

#[test]
fn source_retirement_precedes_next_admission_after_publication_or_failure() {
    let gate = BackgroundAnalysisGate::default();
    let released = Cell::new(false);
    let cancelled = AtomicBool::new(false);
    let lease = AdmittedFingerprint {
        source: HeldSource {
            fingerprint: ContentFingerprint {
                digest: "digest".into(),
                byte_length: 7,
            },
            gate: &gate,
            released: &released,
        },
        _admission: gate.acquire(&cancelled).unwrap().unwrap(),
    };
    assert_eq!(lease.fingerprint().byte_length, 7);
    assert_eq!(lease.revalidate().unwrap_err().code, "source_changed");
    assert!(!released.get());
    assert!(gate.state.lock().unwrap().active.is_some());
    drop(lease);
    assert!(released.get());
    assert!(gate.acquire(&cancelled).unwrap().is_some());
}
