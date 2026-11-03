use rusqlite::params;
use serde_json::json;

use crate::application::classification_discovery::{
    NamingContext, NamingContextCatalog, prepare_category_merge,
};
use crate::domain::classification::ClassificationSubject;
use crate::domain::classification::discovery::{ConceptId, VocabularyConcept};
use crate::domain::classification::maintenance::managed_category_identity;

use super::*;

fn seed(catalog: &mut SqliteCatalog) -> (CategoryId, CategoryDefinition, NamingContextCatalog) {
    let (_, child) = start(catalog);
    let current = catalog.load_classification_configuration().unwrap();
    let rules = current.catalog.taxonomy.unwrap();
    let parent = rules
        .categories()
        .iter()
        .find(|item| item.id().as_str() == "drawing")
        .unwrap();
    let concept = ConceptId::from_bytes([10; 32]);
    let broader = CategoryDefinition::new(
        managed_category_identity(concept, Some(parent.id())).unwrap(),
        "Urban scene drawing".into(),
        vec!["a drawing of an urban scene".into()],
    )
    .unwrap();
    let mut categories = rules.categories().to_vec();
    categories.push(broader.clone());
    let change = request(
        catalog,
        "enable-broader",
        Taxonomy::new(categories).unwrap(),
        ClassificationConfigurationAction::Save,
    );
    catalog.save_classification_configuration(&change).unwrap();
    let payload = json!({
        "version": 1,
        "naming": {"kind": "purpose", "category": parent.id().as_str(), "category_revision": codec::revision(parent.semantic_revision()),
            "label_pattern": "{subject} drawing", "comparison_pattern": "a drawing of {subject}"},
        "criteria": {"minimum_members": 2},
        "candidates": [{"category": broader.id().as_str(), "concept": codec::digest(concept.bytes())}],
    }).to_string();
    catalog.connection.execute(
        "INSERT INTO classification_maintenance_evidence VALUES ('enable-broader', 'enable-broader', ?1, ?2)",
        params![blake3::hash(payload.as_bytes()).to_hex().to_string(), payload],
    ).unwrap();
    catalog.connection.execute(
        "INSERT INTO classification_managed_categories VALUES (?1, ?2, 'drawing', 'enable-broader', ?3, ?4, 0)",
        params![broader.id().as_str(), codec::digest(concept.bytes()), broader.label(), codec::revision(broader.semantic_revision())],
    ).unwrap();
    for image in ["a", "b", "c"] {
        let (leading, alternative) = if image == "a" {
            (&child, broader.id())
        } else {
            (broader.id(), &child)
        };
        predict_context(
            catalog,
            &change.setup,
            image,
            &format!("context-{image}"),
            leading,
            alternative,
        );
    }
    let context = |index, label: &str, subject: &str, broader| NamingContext {
        concept: VocabularyConcept::new(
            ConceptId::from_bytes([index; 32]),
            label.into(),
            format!("an image of {subject}"),
        )
        .unwrap()
        .with_subject(subject.into())
        .unwrap(),
        cues: vec![],
        broader,
    };
    let contexts = NamingContextCatalog::new(vec![
        context(9, "City", "a city", vec![concept]),
        context(10, "Urban scene", "an urban scene", vec![]),
    ])
    .unwrap();
    (child, broader, contexts)
}

fn predict_context(
    catalog: &mut SqliteCatalog,
    setup: &ClassificationAnalysisSetup,
    image: &str,
    prediction_id: &str,
    leading: &CategoryId,
    alternative: &CategoryId,
) {
    let observed = super::super::super::prepare(catalog, &[image]);
    let ClassificationSubject::Exact(content) = &observed.groups[0].target.subject else {
        panic!("expected exact content");
    };
    let scores = setup
        .primary()
        .categories()
        .iter()
        .map(|definition| {
            let similarity = if definition.id() == leading {
                0.8
            } else if definition.id() == alternative {
                0.7
            } else {
                0.1
            };
            CategoryScore {
                category_id: definition.id().clone(),
                similarity,
            }
        })
        .collect();
    let run = setup.run();
    let prediction = PredictionEvidence::new(
        id(prediction_id),
        run.clone(),
        content.clone(),
        RankedPrediction::new(run.primary_rules(), run.model(), scores).unwrap(),
        None,
        ReviewRecommendation::Uncalibrated,
    )
    .unwrap();
    assert!(
        catalog
            .publish_classification_prediction(&selected(catalog, image), &prediction)
            .unwrap()
    );
}

fn plan(catalog: &SqliteCatalog, contexts: &NamingContextCatalog) -> CategoryMergePlan {
    prepare_category_merge(
        id("context-merge"),
        survey(catalog),
        Some(contexts),
        catalog,
        &Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
    .unwrap()
}

#[test]
fn broader_merge_preserves_definitions_history_and_stop_across_restart() {
    let (directory, mut catalog) = super::super::super::fixture();
    let (child, broader, contexts) = seed(&mut catalog);
    let before = catalog.load_classification_configuration().unwrap();
    let r4_revision = scalar(&catalog, "SELECT revision FROM organization_state");
    let pending = plan(&catalog, &contexts);
    assert_eq!(pending.category_id(), &child);
    assert_eq!(pending.retained_id(), broader.id());
    let signal = Arc::new(AtomicBool::new(false));
    let merged = catalog.apply_category_merge(&pending, &signal).unwrap();
    let rules = merged.catalog.taxonomy.as_ref().unwrap();
    assert_eq!(rules.categories().len(), 3);
    assert_eq!(
        rules
            .categories()
            .iter()
            .find(|item| item.id() == broader.id()),
        Some(&broader)
    );
    assert_eq!(
        catalog
            .load_classification_setup(&id("enable-broader"))
            .unwrap()
            .unwrap()
            .primary(),
        before.catalog.taxonomy.as_ref().unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT revision FROM organization_state"),
        r4_revision
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        6
    );
    let origins = catalog
        .load_category_expansion_snapshot(std::slice::from_ref(&child))
        .unwrap();
    assert!(origins.known_categories.contains_key(&child));
    assert!(!origins.managed_categories[&child].protected);
    let encoded: String = catalog.connection.query_row(
        "SELECT payload FROM classification_maintenance_evidence WHERE operation_id = 'context-merge'", [], |row| row.get(0),
    ).unwrap();
    let evidence: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(evidence["algorithm"], "complete-broader-context-support-v1");
    assert_eq!(evidence["merge"]["purpose"], "drawing");
    assert_eq!(
        evidence["merge"]["retained_category"],
        broader.id().as_str()
    );
    assert_eq!(evidence["merge"]["contents"], 1);
    assert_eq!(evidence["merge"]["context"]["retained_contents"], 2);
    assert_eq!(evidence["merge"]["context"]["retained_minimum_members"], 2);
    assert_eq!(
        evidence["merge"]["context"]["retained_origin_operation"],
        "enable-broader"
    );
    assert_eq!(
        evidence["merge"]["context"]["path"],
        json!([codec::digest(&[9; 32]), codec::digest(&[10; 32])])
    );
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
        catalog.apply_category_merge(&pending, &signal).unwrap(),
        stopped
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_maintenance_evidence"
        ),
        3
    );
}

#[test]
fn target_changes_between_selection_and_commit_reject_the_entire_merge() {
    for change in ["meaning", "origin", "evidence", "prediction"] {
        let (_directory, mut catalog) = super::super::super::fixture();
        let (_, broader, contexts) = seed(&mut catalog);
        let pending = plan(&catalog, &contexts);
        match change {
            "meaning" => {
                let mut categories = pending
                    .before
                    .categories
                    .configuration
                    .catalog
                    .taxonomy
                    .as_ref()
                    .unwrap()
                    .categories()
                    .to_vec();
                let definition = categories
                    .iter_mut()
                    .find(|item| item.id() == broader.id())
                    .unwrap();
                *definition = CategoryDefinition::new(
                    broader.id().clone(),
                    broader.label().into(),
                    vec!["a drawing of an interior".into()],
                )
                .unwrap();
                let change = request(
                    &catalog,
                    "meaning-change",
                    Taxonomy::new(categories).unwrap(),
                    ClassificationConfigurationAction::Save,
                );
                catalog.save_classification_configuration(&change).unwrap();
            }
            "origin" => {
                catalog.connection.execute("UPDATE classification_managed_categories SET protected = 1 WHERE category_id = ?1", [broader.id().as_str()]).unwrap();
            }
            "evidence" => {
                catalog.connection.execute("DELETE FROM classification_maintenance_evidence WHERE operation_id = 'enable-broader'", []).unwrap();
            }
            "prediction" => {
                let setup = catalog
                    .load_classification_setup(&id("enable-broader"))
                    .unwrap()
                    .unwrap();
                predict_context(
                    &mut catalog,
                    &setup,
                    "b",
                    "new-target-support",
                    &category("drawing"),
                    broader.id(),
                );
            }
            _ => unreachable!(),
        }
        let before = catalog.load_classification_configuration().unwrap();
        assert_eq!(
            catalog
                .apply_category_merge(&pending, &Arc::new(AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "classification_changed",
            "{change}"
        );
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert!(
            catalog
                .load_classification_setup(&id("context-merge"))
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn context_merge_failure_and_cancellation_leave_a_retryable_plan() {
    for cancel in [false, true] {
        let (_directory, mut catalog) = super::super::super::fixture();
        let (_, _, contexts) = seed(&mut catalog);
        let pending = plan(&catalog, &contexts);
        let before = catalog.load_classification_configuration().unwrap();
        let signal = Arc::new(AtomicBool::new(false));
        if cancel {
            let flag = Arc::clone(&signal);
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
        let error = catalog.apply_category_merge(&pending, &signal).unwrap_err();
        if cancel {
            assert_eq!(error.code, "classification_cancelled");
        } else {
            assert!(error.message.contains("fixture"));
        }
        assert!(catalog.connection.is_autocommit());
        assert_eq!(catalog.load_classification_configuration().unwrap(), before);
        assert!(
            catalog
                .load_classification_setup(&id("context-merge"))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            scalar(
                &catalog,
                "SELECT count(*) FROM classification_operations WHERE operation_id = 'context-merge'"
            ),
            0
        );
        if cancel {
            catalog
                .connection
                .update_hook(None::<fn(rusqlite::hooks::Action, &str, &str, i64)>)
                .unwrap();
            signal.store(false, Ordering::Release);
        } else {
            catalog
                .connection
                .execute_batch("DROP TRIGGER reject_merge")
                .unwrap();
        }
        let merged = catalog.apply_category_merge(&pending, &signal).unwrap();
        assert_eq!(merged.dispatch.run_id, Some(id("context-merge")));
    }
}
