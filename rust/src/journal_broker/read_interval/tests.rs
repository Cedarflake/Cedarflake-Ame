use super::JournalReadInterval;

#[test]
fn empty_coverage_requires_an_exact_complete_receipt() {
    let interval = JournalReadInterval::new(20, 20).expect("valid empty closing interval");
    assert!(interval.admits_coverage(20, true));
    assert!(!interval.admits_coverage(20, false));
    assert!(!interval.admits_coverage(19, true));
    assert!(!interval.admits_coverage(21, true));
}

#[test]
fn nonempty_reads_still_require_progress_and_exact_completion() {
    let interval = JournalReadInterval::new(10, 20).expect("valid journal interval");
    assert!(!interval.admits_coverage(10, false));
    assert!(!interval.admits_coverage(10, true));
    assert!(interval.admits_coverage(15, false));
    assert!(!interval.admits_coverage(15, true));
    assert!(interval.admits_coverage(20, true));
    assert!(!interval.admits_coverage(20, false));
    assert!(!interval.admits_coverage(21, true));
}

#[test]
fn reversed_or_negative_bounds_have_no_read_authority() {
    assert!(JournalReadInterval::new(-1, 0).is_none());
    assert!(JournalReadInterval::new(20, 19).is_none());
    let origin = JournalReadInterval::new(0, 0).expect("empty origin");
    assert!(origin.admits_coverage(0, true));
}
