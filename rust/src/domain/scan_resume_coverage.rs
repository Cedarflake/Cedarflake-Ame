use super::{
    JournalFileReference, JournalIdentifier, JournalUsn, PersistentJournalBaseline,
    PersistentJournalBaselinePhase, PersistentJournalVolumeIdentity, ScanError,
};

#[derive(Debug)]
pub(crate) struct ScanResumeCoverage {
    baseline: PersistentJournalBaseline,
}

impl ScanResumeCoverage {
    pub(crate) fn verify(
        baseline: PersistentJournalBaseline,
        volume: &PersistentJournalVolumeIdentity,
        root_file_reference: &JournalFileReference,
        journal_id: JournalIdentifier,
        first_usn: JournalUsn,
        next_usn: JournalUsn,
    ) -> Result<Option<Self>, ScanError> {
        baseline.validate()?;
        if baseline.phase != PersistentJournalBaselinePhase::Inventory
            || baseline.volume != *volume
            || baseline.root_file_reference != *root_file_reference
            || baseline.journal_id != journal_id
            || baseline.opening_next_usn < first_usn
            || baseline.opening_next_usn > next_usn
        {
            return Ok(None);
        }
        Ok(Some(Self { baseline }))
    }

    pub(crate) fn baseline(&self) -> &PersistentJournalBaseline {
        &self.baseline
    }
}

#[cfg(test)]
mod tests;
