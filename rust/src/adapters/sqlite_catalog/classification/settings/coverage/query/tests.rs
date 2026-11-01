use super::*;
use crate::adapters::sqlite_catalog::SqliteCatalog;

#[test]
fn coverage_pages_seek_contents_and_latest_predictions_through_existing_indexes() {
    let directory = tempfile::tempdir().unwrap();
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let mut statement = catalog
        .connection
        .prepare(&format!("EXPLAIN QUERY PLAN {}", page_sql()))
        .unwrap();
    let plan: Vec<String> = statement
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                "a".repeat(64),
                20,
                3,
                "b".repeat(64)
            ],
            |row| row.get(3),
        )
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(
        plan.iter()
            .any(|step| step.contains("content_fingerprints_content")
                && step.contains("(digest,byte_length)>")),
        "{plan:?}"
    );
    assert!(
        plan.iter()
            .any(|step| step.contains("classification_predictions_content")),
        "{plan:?}"
    );
    assert!(
        plan.iter()
            .any(|step| step.contains("asset_locations_exact_source")),
        "{plan:?}"
    );
    assert!(
        !plan.iter().any(|step| step.starts_with("SCAN f")
            || step.starts_with("SCAN classification_predictions")),
        "{plan:?}"
    );
}
