use super::*;

#[test]
fn windows_preserve_requested_order_missing_entries_duplicates_and_bounds() {
    let directory = tempfile::tempdir().unwrap();
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let first = text(model(), "first");
    let second = text(model(), "second");
    for feature in [&first, &second] {
        cache.store_text_feature(model(), feature).unwrap();
    }
    let requested = [
        second.description,
        revision(9),
        first.description,
        second.description,
    ];
    let window = cache.load_text_features(model(), &requested).unwrap();
    assert_eq!(
        window,
        vec![Some(second.clone()), None, Some(first), Some(second)]
    );
    assert!(cache.load_text_features(model(), &[]).unwrap().is_empty());
    assert_eq!(
        cache
            .load_text_features(model(), &[revision(9); 65])
            .unwrap_err()
            .code,
        "classification_feature_window_exceeded"
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 2);
}

#[test]
fn window_recency_survives_reopening_and_controls_bounded_eviction() {
    let directory = tempfile::tempdir().unwrap();
    let model = ModelCompatibility::new(revision(1), revision(2), revision(3), 4096).unwrap();
    let budget = (MAX_VECTOR_BYTES as u64 + ENTRY_ALLOWANCE) * 2;
    let first = text(model, "first");
    let second = text(model, "second");
    let third = text(model, "third");
    let mut cache =
        SqliteClassificationFeatureCache::open_with_budget(directory.path(), budget).unwrap();
    for feature in [&first, &second] {
        cache.store_text_feature(model, feature).unwrap();
    }
    cache
        .load_text_features(model, &[second.description, first.description])
        .unwrap();
    drop(cache);
    let mut cache =
        SqliteClassificationFeatureCache::open_with_budget(directory.path(), budget).unwrap();
    cache.store_text_feature(model, &third).unwrap();
    assert_eq!(
        cache
            .load_text_features(
                model,
                &[first.description, second.description, third.description]
            )
            .unwrap(),
        vec![Some(first), None, Some(third)]
    );
    assert_eq!(cache.feature_cache_stats().unwrap().accounted_bytes, budget);
}

#[test]
fn corruption_rejects_the_complete_window_without_partial_recency_updates() {
    let directory = tempfile::tempdir().unwrap();
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let first = text(model(), "first");
    let second = text(model(), "second");
    for feature in [&first, &second] {
        cache.store_text_feature(model(), feature).unwrap();
    }
    cache
        .connection
        .execute(
            "UPDATE features SET checksum = zeroblob(32) WHERE key = ?1",
            [text_key(model(), second.description).bytes().as_slice()],
        )
        .unwrap();
    let clock = |cache: &SqliteClassificationFeatureCache| -> i64 {
        cache
            .connection
            .query_row("SELECT clock FROM feature_cache_state", [], |row| {
                row.get(0)
            })
            .unwrap()
    };
    let before = clock(&cache);
    assert_eq!(
        cache
            .load_text_features(model(), &[first.description, second.description])
            .unwrap_err()
            .code,
        "classification_feature_cache_invalid"
    );
    assert_eq!(clock(&cache), before);
    assert_eq!(
        cache.load_text_feature(model(), first.description).unwrap(),
        Some(first)
    );
}
