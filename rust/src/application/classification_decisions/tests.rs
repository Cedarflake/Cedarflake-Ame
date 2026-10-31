use super::*;
use crate::domain::classification::*;

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}

fn category(value: &str) -> CategoryId {
    CategoryId::new(value.into()).unwrap()
}

fn taxonomy() -> Taxonomy {
    Taxonomy::new(
        ["photo", "meme"]
            .into_iter()
            .map(|name| {
                CategoryDefinition::new(category(name), name.into(), vec![format!("a {name}")])
                    .unwrap()
            })
            .collect(),
    )
    .unwrap()
}

fn observation() -> ClassificationObservation {
    let content = ClassificationContent {
        scheme: id("blake3-256-full-v1"),
        digest: [10; 32],
        byte_length: 512,
    };
    ClassificationObservation {
        target: ClassificationTarget {
            subject: ClassificationSubject::Exact(content.clone()),
            membership: revision(11),
        },
        authority: ClassificationFieldAuthority {
            category: revision(12),
            review: revision(13),
        },
        rules: taxonomy().revision(),
        configuration: Some(revision(14)),
        human: HumanClassification::Unreviewed,
        deferred: false,
        prediction: Some(ClassificationProposal {
            evidence: PredictionReference {
                prediction_id: id("prediction-1"),
                run_id: id("run-1"),
                configuration: revision(14),
                primary_rules: taxonomy().revision(),
                content,
                category: category("photo"),
            },
            review: ReviewRecommendation::RoutineSampling,
        }),
    }
}

fn choose(name: &str) -> HumanCategoryChoice {
    HumanCategoryChoice::Corrected {
        category: category(name),
        rules: taxonomy().revision(),
    }
}

fn local_choice(name: &str) -> HumanClassification {
    HumanClassification::Chosen {
        category: category(name),
        origin: HumanDecisionOrigin::Local,
    }
}

#[test]
fn confirmation_preserves_the_exact_displayed_evidence() {
    let current = observation();
    let change = plan_classification_change(
        &current,
        &current,
        &ClassificationAction::Confirm,
        &taxonomy(),
    )
    .unwrap();
    assert_eq!(
        change,
        ClassificationChange::Choose(HumanCategoryChoice::Confirmed(
            current.prediction.unwrap().evidence
        ))
    );
}

#[test]
fn replacement_prediction_is_not_accepted_even_if_its_category_is_unchanged() {
    let observed = observation();
    let mut current = observed.clone();
    current.prediction.as_mut().unwrap().evidence.prediction_id = id("prediction-2");
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::ClassificationPredictionChanged)
    );
}

#[test]
fn changed_run_or_configuration_invalidates_confirmation() {
    let observed = observation();
    let mut current = observed.clone();
    current.prediction.as_mut().unwrap().evidence.run_id = id("run-2");
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::ClassificationPredictionChanged)
    );
    current = observed.clone();
    current.configuration = Some(revision(30));
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::NoCompatiblePrediction)
    );
}

#[test]
fn each_content_membership_and_field_boundary_rejects_a_stale_write() {
    let observed = observation();
    let mut alternatives = vec![observed.clone(); 4];
    alternatives[0].target.subject = ClassificationSubject::Provisional(
        ProvisionalClassificationContent::new("asset".into(), 2).unwrap(),
    );
    alternatives[1].target.membership = revision(30);
    alternatives[2].authority.category = revision(30);
    alternatives[3].authority.review = revision(30);
    for current in alternatives {
        for action in [
            ClassificationAction::Confirm,
            ClassificationAction::Correct(category("meme")),
            ClassificationAction::Defer,
        ] {
            assert_eq!(
                plan_classification_change(&observed, &current, &action, &taxonomy()),
                Err(ClassificationError::ClassificationTargetChanged)
            );
        }
    }
}

#[test]
fn category_correction_survives_a_model_only_replacement() {
    let observed = observation();
    let mut current = observed.clone();
    current.configuration = None;
    current.prediction = None;
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Correct(category("meme")),
            &taxonomy()
        ),
        Ok(ClassificationChange::Choose(choose("meme")))
    );
}

#[test]
fn provisional_content_allows_explicit_correction_but_no_confirmation() {
    let mut current = observation();
    current.target.subject = ClassificationSubject::Provisional(
        ProvisionalClassificationContent::new("asset".into(), 2).unwrap(),
    );
    assert_eq!(current.effective().category, None);
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::NoCompatiblePrediction)
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Correct(category("meme")),
            &taxonomy()
        ),
        Ok(ClassificationChange::Choose(choose("meme")))
    );
}

#[test]
fn changed_or_retired_category_definitions_reject_the_old_choice() {
    let observed = observation();
    let mut current = observed.clone();
    current.rules = revision(30);
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Correct(category("meme")),
            &taxonomy()
        ),
        Err(ClassificationError::ClassificationRulesChanged)
    );
    assert_eq!(
        plan_classification_change(
            &observed,
            &observed,
            &ClassificationAction::Correct(category("retired")),
            &taxonomy()
        ),
        Err(ClassificationError::CategoryRetired)
    );
}

#[test]
fn display_renaming_preserves_an_explicit_correction() {
    let original = taxonomy();
    let renamed = Taxonomy::new(
        original
            .categories()
            .iter()
            .map(|definition| {
                definition
                    .renamed(format!("new {}", definition.label()))
                    .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let current = observation();
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Correct(category("meme")),
            &renamed
        ),
        Ok(ClassificationChange::Choose(choose("meme")))
    );
}

#[test]
fn human_conflict_requires_an_explicit_category() {
    let mut current = observation();
    current.human = HumanClassification::Conflict;
    assert_eq!(
        current.effective(),
        EffectiveClassification {
            category: None,
            review: ClassificationReview::Conflict
        }
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::ClassificationConflictUnresolved)
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Correct(category("meme")),
            &taxonomy()
        ),
        Ok(ClassificationChange::Choose(choose("meme")))
    );
}

#[test]
fn machine_predictions_cannot_replace_a_human_choice_or_reopen_review() {
    let mut current = observation();
    current.human = local_choice("meme");
    current.deferred = true;
    assert_eq!(
        current.effective(),
        EffectiveClassification {
            category: Some(category("meme")),
            review: ClassificationReview::Confirmed
        }
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Err(ClassificationError::HumanChoiceRequiresCorrection)
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Defer,
            &taxonomy()
        ),
        Ok(ClassificationChange::Unchanged)
    );
    current.configuration = None;
    current.prediction = None;
    assert_eq!(current.effective().category, Some(category("meme")));
}

#[test]
fn deferred_work_retains_the_usable_category_without_confirmation() {
    let mut current = observation();
    current.deferred = true;
    assert_eq!(
        current.effective(),
        EffectiveClassification {
            category: Some(category("photo")),
            review: ClassificationReview::Deferred
        }
    );
    current.prediction = None;
    assert_eq!(
        current.effective(),
        EffectiveClassification {
            category: None,
            review: ClassificationReview::Deferred
        }
    );
}

#[test]
fn deferral_does_not_depend_on_model_or_rule_changes() {
    let observed = observation();
    let mut current = observed.clone();
    current.configuration = None;
    current.rules = revision(30);
    assert_eq!(
        plan_classification_change(
            &observed,
            &current,
            &ClassificationAction::Defer,
            &taxonomy()
        ),
        Ok(ClassificationChange::Defer)
    );
}

#[test]
fn sampling_and_missing_predictions_do_not_count_as_confirmed_or_pending_review() {
    let mut current = observation();
    assert_eq!(current.effective().review, ClassificationReview::Suggested);
    for recommendation in [
        ReviewRecommendation::Uncalibrated,
        ReviewRecommendation::LowSeparation,
    ] {
        current.prediction.as_mut().unwrap().review = recommendation;
        assert_eq!(
            current.effective().review,
            ClassificationReview::NeedsReview
        );
    }
    current.prediction = None;
    assert_eq!(
        current.effective(),
        EffectiveClassification {
            category: None,
            review: ClassificationReview::Unclassified
        }
    );
}

#[test]
fn stale_content_or_rule_evidence_is_not_an_effective_category() {
    let mut current = observation();
    current.prediction.as_mut().unwrap().evidence.content.digest = [30; 32];
    assert_eq!(current.effective().category, None);
    current = observation();
    current.prediction.as_mut().unwrap().evidence.primary_rules = revision(30);
    assert_eq!(current.effective().category, None);
}

#[test]
fn equal_choices_from_different_provenance_merge_without_a_conflict() {
    let mut consensus = HumanClassificationConsensus::default();
    consensus.include(&choose("photo"), HumanDecisionOrigin::Local);
    consensus.include(
        &HumanCategoryChoice::Confirmed(observation().prediction.unwrap().evidence),
        HumanDecisionOrigin::Local,
    );
    assert_eq!(consensus.resolve(), local_choice("photo"));
}

#[test]
fn conflicting_local_choices_have_no_latest_or_machine_winner() {
    for order in [["photo", "meme"], ["meme", "photo"]] {
        let mut consensus = HumanClassificationConsensus::default();
        for name in order {
            consensus.include(&choose(name), HumanDecisionOrigin::Local);
        }
        consensus.include(&choose("photo"), HumanDecisionOrigin::Restored);
        assert_eq!(consensus.resolve(), HumanClassification::Conflict);
    }
}

#[test]
fn local_choice_takes_precedence_over_imported_choices_in_either_order() {
    for local_first in [true, false] {
        let mut consensus = HumanClassificationConsensus::default();
        if local_first {
            consensus.include(&choose("photo"), HumanDecisionOrigin::Local);
        }
        consensus.include(&choose("photo"), HumanDecisionOrigin::Restored);
        consensus.include(&choose("meme"), HumanDecisionOrigin::Restored);
        if !local_first {
            consensus.include(&choose("photo"), HumanDecisionOrigin::Local);
        }
        assert_eq!(consensus.resolve(), local_choice("photo"));
    }
}

#[test]
fn imported_conflicts_remain_conflicts_without_a_local_choice() {
    let mut consensus = HumanClassificationConsensus::default();
    consensus.include(&choose("photo"), HumanDecisionOrigin::Restored);
    consensus.include(&choose("meme"), HumanDecisionOrigin::Restored);
    assert_eq!(consensus.resolve(), HumanClassification::Conflict);
}

#[test]
fn unchanged_human_choice_does_not_create_another_operation() {
    let mut current = observation();
    current.human = local_choice("photo");
    for action in [
        ClassificationAction::Confirm,
        ClassificationAction::Correct(category("photo")),
        ClassificationAction::Defer,
    ] {
        assert_eq!(
            plan_classification_change(&current, &current, &action, &taxonomy()),
            Ok(ClassificationChange::Unchanged)
        );
    }
}

#[test]
fn provisional_versions_require_a_valid_asset_and_positive_generation() {
    for (asset, generation) in [
        ("".to_owned(), 1),
        ("asset\n".into(), 1),
        ("x".repeat(257), 1),
        ("asset".into(), 0),
        ("asset".into(), u64::MAX),
    ] {
        assert_eq!(
            ProvisionalClassificationContent::new(asset, generation),
            Err(ClassificationError::InvalidClassificationTarget)
        );
    }
}

#[test]
fn explicitly_accepting_an_imported_category_establishes_local_intent() {
    let mut current = observation();
    current.human = HumanClassification::Chosen {
        category: category("photo"),
        origin: HumanDecisionOrigin::Restored,
    };
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Correct(category("photo")),
            &taxonomy()
        ),
        Ok(ClassificationChange::Choose(choose("photo")))
    );
    assert_eq!(
        plan_classification_change(
            &current,
            &current,
            &ClassificationAction::Confirm,
            &taxonomy()
        ),
        Ok(ClassificationChange::Choose(
            HumanCategoryChoice::Confirmed(current.prediction.unwrap().evidence)
        ))
    );
}

#[test]
fn imported_choice_keeps_its_origin_until_an_explicit_local_action() {
    let mut consensus = HumanClassificationConsensus::default();
    consensus.include(&choose("photo"), HumanDecisionOrigin::Restored);
    assert_eq!(
        consensus.resolve(),
        HumanClassification::Chosen {
            category: category("photo"),
            origin: HumanDecisionOrigin::Restored,
        }
    );
}
