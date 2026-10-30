use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchMode,
};
use crate::domain::classification::{
    CategoryScreeningThresholds, ClassificationAnalysisSetup, ClassificationSetupActivation,
    RefinementIdentity, ScreeningIdentity, ScreeningProfile, ScreeningThresholds,
};
use crate::ports::classification_setup::ClassificationSetupRepository;
use crate::ports::classification_work::ClassificationWorkRepository;

use super::*;

fn calibrated(run_id: &str, category_policy: bool) -> ClassificationAnalysisSetup {
    let primary = taxonomy();
    let refinement = Taxonomy::new(
        ["drawing", "photo"]
            .map(|name| {
                CategoryDefinition::new(
                    category(name),
                    name.into(),
                    vec![format!("A refined {name} example")],
                )
                .unwrap()
            })
            .into(),
    )
    .unwrap();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let identity = ScreeningIdentity {
        model,
        primary_rules: primary.revision(),
        refinement_rules: refinement.revision(),
    };
    let calibration = ClassificationRevision::from_bytes([9; 32]);
    let profile = if category_policy {
        ScreeningProfile::for_categories(
            identity,
            calibration,
            vec![
                CategoryScreeningThresholds {
                    category_id: category("drawing"),
                    thresholds: ScreeningThresholds::new(0.123_456_79, 0.031_25).unwrap(),
                },
                CategoryScreeningThresholds {
                    category_id: category("photo"),
                    thresholds: ScreeningThresholds::without_review_calibration(0.000_000_1)
                        .unwrap(),
                },
            ],
        )
        .unwrap()
    } else {
        ScreeningProfile::new(
            identity,
            calibration,
            ScreeningThresholds::new(0.123_456_79, 0.031_25).unwrap(),
        )
    };
    let run = ClassificationRun::new(
        id(run_id),
        model,
        primary.revision(),
        Some(RefinementIdentity {
            rules: refinement.revision(),
            profile: profile.revision(),
        }),
    )
    .unwrap();
    ClassificationAnalysisSetup::new(run, primary, Some((refinement, profile))).unwrap()
}

fn activate(
    catalog: &mut SqliteCatalog,
    setup: &ClassificationAnalysisSetup,
    activation: ClassificationSetupActivation,
) {
    let catalog_revision = catalog.load_classification_state().unwrap().revision;
    let dispatch_revision = catalog.classification_dispatch_state().unwrap().revision;
    catalog
        .activate_classification_setup(catalog_revision, dispatch_revision, setup, activation)
        .unwrap();
}

fn configure(catalog: &mut SqliteCatalog) {
    catalog
        .configure_classification(&id("configure"), 0, &taxonomy())
        .unwrap();
}

#[test]
fn calibrated_thresholds_and_both_rule_sets_survive_restart_exactly() {
    for category_policy in [false, true] {
        let (directory, mut catalog) = fixture();
        configure(&mut catalog);
        let setup = calibrated("calibrated", category_policy);
        activate(&mut catalog, &setup, ClassificationSetupActivation::Enable);
        drop(catalog);
        let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        let restored = catalog
            .load_classification_setup(setup.run().id())
            .unwrap()
            .unwrap();
        assert_eq!(restored, setup);
        assert_eq!(restored.run().configuration(), setup.run().configuration());
        assert_eq!(
            catalog.classification_dispatch_state().unwrap().mode,
            ClassificationDispatchMode::Automatic
        );
    }
}

#[test]
fn missing_legacy_setup_is_distinct_from_explicit_uncalibrated_analysis() {
    let (_directory, mut catalog) = fixture();
    let run = setup(&mut catalog);
    assert!(
        catalog
            .load_classification_setup(run.id())
            .unwrap()
            .is_none()
    );
    let uncalibrated = ClassificationAnalysisSetup::new(run.clone(), taxonomy(), None).unwrap();
    activate(
        &mut catalog,
        &uncalibrated,
        ClassificationSetupActivation::Enable,
    );
    assert_eq!(
        catalog.load_classification_setup(run.id()).unwrap(),
        Some(uncalibrated)
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_analysis_setups WHERE profile_payload IS NULL"
        ),
        1
    );
}

#[test]
fn repeated_setup_activation_preserves_later_manual_intent() {
    for command in [
        ClassificationDispatchCommand::Pause,
        ClassificationDispatchCommand::Disable,
        ClassificationDispatchCommand::Stop,
    ] {
        let (_directory, mut catalog) = fixture();
        configure(&mut catalog);
        let setup = calibrated("activated", true);
        let revision = catalog.load_classification_state().unwrap().revision;
        catalog
            .activate_classification_setup(
                revision,
                0,
                &setup,
                ClassificationSetupActivation::Enable,
            )
            .unwrap();
        let dispatch = catalog.classification_dispatch_state().unwrap();
        let manual = catalog
            .change_classification_dispatch(dispatch.revision, &command)
            .unwrap();
        let retried = catalog
            .activate_classification_setup(
                revision,
                0,
                &setup,
                ClassificationSetupActivation::Enable,
            )
            .unwrap();
        assert_eq!(retried, manual);
        assert_eq!(
            scalar(&catalog, "SELECT count(*) FROM classification_runs"),
            1
        );
    }
}

#[test]
fn replacement_retains_pause_and_old_immutable_setup() {
    let (_directory, mut catalog) = fixture();
    configure(&mut catalog);
    let first = calibrated("first", false);
    activate(&mut catalog, &first, ClassificationSetupActivation::Enable);
    let dispatch = catalog.classification_dispatch_state().unwrap();
    catalog
        .change_classification_dispatch(dispatch.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    let second = calibrated("second", true);
    activate(
        &mut catalog,
        &second,
        ClassificationSetupActivation::ReplaceRun,
    );
    let state = catalog.classification_dispatch_state().unwrap();
    assert_eq!(state.mode, ClassificationDispatchMode::Paused);
    assert_eq!(state.run_id.as_ref(), Some(second.run().id()));
    assert_eq!(
        catalog.load_classification_setup(first.run().id()).unwrap(),
        Some(first.clone())
    );
    assert!(
        catalog
            .activate_classification_setup(0, 0, &first, ClassificationSetupActivation::Enable)
            .is_err()
    );
    assert_eq!(catalog.classification_dispatch_state().unwrap(), state);
}

#[test]
fn stale_or_rejected_activation_rolls_back_run_profile_and_dispatch_together() {
    let (_directory, mut catalog) = fixture();
    configure(&mut catalog);
    let before = catalog.load_classification_state().unwrap();
    let setup = calibrated("atomic", true);
    for (catalog_revision, dispatch_revision) in [(before.revision + 1, 0), (before.revision, 1)] {
        assert!(
            catalog
                .activate_classification_setup(
                    catalog_revision,
                    dispatch_revision,
                    &setup,
                    ClassificationSetupActivation::Enable
                )
                .is_err()
        );
    }
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_setup_activation BEFORE UPDATE ON classification_dispatch_state
         BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .activate_classification_setup(
                before.revision,
                0,
                &setup,
                ClassificationSetupActivation::Enable
            )
            .is_err()
    );
    assert_eq!(catalog.load_classification_state().unwrap(), before);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_runs"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_analysis_setups"
        ),
        0
    );
    assert_eq!(
        catalog.classification_dispatch_state().unwrap().mode,
        ClassificationDispatchMode::Disabled
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_setup_activation")
        .unwrap();
    activate(&mut catalog, &setup, ClassificationSetupActivation::Enable);
}

#[test]
fn changed_unknown_or_missing_profile_never_falls_back_to_uncalibrated() {
    let (_directory, mut catalog) = fixture();
    configure(&mut catalog);
    let setup = calibrated("profile", false);
    activate(&mut catalog, &setup, ClassificationSetupActivation::Enable);
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT profile_payload FROM classification_analysis_setups",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let original: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let mut unknown = original.clone();
    unknown["version"] = serde_json::json!(2);
    let mut changed = original.clone();
    changed["policy"]["thresholds"]["rescreen_bits"] = serde_json::json!(0.5_f32.to_bits());
    let mut invalid = original.clone();
    invalid["policy"]["thresholds"]["review_bits"] = serde_json::json!(f32::NAN.to_bits());
    for value in [unknown, changed, invalid] {
        catalog
            .connection
            .execute(
                "UPDATE classification_analysis_setups SET profile_payload = ?1",
                [value.to_string()],
            )
            .unwrap();
        assert!(catalog.load_classification_setup(setup.run().id()).is_err());
    }
    catalog
        .connection
        .execute(
            "UPDATE classification_analysis_setups SET profile_payload = NULL",
            [],
        )
        .unwrap();
    assert!(catalog.load_classification_setup(setup.run().id()).is_err());
}

#[test]
fn setup_rejects_a_profile_from_other_rules_or_missing_refinement() {
    let first = calibrated("first", true);
    let other = calibrated("other", false);
    let (rules, profile) = other.refinement().unwrap();
    assert!(
        ClassificationAnalysisSetup::new(
            first.run().clone(),
            first.primary().clone(),
            Some((rules.clone(), profile.clone()))
        )
        .is_err()
    );
    assert!(
        ClassificationAnalysisSetup::new(first.run().clone(), first.primary().clone(), None)
            .is_err()
    );
}

#[cfg(all(windows, feature = "classification-worker"))]
#[test]
fn native_restart_restores_paused_setup_without_starting_a_worker() {
    use crate::application::classification_dispatch::{
        ClassificationDispatchSignals, ClassificationDispatchStep, MonotonicDispatchClock,
    };
    use crate::application::classification_runtime::{
        ClassificationRuntimePaths, NativeClassificationRuntime,
    };
    let (directory, mut catalog) = fixture();
    configure(&mut catalog);
    let setup = calibrated("native", true);
    activate(&mut catalog, &setup, ClassificationSetupActivation::Enable);
    let dispatch = catalog.classification_dispatch_state().unwrap();
    catalog
        .change_classification_dispatch(dispatch.revision, &ClassificationDispatchCommand::Pause)
        .unwrap();
    drop(catalog);
    for session in ["process-one", "process-two"] {
        let paths = ClassificationRuntimePaths {
            database: directory.path().join("catalog.sqlite3"),
            feature_cache: directory.path().join("unused-features.sqlite3"),
            worker: directory.path().join("unavailable-worker.exe"),
            models: directory.path().join("unavailable-models"),
            runtime: directory.path().join("unavailable-runtime"),
        };
        let mut runtime = NativeClassificationRuntime::open(
            paths,
            id(session),
            ClassificationDispatchSignals::new(MonotonicDispatchClock::default()),
        )
        .unwrap();
        assert!(matches!(
            runtime.step().unwrap(),
            ClassificationDispatchStep::Waiting { .. }
        ));
        drop(runtime);
        let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        assert_eq!(
            catalog.classification_dispatch_state().unwrap().mode,
            ClassificationDispatchMode::Paused
        );
        assert!(!directory.path().join("unused-features.sqlite3").exists());
    }
}
