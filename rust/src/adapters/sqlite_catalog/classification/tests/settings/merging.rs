use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::application::classification_discovery::{
    prepare_purpose_category_merge, survey_category_coverage,
};
use crate::domain::classification::maintenance::CategoryMergePlan;
use crate::ports::classification_configuration::{
    CategoryExpansionRepository, CategoryMergeRepository,
};

use super::*;

mod contexts;
mod fixture;
#[cfg(all(windows, feature = "classification-worker"))]
mod runtime;
use fixture::{predict_child, proposal, sparse, start, survey};

#[test]
fn purpose_merge_preserves_history_and_replays_after_restart_without_resuming_stop() {
    let (directory, mut catalog) = super::super::fixture();
    let (run, child) = start(&mut catalog);
    let cancelled = Arc::new(AtomicBool::new(false));
    assert!(
        prepare_purpose_category_merge(id("not-sparse"), survey(&catalog), &catalog, &cancelled)
            .unwrap()
            .is_none()
    );
    predict_child(&mut catalog, &run, &child, "c", "new-purpose", false);
    let before = catalog.load_classification_configuration().unwrap();
    let r4_revision = scalar(&catalog, "SELECT revision FROM organization_state");
    let plan = proposal(&catalog);
    assert_eq!(plan.category_id(), &child);
    assert_eq!(plan.retained_id().as_str(), "drawing");
    let merged = catalog.apply_category_merge(&plan, &cancelled).unwrap();
    assert_eq!(
        merged.catalog.taxonomy.as_ref().unwrap().categories().len(),
        2
    );
    assert_eq!(merged.dispatch.run_id, Some(id("merge")));
    assert_eq!(
        catalog
            .load_classification_setup(&id("enable"))
            .unwrap()
            .unwrap()
            .primary(),
        before.catalog.taxonomy.as_ref().unwrap()
    );
    assert!(
        catalog
            .load_classification_setup(&id("merge"))
            .unwrap()
            .unwrap()
            .refinement()
            .is_none()
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        r4_revision
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        4
    );
    let origins = catalog
        .load_category_expansion_snapshot(std::slice::from_ref(&child))
        .unwrap();
    assert!(!origins.managed_categories[&child].protected);
    assert!(origins.known_categories.contains_key(&child));
    let payload: String = catalog
        .connection
        .query_row(
            "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'merge'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["version"], 3);
    assert_eq!(payload["merge"]["contents"], 2);
    assert_eq!(payload["merge"]["purpose"], "drawing");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(catalog.load_classification_configuration().unwrap(), merged);
    catalog
        .change_classification_dispatch(
            merged.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let stopped = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        catalog.apply_category_merge(&plan, &cancelled).unwrap(),
        stopped
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_maintenance_evidence"
        ),
        2
    );
}

#[test]
fn late_source_prediction_pause_human_choice_and_creation_evidence_changes_reject_merges() {
    for change in [
        "source",
        "prediction",
        "pause",
        "human",
        "origin",
        "evidence",
    ] {
        let (_directory, mut catalog) = super::super::fixture();
        let child = sparse(&mut catalog);
        let plan = proposal(&catalog);
        match change {
            "source" => {
                catalog
                    .connection
                    .execute("DELETE FROM gallery_content_fingerprints", [])
                    .unwrap();
            }
            "prediction" => {
                let run = catalog
                    .load_classification_setup(&id("enable"))
                    .unwrap()
                    .unwrap()
                    .run()
                    .clone();
                predict_child(&mut catalog, &run, &child, "c", "recurring-again", true);
            }
            "pause" => {
                catalog
                    .change_classification_dispatch(
                        plan.before.categories.configuration.dispatch.revision,
                        &ClassificationDispatchCommand::Pause,
                    )
                    .unwrap();
            }
            "human" => {
                let observed = super::super::prepare(&catalog, &["a"]);
                catalog
                    .change_classification(
                        &id("confirm"),
                        &observed,
                        &ClassificationAction::Confirm,
                    )
                    .unwrap();
            }
            "origin" => {
                catalog
                    .connection
                    .execute(
                        "UPDATE classification_managed_categories SET protected = 1",
                        [],
                    )
                    .unwrap();
            }
            "evidence" => {
                catalog
                    .connection
                    .execute("DELETE FROM classification_maintenance_evidence", [])
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let before = catalog.load_classification_configuration().unwrap();
        assert_eq!(
            catalog
                .apply_category_merge(&plan, &Arc::new(AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "classification_changed",
            "{change}"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(
            scalar(
                &catalog,
                "SELECT count(*) FROM classification_operations WHERE operation_id = 'merge'"
            ),
            0
        );
    }
}

#[test]
fn merge_cancel_and_failed_evidence_insert_roll_back_configuration_run_and_receipt() {
    for cancel in [false, true] {
        let (_directory, mut catalog) = super::super::fixture();
        sparse(&mut catalog);
        let plan = proposal(&catalog);
        let before = catalog.load_classification_configuration().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        if cancel {
            let flag = Arc::clone(&cancelled);
            catalog
                .connection
                .update_hook(Some(
                    move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                        if table == "classification_runs" {
                            flag.store(true, Ordering::Release);
                        }
                    },
                ))
                .unwrap();
        } else {
            catalog.connection.execute_batch("CREATE TRIGGER reject_merge BEFORE INSERT ON classification_maintenance_evidence BEGIN SELECT RAISE(ABORT, 'fixture'); END;").unwrap();
        }
        let error = catalog.apply_category_merge(&plan, &cancelled).unwrap_err();
        if cancel {
            assert_eq!(error.code, "classification_cancelled");
        } else {
            assert!(error.message.contains("fixture"));
        }
        assert_eq!(cancelled.load(Ordering::Acquire), cancel);
        assert!(catalog.connection.is_autocommit());
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert_eq!(
            scalar(
                &catalog,
                "SELECT count(*) FROM classification_operations WHERE operation_id = 'merge'"
            ),
            0
        );
        assert!(
            catalog
                .load_classification_setup(&id("merge"))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            scalar(
                &catalog,
                "WITH RECURSIVE numbers(n) AS (VALUES(1) UNION ALL SELECT n + 1 FROM numbers WHERE n < 2000) SELECT count(*) FROM numbers"
            ),
            2000
        );
    }
}

#[test]
fn missing_original_evidence_and_changed_parent_meaning_preserve_sparse_categories() {
    for missing_evidence in [true, false] {
        let (_directory, mut catalog) = super::super::fixture();
        let child = sparse(&mut catalog);
        if missing_evidence {
            catalog
                .connection
                .execute("DELETE FROM classification_maintenance_evidence", [])
                .unwrap();
        } else {
            let current = catalog.load_classification_configuration().unwrap();
            let categories = current
                .catalog
                .taxonomy
                .unwrap()
                .categories()
                .iter()
                .map(|definition| {
                    if definition.id().as_str() == "drawing" {
                        CategoryDefinition::new(
                            definition.id().clone(),
                            definition.label().into(),
                            vec!["a changed purpose".into()],
                        )
                        .unwrap()
                    } else {
                        definition.clone()
                    }
                })
                .collect();
            let change = request(
                &catalog,
                "changed",
                Taxonomy::new(categories).unwrap(),
                ClassificationConfigurationAction::Save,
            );
            catalog.save_classification_configuration(&change).unwrap();
            for (image, is_child) in [("a", true), ("b", true), ("c", false)] {
                predict_child(
                    &mut catalog,
                    change.setup.run(),
                    &child,
                    image,
                    &format!("changed-{image}"),
                    is_child,
                );
            }
        }
        assert!(survey(&catalog).is_complete());
        assert!(
            prepare_purpose_category_merge(
                id("merge"),
                survey(&catalog),
                &catalog,
                &Arc::new(AtomicBool::new(false))
            )
            .unwrap()
            .is_none()
        );
    }
}
