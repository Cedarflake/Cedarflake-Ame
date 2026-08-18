use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, LibraryChangeIntent, LibraryChangeIntentKind,
    LibraryChangeOrigin, LibraryChangePlanningResult, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRootGeneration,
};
use crate::ports::LibraryChangeQueue;

use super::enqueue_library_change_plan;

struct RejectingQueue;

impl LibraryChangeQueue for RejectingQueue {
    fn enqueue_library_change_intents(
        &mut self,
        _intents: &[LibraryChangeIntent],
        _enqueued_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeEnqueueReport, crate::domain::ScanError> {
        panic!("an invalid plan must be rejected before reaching the queue adapter")
    }

    fn lease_library_changes(
        &mut self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, crate::domain::ScanError> {
        panic!("lease is not used by this test")
    }

    fn lease_path_library_changes(
        &mut self,
        _root_id: &str,
        _root_generation: LibraryRootGeneration,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<Vec<crate::domain::LeasedLibraryChange>, crate::domain::ScanError> {
        panic!("path lease is not used by this test")
    }

    fn complete_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _catalog_revision_at_success: u64,
        _completed_unix_ms: i64,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, crate::domain::ScanError> {
        panic!("completion is not used by this test")
    }

    fn retry_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _failure: &crate::domain::LibraryChangeFailure,
        _failed_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, crate::domain::ScanError> {
        panic!("retry is not used by this test")
    }

    fn defer_library_change(
        &mut self,
        _change_id: crate::domain::LibraryChangeId,
        _lease_generation: u64,
        _deferred_unix_ms: i64,
    ) -> Result<crate::domain::LibraryChangeLeaseUpdateOutcome, crate::domain::ScanError> {
        panic!("deferral is not used by this test")
    }

    fn load_library_change_queue_metrics(
        &self,
        _now_unix_ms: i64,
        _policy: LibraryChangeQueuePolicy,
    ) -> Result<crate::domain::LibraryChangeQueueMetrics, crate::domain::ScanError> {
        panic!("metrics are not used by this test")
    }

    fn cleanup_terminal_library_changes(
        &mut self,
        _terminal_before_unix_ms: i64,
        _limit: u32,
    ) -> Result<u32, crate::domain::ScanError> {
        panic!("cleanup is not used by this test")
    }
}

#[test]
fn rejects_an_intent_from_a_different_root_generation() {
    let generation = LibraryRootGeneration::initial();
    let plan = LibraryChangePlanningResult {
        root_id: "root-a".to_owned(),
        root_generation: generation,
        freshness: CatalogFreshnessState::Updating,
        freshness_cause: CatalogFreshnessCause::PendingChanges,
        intents: vec![LibraryChangeIntent {
            root_id: "root-b".to_owned(),
            root_generation: generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Path,
            relative_path: "photo.jpg".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::LiveNotification,
            first_observed_unix_ms: 1_000,
            most_recent_observed_unix_ms: 1_000,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        }],
        issues: Vec::new(),
        received_observation_count: 1,
        superseded_observation_count: 0,
    };

    let error = enqueue_library_change_plan(
        &mut RejectingQueue,
        &plan,
        1_000,
        LibraryChangeQueuePolicy::default(),
    )
    .expect_err("mismatched plan");

    assert_eq!(error.code, "change_queue_plan_mismatch");
}
