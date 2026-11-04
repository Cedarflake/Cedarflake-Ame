use std::time::Duration;

use crate::domain::{
    AssetLocationView, DiscoveredFile, ExpectedFileState, FileIdentityEvidence, RetainedScanIssue,
    ScanError, ScanPublicationReceipt, StagedValidationOutcome,
};
use crate::ports::scan_publication::ScanPublicationRepository;
use crate::ports::scan_staging::ScanStagingRepository;
use crate::ports::{RootConsolidationPlan, ScanPublicationControl};

use super::SqliteCatalog;
use super::scan_publication::{
    RejectedInputValidationRoster, StagedValidationRoster, ValidatedStagingProof,
    load_retained_scan_issue_window, retained_scan_has_unclassified_issues,
};

impl ScanStagingRepository for SqliteCatalog {
    fn with_initial_import_writes<T>(
        &mut self,
        scan_id: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        self.with_initial_import_writes(scan_id, operation)
    }

    fn load_retained_import_location(
        &self,
        scan_id: &str,
        location_id: &str,
    ) -> Result<Option<AssetLocationView>, ScanError> {
        self.load_retained_import_location(scan_id, location_id)
    }

    fn complete_directory_with_identity(
        &mut self,
        scan_id: &str,
        relative_directory: &str,
        identity: Option<&FileIdentityEvidence>,
    ) -> Result<(), ScanError> {
        self.complete_directory_with_identity(scan_id, relative_directory, identity)
    }

    fn finish_retained_import_inventory(&mut self, scan_id: &str) -> Result<(), ScanError> {
        self.finish_retained_import_inventory(scan_id)
    }
}

impl ScanPublicationRepository for SqliteCatalog {
    type StagingRoster = StagedValidationRoster;
    type StagingProof = ValidatedStagingProof;
    type RejectedRoster = RejectedInputValidationRoster;

    fn capture_staging_validation(
        &mut self,
        scan_id: &str,
    ) -> Result<Self::StagingRoster, ScanError> {
        StagedValidationRoster::capture(self, scan_id)
    }

    fn staging_validation_total_items(&self, roster: &Self::StagingRoster) -> u64 {
        roster.total_items()
    }

    fn record_staging_validation(
        &self,
        roster: &Self::StagingRoster,
        location_id: &str,
        outcome: StagedValidationOutcome,
    ) -> Result<(), ScanError> {
        roster.record_outcome(self, location_id, outcome)
    }

    fn load_staging_validation_window(
        &self,
        roster: &Self::StagingRoster,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, String, ExpectedFileState)>, ScanError> {
        roster.load_window(self, after, limit)
    }

    fn staging_superseded_by_live(
        &self,
        roster: &Self::StagingRoster,
        root_id: &str,
        location_id: &str,
    ) -> Result<bool, ScanError> {
        roster.superseded_by_live(self, root_id, location_id)
    }

    fn finish_staging_validation(
        &self,
        roster: Self::StagingRoster,
        validated_items: u64,
    ) -> Result<Self::StagingProof, ScanError> {
        roster.finish(validated_items)
    }

    fn publish_validated_scan(
        &mut self,
        proof: &Self::StagingProof,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
        consolidation: Option<&RootConsolidationPlan>,
        control: &ScanPublicationControl,
    ) -> Result<ScanPublicationReceipt, ScanError> {
        proof.publish(
            self,
            root_id,
            asset_count,
            issue_count,
            consolidation,
            control,
        )
    }

    fn create_rejected_input_roster(&self) -> Result<Self::RejectedRoster, ScanError> {
        RejectedInputValidationRoster::create(self)
    }

    fn record_rejected_input(
        &self,
        roster: &Self::RejectedRoster,
        file: &DiscoveredFile,
    ) -> Result<(), ScanError> {
        roster.record(self, file)
    }

    fn load_rejected_input_window(
        &self,
        roster: &Self::RejectedRoster,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(String, ExpectedFileState)>, ScanError> {
        roster.load_window(self, after, limit)
    }

    fn load_retained_scan_issue_window(
        &self,
        scan_id: &str,
        codes: &[&str],
        after_id: i64,
        limit: u32,
    ) -> Result<Vec<RetainedScanIssue>, ScanError> {
        load_retained_scan_issue_window(self, scan_id, codes, after_id, limit)
    }

    fn retained_scan_has_unclassified_issues(
        &self,
        scan_id: &str,
        classified_codes: &[&str],
    ) -> Result<bool, ScanError> {
        retained_scan_has_unclassified_issues(self, scan_id, classified_codes)
    }

    fn fail_scan_publication_namespace(
        &mut self,
        scan_id: &str,
        root_id: &str,
        failure: &ScanError,
        issue_count: u64,
    ) -> Result<(), ScanError> {
        self.fail_scan_publication_namespace(scan_id, root_id, failure, issue_count)
    }

    fn preserve_authoritative_retry_evidence(
        &mut self,
        scan_id: &str,
        root_id: &str,
        retry_relative_paths: &[String],
    ) -> Result<u64, ScanError> {
        self.preserve_authoritative_retry_evidence(scan_id, root_id, retry_relative_paths)
    }

    fn publish_authoritative_scan(
        &mut self,
        scan_id: &str,
        root_id: &str,
        asset_count: u64,
        issue_count: u64,
        retry_relative_paths: &[String],
    ) -> Result<(), ScanError> {
        self.publish_authoritative_scan(
            scan_id,
            root_id,
            asset_count,
            issue_count,
            retry_relative_paths,
        )
    }

    fn completed_write_epoch(&self) -> u64 {
        self.completed_write_epoch()
    }

    fn wait_for_completed_write_after(&self, observed_epoch: u64, timeout: Duration) -> u64 {
        self.wait_for_completed_write_after(observed_epoch, timeout)
    }
}
