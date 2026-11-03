use std::time::Duration;

use crate::domain::{
    DiscoveredFile, ExpectedFileState, RetainedScanIssue, ScanError, ScanPublicationReceipt,
    StagedValidationOutcome,
};

use super::scan_staging::ScanStagingRepository;
use super::{LibraryChangeQueue, RootConsolidationPlan, ScanPublicationControl};

pub(crate) trait ScanPublicationRepository:
    ScanStagingRepository + LibraryChangeQueue
{
    // Rosters and proofs belong to this repository session; publication rechecks their staged payload.
    type StagingRoster;
    type StagingProof;
    type RejectedRoster;

    fn capture_staging_validation(
        &mut self,
        scan_id: &str,
    ) -> Result<Self::StagingRoster, ScanError>;
    fn staging_validation_total_items(&self, roster: &Self::StagingRoster) -> u64;
    fn record_staging_validation(
        &self,
        roster: &Self::StagingRoster,
        location_id: &str,
        outcome: StagedValidationOutcome,
    ) -> Result<(), ScanError>;
    fn load_staging_validation_window(
        &self,
        roster: &Self::StagingRoster,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, String, ExpectedFileState)>, ScanError>;
    fn staging_superseded_by_live(
        &self,
        roster: &Self::StagingRoster,
        root_id: &str,
        location_id: &str,
    ) -> Result<bool, ScanError>;
    fn finish_staging_validation(
        &self,
        roster: Self::StagingRoster,
        validated_items: u64,
    ) -> Result<Self::StagingProof, ScanError>;
    fn publish_validated_scan(
        &mut self,
        proof: &Self::StagingProof,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
        consolidation: Option<&RootConsolidationPlan>,
        control: &ScanPublicationControl,
    ) -> Result<ScanPublicationReceipt, ScanError>;
    fn create_rejected_input_roster(&self) -> Result<Self::RejectedRoster, ScanError>;
    fn record_rejected_input(
        &self,
        roster: &Self::RejectedRoster,
        file: &DiscoveredFile,
    ) -> Result<(), ScanError>;
    fn load_rejected_input_window(
        &self,
        roster: &Self::RejectedRoster,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, ExpectedFileState)>, ScanError>;
    fn load_retained_scan_issue_window(
        &self,
        scan_id: &str,
        codes: &[&str],
        after_id: i64,
        limit: u32,
    ) -> Result<Vec<RetainedScanIssue>, ScanError>;
    fn retained_scan_has_unclassified_issues(
        &self,
        scan_id: &str,
        classified_codes: &[&str],
    ) -> Result<bool, ScanError>;
    fn fail_scan_publication_namespace(
        &mut self,
        scan_id: &str,
        root_id: &str,
        failure: &ScanError,
        issue_count: u64,
    ) -> Result<(), ScanError>;
    fn preserve_authoritative_retry_evidence(
        &mut self,
        scan_id: &str,
        root_id: &str,
        retry_relative_paths: &[String],
    ) -> Result<u64, ScanError>;
    fn publish_authoritative_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
        retry_relative_paths: &[String],
    ) -> Result<(), ScanError>;
    fn completed_write_epoch(&self) -> u64;
    fn wait_for_completed_write_after(&self, observed_epoch: u64, timeout: Duration) -> u64;
}
