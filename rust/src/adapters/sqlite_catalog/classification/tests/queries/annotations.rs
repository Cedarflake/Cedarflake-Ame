use crate::domain::classification_selection::ClassificationReviewReason;

use super::*;

fn annotations(
    catalog: &SqliteCatalog,
    names: &[&str],
) -> Vec<(String, ClassificationReviewReason)> {
    let revision = scalar(catalog, "SELECT revision FROM catalog_state") as u64;
    catalog
        .classification_annotations(
            revision,
            &names
                .iter()
                .map(|name| selected(catalog, name))
                .collect::<Vec<_>>(),
        )
        .unwrap()
        .into_iter()
        .map(|item| (item.location_id, item.reason))
        .collect()
}

#[test]
fn annotations_follow_display_groups_and_human_precedence() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "suggestion", "drawing");
    assert_eq!(
        annotations(&catalog, &["a", "b", "c"]),
        [
            ("a".into(), ClassificationReviewReason::Uncalibrated),
            ("c".into(), ClassificationReviewReason::Uncalibrated),
        ]
    );
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .change_classification(&id("defer"), &prepared, &ClassificationAction::Defer)
        .unwrap();
    assert_eq!(
        annotations(&catalog, &["c"]),
        [("c".into(), ClassificationReviewReason::Deferred)]
    );
    correct(&mut catalog, "correct", "c", "photo");
    assert!(annotations(&catalog, &["a", "b", "c"]).is_empty());
    assert_eq!(
        ids(
            &mut catalog,
            &query(None, ClassificationReviewFilter::Confirmed)
        ),
        ["a"]
    );
}

#[test]
fn annotation_reads_reject_stale_windows_and_bound_the_selection() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state") as u64;
    let current = selected(&catalog, "a");
    correct(&mut catalog, "correct", "a", "drawing");
    assert_eq!(
        catalog
            .classification_annotations(revision, std::slice::from_ref(&current))
            .unwrap_err()
            .code,
        "catalog_revision_changed"
    );
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state") as u64;
    let mut stale = current.clone();
    stale.source_generation += 1000;
    assert_eq!(
        catalog
            .classification_annotations(revision, &[stale])
            .unwrap_err()
            .code,
        "organization_selection_changed"
    );
    assert_eq!(
        catalog
            .classification_annotations(revision, &vec![current; 257])
            .unwrap_err()
            .code,
        "organization_selection_invalid"
    );
}

#[test]
fn human_conflict_is_distinct_from_uncertain_prediction() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    correct(&mut catalog, "first", "a", "drawing");
    correct(&mut catalog, "second", "c", "photo");
    publish_group(&mut catalog, &["a", "c"], 'a');
    assert_eq!(
        annotations(&catalog, &["a", "c"]),
        [
            ("a".into(), ClassificationReviewReason::Conflict),
            ("c".into(), ClassificationReviewReason::Conflict),
        ]
    );
    assert_eq!(
        ids(
            &mut catalog,
            &query(None, ClassificationReviewFilter::Pending)
        ),
        ["a"]
    );
}

#[test]
fn retained_display_keeps_annotations_without_authorizing_commands() {
    let (_directory, mut catalog) = fixture();
    publish_group(&mut catalog, &["a", "c"], 'a');
    let run = setup(&mut catalog);
    predict(&mut catalog, &run, "a", "suggestion", "drawing");
    let old = selected(&catalog, "c");
    catalog.connection.execute(
        "UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'c'", [],
    ).unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO gallery_retained_fingerprints VALUES (?1, ?2, 'blake3-256-full-v1')",
            rusqlite::params![
                (old.source_generation + 100) as i64,
                old.source_generation as i64
            ],
        )
        .unwrap();
    assert_eq!(
        annotations(&catalog, &["c"]),
        [("c".into(), ClassificationReviewReason::Uncalibrated)]
    );
    assert_eq!(
        catalog
            .prepare_classification_selection(&[selected(&catalog, "c")])
            .unwrap_err()
            .code,
        "organization_evidence_pending"
    );
}
