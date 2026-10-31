use super::*;
use crate::domain::classification::ClassificationRecordId;

mod window;

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

fn content(value: u8) -> ClassificationContent {
    ClassificationContent {
        scheme: ClassificationRecordId::new("test-content-v1".into()).unwrap(),
        digest: [value; 32],
        byte_length: 32,
    }
}

fn image(model: ModelCompatibility) -> ImageFeature {
    let mut values = vec![0.0; model.dimensions()];
    values[0] = 0.7;
    values[1] = 0.3;
    ImageFeature {
        identity: model.image_identity(),
        embedding: Embedding::normalized(values).unwrap(),
    }
}

fn text(model: ModelCompatibility, description: &str) -> TextFeature {
    TextFeature {
        identity: model.text_identity(),
        description: ClassificationRevision::description(description),
        embedding: image(model).embedding,
    }
}

#[test]
fn reopening_preserves_exact_normalized_values_and_independent_encoder_keys() {
    let directory = tempfile::tempdir().unwrap();
    let source = content(4);
    let image = image(model());
    let text = text(model(), "a photograph");
    {
        let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
        cache.store_image_feature(model(), &source, &image).unwrap();
        cache.store_text_feature(model(), &text).unwrap();
    }
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    assert_eq!(
        cache.load_image_feature(model(), &source).unwrap(),
        Some(image.clone())
    );
    assert_eq!(
        cache.load_text_feature(model(), text.description).unwrap(),
        Some(text.clone())
    );
    let new_text = ModelCompatibility::new(revision(1), revision(2), revision(8), 2).unwrap();
    assert_eq!(
        cache.load_image_feature(new_text, &source).unwrap(),
        Some(image)
    );
    assert_eq!(
        cache.load_text_feature(new_text, text.description).unwrap(),
        None
    );
    let new_image = ModelCompatibility::new(revision(1), revision(9), revision(3), 2).unwrap();
    assert_eq!(cache.load_image_feature(new_image, &source).unwrap(), None);
    assert_eq!(
        cache
            .load_text_feature(new_image, text.description)
            .unwrap(),
        Some(text)
    );
}

#[test]
fn changed_content_space_dimensions_or_description_cannot_reuse_an_entry() {
    let directory = tempfile::tempdir().unwrap();
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    cache
        .store_image_feature(model(), &content(4), &image(model()))
        .unwrap();
    cache
        .store_text_feature(model(), &text(model(), "a photograph"))
        .unwrap();
    let mut changed = content(4);
    changed.byte_length += 1;
    assert!(
        cache
            .load_image_feature(model(), &changed)
            .unwrap()
            .is_none()
    );
    changed = content(4);
    changed.scheme = ClassificationRecordId::new("another-scheme".into()).unwrap();
    assert!(
        cache
            .load_image_feature(model(), &changed)
            .unwrap()
            .is_none()
    );
    assert!(
        cache
            .load_image_feature(model(), &content(5))
            .unwrap()
            .is_none()
    );
    for other in [
        ModelCompatibility::new(revision(9), revision(2), revision(3), 2).unwrap(),
        ModelCompatibility::new(revision(1), revision(2), revision(3), 3).unwrap(),
    ] {
        assert!(
            cache
                .load_image_feature(other, &content(4))
                .unwrap()
                .is_none()
        );
        assert!(
            cache
                .load_text_feature(other, ClassificationRevision::description("a photograph"))
                .unwrap()
                .is_none()
        );
    }
    assert!(
        cache
            .load_text_feature(
                model(),
                ClassificationRevision::description("a reaction photograph")
            )
            .unwrap()
            .is_none()
    );
}

#[test]
fn eviction_preserves_recently_read_entries_within_the_budget() {
    let directory = tempfile::tempdir().unwrap();
    let model = ModelCompatibility::new(revision(1), revision(2), revision(3), 4096).unwrap();
    let cost = MAX_VECTOR_BYTES as u64 + ENTRY_ALLOWANCE;
    let mut cache =
        SqliteClassificationFeatureCache::open_with_budget(directory.path(), cost * 2).unwrap();
    for value in [1, 2] {
        cache
            .store_image_feature(model, &content(value), &image(model))
            .unwrap();
    }
    assert!(
        cache
            .load_image_feature(model, &content(1))
            .unwrap()
            .is_some()
    );
    cache
        .store_image_feature(model, &content(3), &image(model))
        .unwrap();
    assert!(
        cache
            .load_image_feature(model, &content(1))
            .unwrap()
            .is_some()
    );
    assert!(
        cache
            .load_image_feature(model, &content(2))
            .unwrap()
            .is_none()
    );
    assert!(
        cache
            .load_image_feature(model, &content(3))
            .unwrap()
            .is_some()
    );
    assert_eq!(
        cache.feature_cache_stats().unwrap(),
        ClassificationFeatureCacheStats {
            entries: 2,
            accounted_bytes: cost * 2
        }
    );
}

#[test]
fn insertion_failure_rolls_back_eviction_and_accounting() {
    let directory = tempfile::tempdir().unwrap();
    let model = ModelCompatibility::new(revision(1), revision(2), revision(3), 4096).unwrap();
    let cost = MAX_VECTOR_BYTES as u64 + ENTRY_ALLOWANCE;
    let mut cache =
        SqliteClassificationFeatureCache::open_with_budget(directory.path(), cost).unwrap();
    cache
        .store_image_feature(model, &content(1), &image(model))
        .unwrap();
    cache.connection.execute_batch("CREATE TRIGGER reject_feature BEFORE INSERT ON features BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(
        cache
            .store_image_feature(model, &content(2), &image(model))
            .is_err()
    );
    assert!(
        cache
            .load_image_feature(model, &content(1))
            .unwrap()
            .is_some()
    );
    assert_eq!(cache.feature_cache_stats().unwrap().accounted_bytes, cost);
}

#[test]
fn mismatched_features_and_corrupt_payloads_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    let mut wrong = image(model());
    wrong.identity.pipeline = revision(7);
    assert!(
        cache
            .store_image_feature(model(), &content(1), &wrong)
            .is_err()
    );
    wrong = image(model());
    wrong.embedding = Embedding::normalized(vec![1.0, 0.0, 0.0]).unwrap();
    assert!(
        cache
            .store_image_feature(model(), &content(1), &wrong)
            .is_err()
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 0);
    cache
        .store_image_feature(model(), &content(1), &image(model()))
        .unwrap();
    cache
        .connection
        .execute(
            "UPDATE features SET payload = zeroblob(length(payload))",
            [],
        )
        .unwrap();
    assert_eq!(
        cache
            .load_image_feature(model(), &content(1))
            .unwrap_err()
            .code,
        "classification_feature_cache_invalid"
    );
    let key = image_key(model(), &content(1));
    let zero = [0_u8; 8];
    cache
        .connection
        .execute(
            "UPDATE features SET checksum = ?1",
            [payload_digest(key, &zero).bytes().as_slice()],
        )
        .unwrap();
    assert_eq!(
        cache
            .load_image_feature(model(), &content(1))
            .unwrap_err()
            .code,
        "classification_feature_cache_invalid"
    );
}

#[test]
fn cleanup_is_bounded_and_does_not_remove_other_storage() {
    let directory = tempfile::tempdir().unwrap();
    let decisions = directory.path().join("decisions.sqlite3");
    std::fs::write(&decisions, b"retained decisions").unwrap();
    let mut cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    for index in 0..260 {
        cache
            .store_text_feature(model(), &text(model(), &format!("description {index}")))
            .unwrap();
    }
    assert_eq!(cache.clear_feature_cache_batch().unwrap().entries, 4);
    assert_eq!(
        cache.clear_feature_cache_batch().unwrap(),
        ClassificationFeatureCacheStats {
            entries: 0,
            accounted_bytes: 0
        }
    );
    assert_eq!(std::fs::read(decisions).unwrap(), b"retained decisions");
    drop(cache);
    let cache = SqliteClassificationFeatureCache::open(directory.path()).unwrap();
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 0);
}

#[test]
fn unrelated_and_newer_databases_are_preserved_without_initialization() {
    for application in [0, storage::APPLICATION_ID] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(storage::FILE_NAME);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE retained(value TEXT); INSERT INTO retained VALUES ('untouched');",
            )
            .unwrap();
        connection
            .pragma_update(None, "application_id", application)
            .unwrap();
        connection.pragma_update(None, "user_version", 2).unwrap();
        drop(connection);
        let before = std::fs::read(&path).unwrap();
        assert!(SqliteClassificationFeatureCache::open(directory.path()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[cfg(windows)]
#[test]
fn cache_file_and_directory_cannot_move_until_the_owner_retires() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("cache");
    let cache = SqliteClassificationFeatureCache::open(&root).unwrap();
    let file = root.join(storage::FILE_NAME);
    assert!(std::fs::rename(&file, root.join("moved.sqlite3")).is_err());
    assert!(std::fs::rename(&root, directory.path().join("moved")).is_err());
    drop(cache);
    std::fs::rename(&root, directory.path().join("moved")).unwrap();
}
