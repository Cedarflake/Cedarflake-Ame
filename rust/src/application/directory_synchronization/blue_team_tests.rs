use crate::domain::{
    CatalogFreshnessState, DerivedEvidenceDisposition, FileIdentityEvidence,
    IncrementalReconciliationOutcome, LibraryChangeIntentKind, LibraryChangeObservation,
    LibraryChangeObservationKind, LibraryChangeOrigin, LibraryChangePlanningContext,
    LibraryChangePlanningIssue, LibraryChangePlanningLimits, LibraryChangeScope,
    LibraryChangeSourceHealth, LibraryRootAvailability, LibraryRootGeneration,
    ReconciliationFileEvidence, ReconciliationObservedState,
};

use super::{plan_library_changes, reconcile_path_evidence};

#[test]
fn fallback_plan_is_invariant_to_equal_sequence_failure_order() {
    let evidence_gap = observation(
        7,
        1_200,
        LibraryChangeObservationKind::EvidenceGap,
        "album",
        LibraryChangeOrigin::LiveNotification,
    );
    let invalid_path = observation(
        7,
        1_100,
        LibraryChangeObservationKind::Modified,
        "../outside.jpg",
        LibraryChangeOrigin::ConsistencyAudit,
    );

    let forward = plan_library_changes(
        &context(),
        [evidence_gap.clone(), invalid_path.clone()],
        LibraryChangePlanningLimits::default(),
    )
    .expect("forward plan");
    let reversed = plan_library_changes(
        &context(),
        [invalid_path, evidence_gap],
        LibraryChangePlanningLimits::default(),
    )
    .expect("reversed plan");

    assert_eq!(forward, reversed);
}

#[test]
fn empty_identity_cannot_prove_cross_path_continuity() {
    let prior = evidence("old/photo.jpg", Some(("", "")));
    let decision = reconcile_path_evidence(
        Some(&prior),
        ReconciliationObservedState::Present(evidence("new/photo.jpg", Some(("", "")))),
    );

    assert_eq!(
        decision.outcome,
        IncrementalReconciliationOutcome::TerminalIssue
    );
    assert_eq!(
        decision.evidence_disposition,
        DerivedEvidenceDisposition::PreserveLastTrustworthy
    );
}

#[test]
fn paired_self_rename_degrades_to_one_path_reconciliation() {
    let mut rename = observation(
        1,
        1_000,
        LibraryChangeObservationKind::Renamed {
            is_reliably_paired: true,
        },
        "album/./photo.jpg",
        LibraryChangeOrigin::LiveNotification,
    );
    rename.previous_relative_path = Some("album\\photo.jpg".to_owned());

    let result = plan_library_changes(&context(), [rename], LibraryChangePlanningLimits::default())
        .expect("plan rename");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].kind, LibraryChangeIntentKind::Reconcile);
    assert_eq!(result.intents[0].relative_path, "album/photo.jpg");
    assert!(result.intents[0].previous_relative_path.is_none());
}

#[test]
fn nul_path_escalates_without_claiming_synchronized_state() {
    let result = plan_library_changes(
        &context(),
        [observation(
            1,
            1_000,
            LibraryChangeObservationKind::Created,
            "album/invalid\0photo.jpg",
            LibraryChangeOrigin::LiveNotification,
        )],
        LibraryChangePlanningLimits::default(),
    )
    .expect("plan invalid path");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
    assert!(
        result
            .issues
            .contains(&LibraryChangePlanningIssue::InvalidRelativePath)
    );
    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
}

#[test]
fn every_permutation_of_one_batch_produces_the_same_plan() {
    let mut rename = observation(
        2,
        1_100,
        LibraryChangeObservationKind::Renamed {
            is_reliably_paired: false,
        },
        "new/photo.jpg",
        LibraryChangeOrigin::StartupCatchUp,
    );
    rename.previous_relative_path = Some("old/photo.jpg".to_owned());
    let observations = vec![
        observation(
            3,
            1_300,
            LibraryChangeObservationKind::Modified,
            "new/photo.jpg",
            LibraryChangeOrigin::LiveNotification,
        ),
        rename,
        observation(
            1,
            1_000,
            LibraryChangeObservationKind::DirectoryChanged,
            "album",
            LibraryChangeOrigin::ConsistencyAudit,
        ),
        observation(
            3,
            1_250,
            LibraryChangeObservationKind::Modified,
            "new\\photo.jpg",
            LibraryChangeOrigin::UserRefresh,
        ),
    ];
    let expected = plan_library_changes(
        &context(),
        observations.clone(),
        LibraryChangePlanningLimits::default(),
    )
    .expect("baseline plan");

    for permutation in permutations(observations) {
        let actual = plan_library_changes(
            &context(),
            permutation,
            LibraryChangePlanningLimits::default(),
        )
        .expect("permuted plan");
        assert_eq!(actual, expected);
    }
}

#[test]
fn one_extra_duplicate_over_the_observation_limit_never_claims_completeness() {
    let duplicate = observation(
        1,
        1_000,
        LibraryChangeObservationKind::Modified,
        "photo.jpg",
        LibraryChangeOrigin::LiveNotification,
    );
    let result = plan_library_changes(
        &context(),
        [duplicate.clone(), duplicate],
        LibraryChangePlanningLimits {
            max_observations: 1,
            max_intents: 10,
        },
    )
    .expect("bounded plan");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
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
}

#[test]
fn unknown_or_inaccessible_root_never_claims_synchronized_without_events() {
    for availability in [
        LibraryRootAvailability::Unknown,
        LibraryRootAvailability::Missing,
        LibraryRootAvailability::Inaccessible,
        LibraryRootAvailability::Offline,
    ] {
        let mut planning_context = context();
        planning_context.availability = availability;
        let result = plan_library_changes(
            &planning_context,
            [],
            LibraryChangePlanningLimits::default(),
        )
        .expect("unavailable plan");

        assert_eq!(result.freshness, CatalogFreshnessState::Unavailable);
        assert_ne!(result.freshness, CatalogFreshnessState::Synchronized);
    }
}

#[test]
fn identity_scheme_mismatch_cannot_preserve_or_rename_an_asset() {
    let prior = evidence("photo.jpg", Some(("scheme-a", "same-value")));
    let decision = reconcile_path_evidence(
        Some(&prior),
        ReconciliationObservedState::Present(evidence(
            "photo.jpg",
            Some(("scheme-b", "same-value")),
        )),
    );

    assert_eq!(decision.outcome, IncrementalReconciliationOutcome::Replaced);
    assert_eq!(
        decision.evidence_disposition,
        DerivedEvidenceDisposition::NoReusableEvidence
    );
}

#[test]
fn nul_identity_cannot_enter_reconciliation_evidence() {
    for identity in [("scheme\0suffix", "value"), ("scheme", "value\0suffix")] {
        let decision = reconcile_path_evidence(
            None,
            ReconciliationObservedState::Present(evidence("photo.jpg", Some(identity))),
        );

        assert_eq!(
            decision.outcome,
            IncrementalReconciliationOutcome::TerminalIssue
        );
        assert_eq!(
            decision.evidence_disposition,
            DerivedEvidenceDisposition::PreserveLastTrustworthy
        );
    }
}

#[test]
fn authoritative_absence_must_be_bound_to_the_prior_path() {
    let prior = evidence("album/current.jpg", Some(("scheme", "identity")));
    let decision = reconcile_path_evidence(
        Some(&prior),
        ReconciliationObservedState::Missing {
            relative_path: "album/different.jpg".to_owned(),
            is_authoritative: true,
        },
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
        Some("missing_relative_path_mismatch")
    );
}

#[test]
fn planning_limits_cannot_disable_the_absolute_capacity_bound() {
    let error = plan_library_changes(
        &context(),
        [],
        LibraryChangePlanningLimits {
            max_observations: usize::MAX,
            max_intents: usize::MAX,
        },
    )
    .expect_err("unbounded limits must be rejected");

    assert_eq!(error.code, "change_planning_limit_exceeded");
}

#[test]
fn nul_root_id_is_rejected_before_it_reaches_persistence() {
    let mut invalid_context = context();
    invalid_context.root_id = "root\0alias".to_owned();
    let error = plan_library_changes(&invalid_context, [], LibraryChangePlanningLimits::default())
        .expect_err("NUL root identifier must fail");

    assert_eq!(error.code, "change_root_id_invalid");
}

#[test]
fn fallback_intent_preserves_the_current_generation_observation_range() {
    let mut stale = observation(
        1,
        500,
        LibraryChangeObservationKind::Modified,
        "stale.jpg",
        LibraryChangeOrigin::LiveNotification,
    );
    stale.root_generation = LibraryRootGeneration::new(2).expect("stale generation");
    let result = plan_library_changes(
        &context(),
        [
            observation(
                8,
                1_800,
                LibraryChangeObservationKind::EvidenceGap,
                "album",
                LibraryChangeOrigin::LiveNotification,
            ),
            observation(
                4,
                1_200,
                LibraryChangeObservationKind::Modified,
                "../outside.jpg",
                LibraryChangeOrigin::ConsistencyAudit,
            ),
            stale,
        ],
        LibraryChangePlanningLimits::default(),
    )
    .expect("fallback plan");

    let intent = &result.intents[0];
    assert_eq!(intent.first_sequence, 4);
    assert_eq!(intent.most_recent_sequence, 8);
    assert_eq!(intent.first_observed_unix_ms, 1_200);
    assert_eq!(intent.most_recent_observed_unix_ms, 1_800);
    assert_eq!(intent.coalesced_observation_count, 2);
}

#[test]
fn parent_subtree_supersession_happens_before_intent_capacity_degradation() {
    let mut observations = (0..8)
        .map(|index| {
            observation(
                index,
                1_000 + i64::try_from(index).expect("index"),
                LibraryChangeObservationKind::Modified,
                &format!("album/{index}.jpg"),
                LibraryChangeOrigin::LiveNotification,
            )
        })
        .collect::<Vec<_>>();
    let mut parent = observation(
        9,
        1_100,
        LibraryChangeObservationKind::DirectoryChanged,
        "album",
        LibraryChangeOrigin::LiveNotification,
    );
    parent.scope = LibraryChangeScope::Subtree;
    observations.push(parent);

    let result = plan_library_changes(
        &context(),
        observations,
        LibraryChangePlanningLimits {
            max_observations: 16,
            max_intents: 8,
        },
    )
    .expect("subtree plan");

    assert_eq!(result.freshness, CatalogFreshnessState::Updating);
    assert!(
        !result
            .issues
            .contains(&LibraryChangePlanningIssue::IntentLimitExceeded)
    );
    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Subtree);
    assert_eq!(result.intents[0].relative_path, "album");
    assert_eq!(result.intents[0].first_sequence, 0);
    assert_eq!(result.intents[0].most_recent_sequence, 9);
    assert_eq!(result.intents[0].coalesced_observation_count, 9);
}

#[test]
fn subtree_supersession_does_not_double_count_one_unpaired_rename() {
    let mut directory = observation(
        2,
        1_200,
        LibraryChangeObservationKind::DirectoryChanged,
        "album",
        LibraryChangeOrigin::LiveNotification,
    );
    directory.scope = LibraryChangeScope::Subtree;
    let mut rename = observation(
        1,
        1_100,
        LibraryChangeObservationKind::Renamed {
            is_reliably_paired: false,
        },
        "album/new.jpg",
        LibraryChangeOrigin::LiveNotification,
    );
    rename.previous_relative_path = Some("album/old.jpg".to_owned());

    let result = plan_library_changes(
        &context(),
        [rename, directory],
        LibraryChangePlanningLimits::default(),
    )
    .expect("subtree plan");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Subtree);
    assert_eq!(result.intents[0].coalesced_observation_count, 2);
}

#[test]
fn parent_subtree_supersedes_nested_subtree_before_capacity_degradation() {
    let mut observations = (0..8)
        .map(|index| {
            let mut child = observation(
                index,
                1_000 + i64::try_from(index).expect("index"),
                LibraryChangeObservationKind::DirectoryChanged,
                &format!("album/{index}"),
                LibraryChangeOrigin::LiveNotification,
            );
            child.scope = LibraryChangeScope::Subtree;
            child
        })
        .collect::<Vec<_>>();
    let mut parent = observation(
        9,
        1_100,
        LibraryChangeObservationKind::DirectoryChanged,
        "album",
        LibraryChangeOrigin::LiveNotification,
    );
    parent.scope = LibraryChangeScope::Subtree;
    observations.push(parent);

    let result = plan_library_changes(
        &context(),
        observations,
        LibraryChangePlanningLimits {
            max_observations: 16,
            max_intents: 8,
        },
    )
    .expect("nested subtree plan");

    assert_eq!(result.freshness, CatalogFreshnessState::Updating);
    assert_eq!(result.intents.len(), 1);
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Subtree);
    assert_eq!(result.intents[0].relative_path, "album");
    assert_eq!(result.intents[0].coalesced_observation_count, 9);
}

#[test]
fn overflow_fallback_is_invariant_to_prefix_composition() {
    let valid = observation(
        3,
        1_300,
        LibraryChangeObservationKind::Modified,
        "photo.jpg",
        LibraryChangeOrigin::LiveNotification,
    );
    let invalid = observation(
        2,
        1_200,
        LibraryChangeObservationKind::Modified,
        "../outside.jpg",
        LibraryChangeOrigin::ConsistencyAudit,
    );
    let mut stale = observation(
        1,
        1_100,
        LibraryChangeObservationKind::Modified,
        "stale.jpg",
        LibraryChangeOrigin::StartupCatchUp,
    );
    stale.root_generation = LibraryRootGeneration::new(2).expect("stale generation");
    let limits = LibraryChangePlanningLimits {
        max_observations: 2,
        max_intents: 2,
    };
    let expected = plan_library_changes(
        &context(),
        [valid.clone(), invalid.clone(), stale.clone()],
        limits,
    )
    .expect("baseline overflow");

    for permutation in permutations(vec![valid.clone(), invalid.clone(), stale.clone()]) {
        let actual =
            plan_library_changes(&context(), permutation, limits).expect("permuted overflow");
        assert_eq!(actual, expected);
    }
}

#[test]
fn coalesced_origin_follows_the_latest_observation_not_enum_priority() {
    let result = plan_library_changes(
        &context(),
        [
            observation(
                7,
                1_000,
                LibraryChangeObservationKind::Modified,
                "photo.jpg",
                LibraryChangeOrigin::ConsistencyAudit,
            ),
            observation(
                7,
                2_000,
                LibraryChangeObservationKind::Modified,
                "photo.jpg",
                LibraryChangeOrigin::LiveNotification,
            ),
        ],
        LibraryChangePlanningLimits::default(),
    )
    .expect("coalesced plan");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].origin,
        LibraryChangeOrigin::LiveNotification
    );
    assert_eq!(result.intents[0].most_recent_observed_unix_ms, 2_000);
}

#[test]
fn rename_without_a_previous_path_cannot_leave_an_unknown_old_location_live() {
    let rename_without_old_path = observation(
        8,
        2_100,
        LibraryChangeObservationKind::Renamed {
            is_reliably_paired: false,
        },
        "new/photo.jpg",
        LibraryChangeOrigin::LiveNotification,
    );

    let result = plan_library_changes(
        &context(),
        [rename_without_old_path],
        LibraryChangePlanningLimits::default(),
    )
    .expect("conservative rename plan");

    assert_eq!(result.freshness, CatalogFreshnessState::NeedsReconciliation);
    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(result.intents[0].scope, LibraryChangeScope::Root);
    assert!(
        result
            .issues
            .contains(&LibraryChangePlanningIssue::ChangeEvidenceGap)
    );
}

#[test]
fn root_fallback_origin_follows_the_latest_current_generation_signal() {
    let earlier_gap = observation(
        10,
        2_000,
        LibraryChangeObservationKind::EvidenceGap,
        "",
        LibraryChangeOrigin::ConsistencyAudit,
    );
    let later_invalid_path = observation(
        11,
        2_100,
        LibraryChangeObservationKind::Modified,
        "../outside.jpg",
        LibraryChangeOrigin::LiveNotification,
    );

    let result = plan_library_changes(
        &context(),
        [later_invalid_path, earlier_gap],
        LibraryChangePlanningLimits::default(),
    )
    .expect("fallback plan");

    assert_eq!(result.intents.len(), 1);
    assert_eq!(
        result.intents[0].kind,
        LibraryChangeIntentKind::FreshnessUnknown
    );
    assert_eq!(
        result.intents[0].origin,
        LibraryChangeOrigin::LiveNotification
    );
}

fn permutations<T: Clone>(values: Vec<T>) -> Vec<Vec<T>> {
    fn collect<T: Clone>(values: &mut [T], index: usize, output: &mut Vec<Vec<T>>) {
        if index == values.len() {
            output.push(values.to_vec());
            return;
        }
        for swap_index in index..values.len() {
            values.swap(index, swap_index);
            collect(values, index + 1, output);
            values.swap(index, swap_index);
        }
    }

    let mut values = values;
    let mut output = Vec::new();
    collect(&mut values, 0, &mut output);
    output
}

fn context() -> LibraryChangePlanningContext {
    LibraryChangePlanningContext {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(3).expect("generation"),
        availability: LibraryRootAvailability::Available,
        source_health: LibraryChangeSourceHealth::Healthy,
    }
}

fn observation(
    sequence: u64,
    observed_unix_ms: i64,
    kind: LibraryChangeObservationKind,
    relative_path: &str,
    origin: LibraryChangeOrigin,
) -> LibraryChangeObservation {
    LibraryChangeObservation {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::new(3).expect("generation"),
        sequence,
        observed_unix_ms,
        kind,
        scope: LibraryChangeScope::Path,
        relative_path: relative_path.to_owned(),
        previous_relative_path: None,
        origin,
    }
}

fn evidence(relative_path: &str, identity: Option<(&str, &str)>) -> ReconciliationFileEvidence {
    ReconciliationFileEvidence {
        relative_path: relative_path.to_owned(),
        file_size: 100,
        modified_unix_ms: 1_000,
        file_identity: identity.map(|(scheme, value)| FileIdentityEvidence {
            scheme: scheme.to_owned(),
            value: value.to_owned(),
        }),
    }
}
