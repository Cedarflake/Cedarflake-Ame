use crate::journal_broker::read_interval::JournalReadInterval;
use crate::journal_broker::wire::MAX_SHARED_HANDOFFS;

use super::{
    BrokerFailure, BrokerFailureCode, ExistingJournalMetadata, JournalRange,
    SharedBackendJournalPage, map_backend_failure,
};

#[cfg(test)]
mod tests;

pub(super) fn validate_shared_backend_page(
    before: ExistingJournalMetadata,
    range: JournalRange,
    page: &SharedBackendJournalPage,
    expected_roots: usize,
) -> Result<(), BrokerFailure> {
    let interval = JournalReadInterval::new(range.start_usn, range.end_usn)
        .ok_or_else(|| BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous))?;
    if page.candidates_by_root.len() != expected_roots
        || page.rename_barriers_by_root.len() != expected_roots
        || page
            .rename_barriers_by_root
            .iter()
            .any(|barrier| barrier.is_some_and(|usn| usn < range.start_usn || usn >= range.end_usn))
        || page.handoffs.len() > MAX_SHARED_HANDOFFS
        || (page.candidates_by_root.iter().any(Result::is_ok) != page.proof.is_some())
        || (page.proof.is_none() && !page.handoffs.is_empty())
        || page.handoffs.iter().any(|handoff| {
            (handoff.previous_root_index.is_some() == handoff.previous_pending_index.is_some())
                || handoff
                    .previous_root_index
                    .is_some_and(|index| index >= expected_roots)
                || handoff.current_root_index >= expected_roots
                || handoff.previous_root_index == Some(handoff.current_root_index)
                || handoff
                    .previous_root_index
                    .is_some_and(|index| page.candidates_by_root[index].is_err())
                || page.candidates_by_root[handoff.current_root_index].is_err()
                || !matches!(handoff.file_reference.len(), 8 | 16)
                || handoff.usn < range.start_usn
                || page
                    .proof
                    .is_none_or(|proof| handoff.usn >= proof.covered_until_usn)
        })
        || page.pending_renames.iter().any(|pending| {
            pending.root_index >= expected_roots
                || page.candidates_by_root[pending.root_index].is_err()
        })
        || page
            .candidates_by_root
            .iter()
            .filter_map(|result| result.as_ref().ok())
            .any(|candidates| {
                let mut last = None;
                candidates.iter().any(|candidate| {
                    let invalid = candidate.usn < range.start_usn
                        || page
                            .proof
                            .is_none_or(|proof| candidate.usn >= proof.covered_until_usn)
                        || last.is_some_and(|previous| candidate.usn <= previous);
                    last = Some(candidate.usn);
                    invalid
                })
            })
    {
        return Err(BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous));
    }
    if let Some(proof) = page.proof {
        let after = proof.after.validate().map_err(map_backend_failure)?;
        if after.journal_id != before.journal_id
            || after.first_usn > range.start_usn
            || after.next_usn < range.end_usn
            || !interval.admits_coverage(proof.covered_until_usn, proof.is_complete)
        {
            return Err(BrokerFailure::new(BrokerFailureCode::JournalDiscontinuous));
        }
    }
    Ok(())
}
