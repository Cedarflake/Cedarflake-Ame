use crate::domain::exact_duplicates::EXACT_PUBLICATION_BATCH_SIZE;
use crate::domain::{GalleryDuplicateMode, GalleryQuery};

use super::*;

fn snapshot(catalog: &mut SqliteCatalog) -> crate::domain::CatalogSnapshot {
    catalog
        .load_snapshot(
            10,
            &GalleryQuery {
                duplicate_mode: GalleryDuplicateMode::Merged,
                ..GalleryQuery::default()
            },
            "query",
            None,
            None,
            None,
        )
        .unwrap()
}

fn sources(catalog: &SqliteCatalog) -> Vec<u64> {
    ["a", "b"]
        .map(|id| candidate(catalog, id).location.source_generation)
        .to_vec()
}

#[test]
fn active_run_publishes_completed_pairs_at_one_revision_without_retiring() {
    let (_directory, mut catalog) = fixture();
    let before = snapshot(&mut catalog);
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    let generations = sources(&catalog);
    catalog
        .publish_exact_analysis_batch("run", &generations)
        .unwrap();
    let after = snapshot(&mut catalog);
    assert_eq!(after.revision, before.revision + 1);
    assert_eq!(after.assets.len(), 2);
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT COUNT(*) FROM exact_analysis_outcomes", [], |row| {
                row.get::<_, u32>(0)
            })
            .unwrap(),
        0
    );
    catalog
        .publish_exact_analysis_batch("run", &generations)
        .unwrap();
    assert_eq!(snapshot(&mut catalog).revision, after.revision);
    let mut completed = ExactAnalysisProgress::running("run".to_owned());
    completed.phase = ExactAnalysisPhase::Completed;
    catalog.finish_exact_analysis(&completed).unwrap();
    assert_eq!(snapshot(&mut catalog).revision, after.revision);
    assert_eq!(
        catalog
            .publish_exact_analysis_batch("run", &generations)
            .unwrap_err()
            .code,
        "exact_publication_run_retired"
    );
}

#[test]
fn batch_failure_rolls_back_membership_and_revision() {
    let (_directory, mut catalog) = fixture();
    let before = snapshot(&mut catalog);
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    let generations = sources(&catalog);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_gallery_batch BEFORE INSERT ON gallery_content_fingerprints
         WHEN (SELECT COUNT(*) FROM gallery_content_fingerprints) > 0
         BEGIN SELECT RAISE(ABORT, 'fixture batch failure'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .publish_exact_analysis_batch("run", &generations)
            .is_err()
    );
    let failed = snapshot(&mut catalog);
    assert_eq!(failed.revision, before.revision);
    assert_eq!(failed.assets.len(), 3);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_gallery_batch;")
        .unwrap();
    catalog
        .publish_exact_analysis_batch("run", &generations)
        .unwrap();
    assert_eq!(snapshot(&mut catalog).assets.len(), 2);
}

#[test]
fn batches_are_bounded_and_cannot_admit_another_runs_evidence() {
    let (_directory, mut catalog) = fixture();
    let before = snapshot(&mut catalog);
    publish(&mut catalog, "a", &"a".repeat(64));
    publish(&mut catalog, "b", &"a".repeat(64));
    catalog
        .begin_exact_analysis(&ExactAnalysisRun {
            run_id: "other".to_owned(),
            started_unix_ms: 2,
        })
        .unwrap();
    let generations = sources(&catalog);
    catalog
        .publish_exact_analysis_batch("other", &generations)
        .unwrap();
    assert_eq!(snapshot(&mut catalog).revision, before.revision);
    assert_eq!(snapshot(&mut catalog).assets.len(), 3);
    assert_eq!(
        catalog
            .publish_exact_analysis_batch(
                "run",
                &vec![generations[0]; EXACT_PUBLICATION_BATCH_SIZE + 1]
            )
            .unwrap_err()
            .code,
        "exact_publication_batch_exceeded"
    );
    assert_eq!(snapshot(&mut catalog).revision, before.revision);
}
