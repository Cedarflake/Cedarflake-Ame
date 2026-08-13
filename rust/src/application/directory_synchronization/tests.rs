use crate::domain::{
    CatalogFreshnessCause, CatalogFreshnessState, DerivedEvidenceDisposition, FileIdentityEvidence,
    IncrementalReconciliationOutcome, LibraryChangeIntentKind, LibraryChangeObservation,
    LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangePlanningContext,
    LibraryChangePlanningIssue, LibraryChangePlanningLimits, LibraryChangeScope,
    LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootGeneration,
    ReconciliationFileEvidence, ReconciliationObservedState,
};

use super::{plan_library_changes, reconcile_path_evidence};

#[test]
fn create_modify_duplicates_and_reordering_coalesce_to_one_final_reconciliation() {
    let result = plan_library_changes(
        &available_context(),
        [
            observation(3, LibraryChangeObservationKind::Modified, "album/image.jpg"),
            observation(1, LibraryChangeObservationKind::Created, "album/image.jpg"),
            observation(2, LibraryChangeObservationKind::Modified, "album/image.jpg"),
            observation(2, LibraryChangeObservationKind::Modified, "album/image.jpg"),
        ],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::Updating);
    assert_eq!(
        result.freshness_cause,
        CatalogFreshnessCause::PendingChanges
    );
    assert_eq!(result.intents.len(), 1);
    let intent = &result.intents[0];
    assert_eq!(intent.kind, LibraryChangeIntentKind::Reconcile);
    assert_eq!(intent.relative_path, "album/image.jpg");
    assert_eq!(intent.first_sequence, 1);
    assert_eq!(intent.most_recent_sequence, 3);
    assert_eq!(intent.coalesced_observation_count, 4);
}

#[test]
fn create_then_delete_still_reconciles_final_filesystem_state() {
    let result = plan_library_changes(
        &available_context(),
        [
            observation(1, LibraryChangeObservationKind::Created, "transient.png"),
            observation(2, LibraryChangeObservationKind::Removed, "transient.png"),
        ],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].kind, LibraryChangeIntentKind::Reconcile);
    assert_eq!(result.intents[0].coalesced_observation_count, 2);
}

#[test]
fn non_rename_observation_ignores_an_inapplicable_previous_path() {
    let mut observation = observation(1, LibraryChangeObservationKind::Modified, "photo.jpg");
    observation.previous_relative_path = Some("../irrelevant.jpg".to_owned());
    let result = plan_library_changes(&available_context(), [observation], default_limits())
        .expect("plan changes");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].relative_path, "photo.jpg");
    assert!(result.intents[0].previous_relative_path.is_none());
    assert!(result.issues.is_empty());
}

#[test]
fn paired_rename_remains_one_atomic_candidate() {
    let result = plan_library_changes(
        &available_context(),
        [rename_observation(
            1,
            "old/image.jpg",
            "new/image.jpg",
            true,
        )],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::RenameCandidate
    );
    assert_eq!(
        result.intents[0].previous_relative_path.as_deref(),
        Some("old/image.jpg")
    );
    assert_eq!(result.intents[0].relative_path, "new/image.jpg");
}

#[test]
fn unpaired_rename_degrades_to_old_and_new_path_reconciliation() {
    let result = plan_library_changes(
        &available_context(),
        [rename_observation(1, "旧/照片.jpg", "新/照片.jpg", false)],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents.len(), 2);
    assert!(result.intents.iter().all(|intent| {
        intent.kind == LibraryChangeIntentKind::Reconcile && intent.previous_relative_path.is_none()
    }));
    let paths = result
        .intents
        .iter()
        .map(|intent| intent.relative_path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        paths,
        std::collections::BTreeSet::from(["旧/照片.jpg", "新/照片.jpg"])
    );
}

#[test]
fn parent_subtree_supersedes_children_but_not_cross_subtree_rename() {
    let result = plan_library_changes(
        &available_context(),
        [
            observation(
                1,
                LibraryChangeObservationKind::Modified,
                "album/child/image.jpg",
            ),
            directory_observation(2, "album"),
            rename_observation(3, "outside/image.jpg", "album/moved.jpg", true),
        ],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents.len(), 2);
    assert!(result.intents.iter().any(|intent| {
        intent.scope == LibraryChangeScope::Subtree && intent.relative_path == "album"
    }));
    assert!(result.intents.iter().any(|intent| {
        intent.kind == LibraryChangeIntentKind::RenameCandidate
            && intent.relative_path == "album/moved.jpg"
    }));
}

#[test]
fn root_directory_change_supersedes_narrower_reconciliation() {
    let result = plan_library_changes(
        &available_context(),
        [
            observation(1, LibraryChangeObservationKind::Modified, "album/image.jpg"),
            directory_observation(2, ""),
        ],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Root);
    assert!(result.intents[0].relative_path.is_empty());
}

#[test]
fn root_scope_drops_an_inapplicable_previous_rename_path() {
    let mut observation = rename_observation(1, "old.jpg", "new.jpg", true);
    observation.scope = LibraryChangeScope::Root;
    let result = plan_library_changes(&available_context(), [observation], default_limits())
        .expect("plan changes");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].kind, LibraryChangeIntentKind::Reconcile);
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Root);
    assert!(result.intents[0].previous_relative_path.is_none());
}

#[test]
fn old_generation_and_other_root_observations_cannot_enter_current_plan() {
    let mut old_generation = observation(1, LibraryChangeObservationKind::Modified, "stale.jpg");
    old_generation.root_generation = LibraryRootGeneration::new(6).expect("generation");
    let mut other_root = observation(2, LibraryChangeObservationKind::Modified, "other.jpg");
    other_root.root_id = "other-root".to_owned();

    let result = plan_library_changes(
        &available_context(),
        [old_generation, other_root],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::Synchronized);
    assert!(result.intents.is_empty());
    assert_eq!(result.superseded_observation_count, 2);
}

#[test]
fn unavailable_root_retains_pending_work_without_claiming_freshness() {
    let mut context = available_context();
    context.availability = LibraryRootAvailability::Offline;
    let result = plan_library_changes(
        &context,
        [observation(
            1,
            LibraryChangeObservationKind::Removed,
            "offline.jpg",
        )],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::Unavailable);
    assert_eq!(
        result.freshness_cause,
        CatalogFreshnessCause::RootUnavailable
    );
    assert_eq!(result.intents.len(), 1);
}

#[test]
fn evidence_gap_and_unhealthy_source_require_authoritative_root_reconciliation() {
    for (health, expected_issue) in [
        (
            LibraryChangeSourceHealth::Healthy,
            LibraryChangePlanningIssue::ChangeEvidenceGap,
        ),
        (
            LibraryChangeSourceHealth::Failed,
            LibraryChangePlanningIssue::ChangeSourceUnhealthy,
        ),
    ] {
        let mut context = available_context();
        context.source_health = health;
        let observations = matches!(health, LibraryChangeSourceHealth::Healthy)
            .then(|| observation(1, LibraryChangeObservationKind::EvidenceGap, "album"))
            .into_iter();
        let result =
            plan_library_changes(&context, observations, default_limits()).expect("plan changes");

        assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
        assert!(result.issues.contains(&expected_issue));
        assert_eq!(result.intents.len(), 1);
        assert_eq!(
            result.intents[0].kind,
            LibraryChangeIntentKind::FreshnessUnknown
        );
        assert_eq!(result.intents[0].scope, LibraryChangeScope::Root);
    }
}

#[test]
fn invalid_or_escaping_paths_never_become_path_intents() {
    for path in ["../outside.jpg", "C:\\absolute.jpg", "/absolute.jpg"] {
        let result = plan_library_changes(
            &available_context(),
            [observation(1, LibraryChangeObservationKind::Created, path)],
            default_limits(),
        )
        .expect("plan changes");

        assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
        assert_eq!(
            result.intents[0].kind,
            LibraryChangeIntentKind::FreshnessUnknown
        );
        assert!(
            result
                .issues
                .contains(&LibraryChangePlanningIssue::InvalidRelativePath)
        );
    }
}

#[test]
fn invalid_previous_rename_path_escalates_instead_of_losing_half_the_event() {
    let result = plan_library_changes(
        &available_context(),
        [rename_observation(1, "../outside.jpg", "inside.jpg", true)],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert!(
        result
            .issues
            .contains(&LibraryChangePlanningIssue::InvalidRelativePath)
    );
}

#[test]
fn chinese_and_long_relative_paths_remain_lossless_and_normalized() {
    let long_name = format!("相册/{}/照片.jpg", "很长".repeat(100));
    let windows_path = long_name.replace('/', "\\");
    let result = plan_library_changes(
        &available_context(),
        [observation(
            1,
            LibraryChangeObservationKind::Created,
            &windows_path,
        )],
        default_limits(),
    )
    .expect("plan changes");

    assert_eq!(result.intents[0].relative_path, long_name);
}

#[test]
fn event_storm_is_bounded_and_escalates_without_silent_loss() {
    let observations = (0_usize..100).map(|index| {
        observation(
            u64::try_from(index).expect("index"),
            LibraryChangeObservationKind::Modified,
            &format!("storm/{index}.jpg"),
        )
    });
    let result = plan_library_changes(
        &available_context(),
        observations,
        LibraryChangePlanningLimits {
            max_observations: 10,
            max_intents: 10,
        },
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
    assert_eq!(
        result.freshness_cause,
        CatalogFreshnessCause::BoundedCapacityExceeded
    );
    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert!(
        result
            .issues
            .contains(&LibraryChangePlanningIssue::ObservationLimitExceeded)
    );
    assert_eq!(result.received_observation_count, 100);
    assert_eq!(result.intents[0].coalesced_observation_count, 100);
}

#[test]
fn distinct_paths_exceeding_intent_capacity_escalate_to_root_reconciliation() {
    let result = plan_library_changes(
        &available_context(),
        [
            observation(1, LibraryChangeObservationKind::Created, "one.jpg"),
            observation(2, LibraryChangeObservationKind::Created, "two.jpg"),
        ],
        LibraryChangePlanningLimits {
            max_observations: 10,
            max_intents: 1,
        },
    )
    .expect("plan changes");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
    assert!(
        result
            .issues
            .contains(&LibraryChangePlanningIssue::IntentLimitExceeded)
    );
    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
}

#[test]
fn reconciliation_decisions_follow_identity_and_final_state_evidence() {
    let prior = evidence("old/photo.jpg", 100, 1, Some("identity-a"));
    let cases = [
        (
            Some(&prior),
            ReconciliationObservedState::Present(evidence(
                "old/photo.jpg",
                100,
                1,
                Some("identity-a"),
            )),
            IncrementalReconciliationOutcome::Unchanged,
            DerivedEvidenceDisposition::RetainCompatible,
        ),
        (
            Some(&prior),
            ReconciliationObservedState::Present(evidence(
                "old/photo.jpg",
                120,
                2,
                Some("identity-a"),
            )),
            IncrementalReconciliationOutcome::Modified,
            DerivedEvidenceDisposition::InvalidateDerived,
        ),
        (
            Some(&prior),
            ReconciliationObservedState::Present(evidence(
                "new/photo.jpg",
                100,
                1,
                Some("identity-a"),
            )),
            IncrementalReconciliationOutcome::RenamedOrMoved,
            DerivedEvidenceDisposition::RetainCompatible,
        ),
        (
            Some(&prior),
            ReconciliationObservedState::Present(evidence(
                "old/photo.jpg",
                100,
                1,
                Some("identity-b"),
            )),
            IncrementalReconciliationOutcome::Replaced,
            DerivedEvidenceDisposition::NoReusableEvidence,
        ),
        (
            None,
            ReconciliationObservedState::Present(evidence("new.jpg", 1, 1, None)),
            IncrementalReconciliationOutcome::Added,
            DerivedEvidenceDisposition::NoReusableEvidence,
        ),
    ];

    for (prior, observed, expected_outcome, expected_disposition) in cases {
        let decision = reconcile_path_evidence(prior, observed);
        assert_eq!(decision.outcome, expected_outcome);
        assert_eq!(decision.evidence_disposition, expected_disposition);
    }
}

#[test]
fn invalid_reconciliation_paths_preserve_the_last_trustworthy_catalog() {
    let prior = evidence("photo.jpg", 100, 1, Some("identity-a"));
    for current_path in ["../outside.jpg", "C:\\absolute.jpg", ""] {
        let decision = reconcile_path_evidence(
            Some(&prior),
            ReconciliationObservedState::Present(evidence(
                current_path,
                100,
                1,
                Some("identity-a"),
            )),
        );
        assert_eq!(
            decision.outcome,
            IncrementalReconciliationOutcome::TerminalIssue
        );
        assert_eq!(
            decision.evidence_disposition,
            DerivedEvidenceDisposition::PreserveLastTrustworthy
        );
        assert_eq!(
            decision.issue_code.as_deref(),
            Some("current_relative_path_invalid")
        );
    }
}

#[test]
fn removal_requires_authoritative_absence_and_failures_preserve_catalog() {
    let prior = evidence("photo.jpg", 100, 1, Some("identity-a"));
    let removed = reconcile_path_evidence(
        Some(&prior),
        ReconciliationObservedState::Missing {
            relative_path: "photo.jpg".to_owned(),
            is_authoritative: true,
        },
    );
    assert_eq!(removed.outcome, IncrementalReconciliationOutcome::Removed);
    assert_eq!(
        removed.evidence_disposition,
        DerivedEvidenceDisposition::RemoveFromCurrentProjection
    );

    for observed in [
        ReconciliationObservedState::Missing {
            relative_path: "photo.jpg".to_owned(),
            is_authoritative: false,
        },
        ReconciliationObservedState::RetryableFailure {
            code: "locked".to_owned(),
        },
        ReconciliationObservedState::Skipped {
            code: "offline_placeholder".to_owned(),
        },
        ReconciliationObservedState::TerminalIssue {
            code: "unsupported".to_owned(),
        },
    ] {
        let decision = reconcile_path_evidence(Some(&prior), observed);
        assert_eq!(
            decision.evidence_disposition,
            DerivedEvidenceDisposition::PreserveLastTrustworthy
        );
        assert_ne!(decision.outcome, IncrementalReconciliationOutcome::Removed);
    }
}

#[test]
fn root_generation_is_nonzero_monotonic_and_overflow_safe() {
    let initial = LibraryRootGeneration::initial();
    assert_eq!(initial.value(), 1);
    assert_eq!(initial.next().expect("next").value(), 2);
    assert!(LibraryRootGeneration::new(0).is_none());
    let maximum = LibraryRootGeneration::new(u64::MAX).expect("maximum");
    assert!(maximum.next().is_none());
}

#[test]
fn invalid_planning_requests_return_synchronization_error_codes() {
    let mut context = available_context();
    context.root_id = "  ".to_owned();
    let invalid_root = plan_library_changes(&context, [], default_limits())
        .expect_err("blank root identifier must fail");
    assert_eq!(invalid_root.code, "change_root_id_invalid");

    let invalid_limits = plan_library_changes(
        &available_context(),
        [],
        LibraryChangePlanningLimits {
            max_observations: 0,
            max_intents: 1,
        },
    )
    .expect_err("zero observation capacity must fail");
    assert_eq!(invalid_limits.code, "change_planning_limit_invalid");
}

fn available_context() -> LibraryChangePlanningContext {
    LibraryChangePlanningContext {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(7).expect("generation"),
        availability: LibraryRootAvailability::Available,
        source_health: LibraryChangeSourceHealth::Healthy,
    }
}

fn default_limits() -> LibraryChangePlanningLimits {
    LibraryChangePlanningLimits::default()
}

fn observation(
    sequence: u64,
    kind: LibraryChangeObservationKind,
    relative_path: &str,
) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(7).expect("generation"),
        sequence,
        observed_unix_ms: 1_000 + i64::try_from(sequence).expect("sequence"),
        kind,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
    }
}

fn directory_observation(sequence: u64, relative_path: &str) -> LibraryChangeObservation {
    let mut observation = observation(
        sequence,
        LibraryChangeObservationKind::DirectoryChanged,
        relative_path,
    );
    observation.scope = LibraryChangeScope::Subtree;
    observation
}

fn rename_observation(
    sequence: u64,
    previous_relative_path: &str,
    relative_path: &str,
    is_reliably_paired: bool,
) -> LibraryChangeObservation {
    let mut observation = observation(
        sequence,
        LibraryChangeObservationKind::Renamed { is_reliably_paired },
        relative_path,
    );
    observation.previous_relative_path = Some(previous_relative_path.to_owned());
    observation
}

fn evidence(
    relative_path: &str,
    file_size: u64,
    modified_unix_ms: i64,
    identity: Option<&str>,
) -> ReconciliationFileEvidence {
    ReconciliationFileEvidence {
        relative_path: relative_path.to_owned(),
        file_size,
        modified_unix_ms,
        file_identity: identity.map(|value| FileIdentityEvidence {
            scheme: "fixture-v1".to_owned(),
            value: value.to_owned(),
        }),
    }
}
