use std::collections::BTreeSet;

use crate::ports::{IdentityJournalPage, IdentityJournalRecord, MAX_IDENTITY_JOURNAL_REFERENCES};

pub(super) fn bounded_prefix(
    mut records: Vec<IdentityJournalRecord>,
    covered_until_usn: i64,
    max_records: u32,
) -> IdentityJournalPage {
    let mut references = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let additional = BTreeSet::from([
            record.file_reference.to_canonical_text(),
            record.parent_reference.to_canonical_text(),
        ]);
        if index >= max_records as usize
            || references.len() + additional.difference(&references).count()
                > MAX_IDENTITY_JOURNAL_REFERENCES
        {
            // Coverage stops before the first unprocessed record, including its identities.
            let covered_until_usn = record.usn;
            records.truncate(index);
            return IdentityJournalPage {
                records,
                covered_until_usn,
            };
        }
        references.extend(additional);
    }
    IdentityJournalPage {
        records,
        covered_until_usn,
    }
}

#[cfg(test)]
mod tests;
