use crate::domain::JournalFileReference;
use crate::ports::MAX_IDENTITY_JOURNAL_PAGE_RECORDS;

use super::*;

fn record(usn: i64, file: u64, parent: u64) -> IdentityJournalRecord {
    IdentityJournalRecord {
        file_reference: JournalFileReference::from_bytes(&file.to_le_bytes()).unwrap(),
        parent_reference: JournalFileReference::from_bytes(&parent.to_le_bytes()).unwrap(),
        usn,
        is_directory: false,
        is_deleted: false,
    }
}

#[test]
fn repeated_volume_changes_do_not_hide_a_later_root_change_behind_the_old_record_ceiling() {
    let mut records: Vec<_> = (10..522).map(|usn| record(usn, 100, 101)).collect();
    records.push(record(522, 2, 3));
    let page = bounded_prefix(records, 600, MAX_IDENTITY_JOURNAL_PAGE_RECORDS);
    assert_eq!(page.records.len(), 513);
    assert_eq!(page.records.last().unwrap().usn, 522);
    assert_eq!(page.covered_until_usn, 600);
}

#[test]
fn identity_capacity_leaves_the_first_unresolved_record_for_the_next_read() {
    let records = (0..129)
        .map(|index| record(index + 10, index as u64 * 2, index as u64 * 2 + 1))
        .collect();
    let page = bounded_prefix(records, 200, MAX_IDENTITY_JOURNAL_PAGE_RECORDS);
    assert_eq!(page.records.len(), 128);
    assert_eq!(page.covered_until_usn, 138);
    assert_eq!(page.records.last().unwrap().usn, 137);
}

#[test]
fn repeated_parent_and_file_reference_consumes_one_identity_slot() {
    let records = (0..257)
        .map(|index| record(index + 10, index as u64, index as u64))
        .collect();
    let page = bounded_prefix(records, 300, MAX_IDENTITY_JOURNAL_PAGE_RECORDS);
    assert_eq!(page.records.len(), 256);
    assert_eq!(page.covered_until_usn, 266);
}

#[test]
fn explicit_record_limit_preserves_coverage_and_an_empty_read_preserves_the_boundary() {
    let records = (10..90).map(|usn| record(usn, 2, 3)).collect();
    let page = bounded_prefix(records, 100, 64);
    assert_eq!(page.records.len(), 64);
    assert_eq!(page.covered_until_usn, 74);
    let empty = bounded_prefix(Vec::new(), 10, MAX_IDENTITY_JOURNAL_PAGE_RECORDS);
    assert!(empty.records.is_empty());
    assert_eq!(empty.covered_until_usn, 10);
}
