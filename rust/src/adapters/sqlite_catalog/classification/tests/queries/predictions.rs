use crate::domain::classification::RefinementIdentity;

use super::*;

#[test]
fn refined_suggestions_and_latest_publication_keep_review_optional() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a"], 'a');
    let initial = setup(&mut catalog);
    let refinement = Taxonomy::new(
        taxonomy()
            .categories()
            .iter()
            .map(|definition| {
                CategoryDefinition::new(
                    definition.id().clone(),
                    definition.label().into(),
                    vec![format!("Refined purpose for {}", definition.label())],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let run = ClassificationRun::new(
        id("refined-run"),
        initial.model(),
        initial.primary_rules(),
        Some(RefinementIdentity {
            rules: refinement.revision(),
            profile: ClassificationRevision::from_bytes([9; 32]),
        }),
    )
    .unwrap();
    catalog
        .begin_classification_run(&run, Some(&refinement))
        .unwrap();
    let observed = prepare(&catalog, &["a"]);
    let crate::domain::classification::ClassificationSubject::Exact(content) =
        &observed.groups[0].target.subject
    else {
        panic!("expected verified fixture")
    };
    let ranking = |rules, drawing, photo| {
        RankedPrediction::new(
            rules,
            run.model(),
            vec![
                CategoryScore {
                    category_id: category("drawing"),
                    similarity: drawing,
                },
                CategoryScore {
                    category_id: category("photo"),
                    similarity: photo,
                },
            ],
        )
        .unwrap()
    };
    for (name, refined, review, expected_category, pending) in [
        (
            "refined",
            Some(ranking(refinement.revision(), 0.1, 0.8)),
            ReviewRecommendation::RoutineSampling,
            "photo",
            false,
        ),
        (
            "tie",
            None,
            ReviewRecommendation::LowSeparation,
            "drawing",
            true,
        ),
    ] {
        let evidence = PredictionEvidence::new(
            id(name),
            run.clone(),
            content.clone(),
            ranking(run.primary_rules(), 0.7, 0.7),
            refined,
            review,
        )
        .unwrap();
        catalog
            .publish_classification_prediction(&selected(&catalog, "a"), &evidence)
            .unwrap();
        assert_eq!(
            ids(
                &mut catalog,
                &query(Some(expected_category), ClassificationReviewFilter::All)
            ),
            ["a"]
        );
        assert!(
            ids(
                &mut catalog,
                &query(None, ClassificationReviewFilter::Confirmed)
            )
            .is_empty()
        );
        assert_eq!(
            ids(
                &mut catalog,
                &query(None, ClassificationReviewFilter::Pending)
            )
            .len(),
            usize::from(pending)
        );
        assert_eq!(
            prepare(&catalog, &["a"]).groups[0].effective().category,
            Some(category(expected_category))
        );
    }
}

#[test]
fn human_projection_uses_the_group_membership_index() {
    let (_directory, catalog) = fixture();
    let mut statement = catalog.connection.prepare(
        "EXPLAIN QUERY PLAN SELECT * FROM classification_human_projection WHERE group_id = 'selected-group'",
    ).unwrap();
    let plan = statement
        .query_map([], |row| row.get::<_, String>(3))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
        .join("\n");
    assert!(
        plan.contains("USING COVERING INDEX organization_record_group_members (group_id=?)"),
        "{plan}"
    );
    assert!(!plan.contains("SCAN decision"), "{plan}");
}
