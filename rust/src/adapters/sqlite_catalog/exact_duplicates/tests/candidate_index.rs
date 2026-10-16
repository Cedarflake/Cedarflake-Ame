use super::*;

#[test]
fn candidate_pages_reuse_location_cursor_and_size_index_after_retirement() {
    let (_directory, catalog) = fixture();
    let plan = catalog
        .connection
        .prepare(&format!(
            "EXPLAIN QUERY PLAN {}",
            queries::candidate_query()
        ))
        .unwrap()
        .query_map(
            params![
                EXACT_FINGERPRINT_SCHEME,
                "",
                EXACT_DUPLICATE_PAGE_SIZE as i64
            ],
            |row| row.get::<_, String>(3),
        )
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(
        plan.iter().any(|step| step
            .contains("SEARCH l USING INDEX asset_locations_location_id (location_id>?)")),
        "{plan:?}"
    );
    assert!(
        plan.iter().any(|step| step
            .contains("SEARCH peer USING COVERING INDEX asset_locations_exact_size (file_size=?)")),
        "{plan:?}"
    );
    let first = catalog.load_fingerprint_candidates("").unwrap();
    assert_eq!(
        first
            .iter()
            .map(|candidate| candidate.location.location_id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    let after = catalog.load_fingerprint_candidates("b").unwrap();
    assert_eq!(
        after
            .iter()
            .map(|candidate| candidate.location.location_id.as_str())
            .collect::<Vec<_>>(),
        ["c"]
    );
}
