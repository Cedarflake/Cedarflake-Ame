use super::*;

fn model(pipeline: u8) -> ModelCompatibility {
    ModelCompatibility::new(
        AnalysisRevision::from_bytes([1; 32]),
        AnalysisRevision::from_bytes([pipeline; 32]),
        AnalysisRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap()
}

fn content(id: u8) -> ImageContentIdentity {
    ImageContentIdentity::new("blake3-256-full-v1".to_owned(), [id; 32], 100).unwrap()
}

fn image(model: ModelCompatibility) -> ImageFeature {
    ImageFeature {
        identity: model.image_identity(),
        embedding: Embedding::normalized(vec![1.0, 2.0]).unwrap(),
    }
}

#[test]
fn retains_exact_features_across_reopen_and_separates_image_pipelines() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("语义特征");
    let cpu = model(2);
    let gpu = model(4);
    let feature = image(cpu);
    let mut store = SqliteSemanticFeatureStore::open(&path).unwrap();
    assert_eq!(
        store.publish_image(cpu, &content(1), &feature).unwrap(),
        FeaturePublication::Stored
    );
    assert_eq!(
        store.publish_image(cpu, &content(1), &feature).unwrap(),
        FeaturePublication::AlreadyPresent
    );
    assert_eq!(store.storage().unwrap().generation, 1);
    assert_eq!(store.load_image(gpu, &content(1)).unwrap(), None);
    assert_eq!(
        store.publish_image(gpu, &content(1), &image(gpu)).unwrap(),
        FeaturePublication::Stored
    );
    drop(store);
    let reopened = SqliteSemanticFeatureStore::open(&path).unwrap();
    assert_eq!(
        reopened.load_image(cpu, &content(1)).unwrap(),
        Some(feature)
    );
    assert_eq!(
        reopened.load_image(gpu, &content(1)).unwrap(),
        Some(image(gpu))
    );
}

#[test]
fn reaching_the_budget_preserves_coverage_and_never_evicts_previous_features() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("features");
    let model = model(2);
    let budget = ENTRY_ALLOWANCE + 8;
    let mut store = SqliteSemanticFeatureStore::open_with_budget(&path, budget).unwrap();
    store
        .publish_image(model, &content(1), &image(model))
        .unwrap();
    assert_eq!(
        store
            .publish_image(model, &content(2), &image(model))
            .unwrap(),
        FeaturePublication::BudgetReached
    );
    let state = store.storage().unwrap();
    assert_eq!(
        (state.features, state.generation, state.accounted_bytes),
        (1, 1, budget)
    );
    assert!(state.budget_reached);
    drop(store);
    let reopened = SqliteSemanticFeatureStore::open_with_budget(&path, budget).unwrap();
    assert_eq!(
        reopened.load_image(model, &content(1)).unwrap(),
        Some(image(model))
    );
    assert_eq!(reopened.load_image(model, &content(2)).unwrap(), None);
    assert!(reopened.storage().unwrap().budget_reached);
}

#[test]
fn failed_publication_rolls_back_vector_and_generation_together() {
    let root = tempfile::tempdir().unwrap();
    let mut store = SqliteSemanticFeatureStore::open(root.path()).unwrap();
    store
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_publication BEFORE UPDATE ON semantic_store_state
         BEGIN SELECT RAISE(ABORT, 'publication rejected'); END;",
        )
        .unwrap();
    assert!(
        store
            .publish_image(model(2), &content(1), &image(model(2)))
            .is_err()
    );
    assert_eq!(store.load_image(model(2), &content(1)).unwrap(), None);
    assert_eq!(
        (
            store.storage().unwrap().generation,
            store.storage().unwrap().features
        ),
        (0, 0)
    );
}

#[test]
fn rejects_damaged_features_without_republishing_them_as_valid() {
    let root = tempfile::tempdir().unwrap();
    let mut store = SqliteSemanticFeatureStore::open(root.path()).unwrap();
    let model = model(2);
    store
        .publish_image(model, &content(1), &image(model))
        .unwrap();
    store
        .connection
        .execute("UPDATE image_features SET vector = zeroblob(8)", [])
        .unwrap();
    assert_eq!(
        store.load_image(model, &content(1)).unwrap_err().code,
        "semantic_storage_invalid"
    );
    assert!(
        store
            .publish_image(model, &content(1), &image(model))
            .is_err()
    );
    assert_eq!(store.storage().unwrap().generation, 1);
}

#[test]
fn refuses_an_unknown_database_without_changing_its_contents() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(storage::FILE_NAME);
    let foreign = Connection::open(&path).unwrap();
    foreign
        .execute_batch("CREATE TABLE records(value TEXT); INSERT INTO records VALUES ('retained');")
        .unwrap();
    drop(foreign);
    let before = std::fs::read(&path).unwrap();
    assert!(
        matches!(SqliteSemanticFeatureStore::open(root.path()), Err(error) if error.code == "semantic_storage_invalid")
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}
