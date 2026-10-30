use super::*;

use crate::domain::classification::CategoryDefinition;

fn revision(value: u8) -> ClassificationRevision {
    ClassificationRevision::from_bytes([value; 32])
}

fn model() -> ModelCompatibility {
    ModelCompatibility::new(revision(1), revision(2), revision(3), 2).unwrap()
}

fn category(id: &str, descriptions: &[&str]) -> CategoryDefinition {
    CategoryDefinition::new(
        CategoryId::new(id.to_owned()).unwrap(),
        id.to_owned(),
        descriptions.iter().map(|text| (*text).to_owned()).collect(),
    )
    .unwrap()
}

fn text(description: &str, values: [f32; 2]) -> TextFeature {
    TextFeature {
        identity: model().text_identity(),
        description: ClassificationRevision::description(description),
        embedding: Embedding::normalized(values.to_vec()).unwrap(),
    }
}

fn image(values: [f32; 2]) -> ImageFeature {
    ImageFeature {
        identity: model().image_identity(),
        embedding: Embedding::normalized(values.to_vec()).unwrap(),
    }
}

fn taxonomy() -> Taxonomy {
    Taxonomy::new(vec![
        category("photo", &["a photo"]),
        category("meme", &["a reaction image"]),
    ])
    .unwrap()
}

fn features() -> Vec<TextFeature> {
    vec![
        text("a photo", [1.0, 0.0]),
        text("a reaction image", [0.0, 1.0]),
    ]
}

#[test]
fn rematches_retained_images_when_category_descriptions_change() {
    let retained = image([1.0, 0.0]);
    let original = CategoryMatcher::new(taxonomy(), model(), features())
        .unwrap()
        .rank(&retained)
        .unwrap();
    assert_eq!(original.leading().category_id.as_str(), "photo");
    let updated = Taxonomy::new(vec![
        category("photo", &["an ordinary photo"]),
        category("meme", &["a photographed reaction toy"]),
    ])
    .unwrap();
    let matcher = CategoryMatcher::new(
        updated,
        model(),
        vec![
            text("an ordinary photo", [0.0, 1.0]),
            text("a photographed reaction toy", [1.0, 0.0]),
        ],
    )
    .unwrap();
    let changed = matcher.rank(&retained).unwrap();
    assert_eq!(changed.leading().category_id.as_str(), "meme");
    assert_eq!(changed.model(), original.model());
    assert_ne!(changed.rules_revision(), original.rules_revision());
}

#[test]
fn retains_maximum_description_policy_and_emits_actual_separation() {
    let scheme = Taxonomy::new(vec![
        category("a", &["match", "opposite"]),
        category("b", &["near"]),
    ])
    .unwrap();
    let matcher = CategoryMatcher::new(
        scheme,
        model(),
        vec![
            text("match", [1.0, 0.0]),
            text("opposite", [-1.0, 0.0]),
            text("near", [1.0, 1.0]),
        ],
    )
    .unwrap();
    let result = matcher.rank(&image([1.0, 0.0])).unwrap();
    assert_eq!(result.leading().category_id.as_str(), "a");
    assert_eq!(result.leading().similarity, 1.0);
    assert!((result.margin().unwrap() - (1.0 - std::f32::consts::FRAC_1_SQRT_2)).abs() < 0.000001);
}

#[test]
fn tied_categories_have_stable_order_and_zero_margin() {
    let scheme =
        Taxonomy::new(vec![category("z", &["shared"]), category("a", &["shared"])]).unwrap();
    let matcher = CategoryMatcher::new(scheme, model(), vec![text("shared", [1.0, 0.0])]).unwrap();
    let result = matcher.rank(&image([1.0, 0.0])).unwrap();
    assert_eq!(result.categories().len(), 2);
    assert_eq!(result.leading().category_id.as_str(), "a");
    assert_eq!(result.margin(), Some(0.0));
}

#[test]
fn single_category_does_not_invent_a_runner_up_or_probability() {
    let scheme = Taxonomy::new(vec![category("custom", &["one"])]).unwrap();
    let matcher = CategoryMatcher::new(scheme, model(), vec![text("one", [1.0, 0.0])]).unwrap();
    let result = matcher.rank(&image([-1.0, 0.0])).unwrap();
    assert_eq!(result.leading().similarity, -1.0);
    assert_eq!(result.margin(), None);
}

#[test]
fn rejects_missing_duplicate_and_unexpected_text_instead_of_partial_scoring() {
    assert_eq!(
        CategoryMatcher::new(taxonomy(), model(), vec![]).err(),
        Some(ClassificationError::MissingTextFeature)
    );
    assert_eq!(
        CategoryMatcher::new(taxonomy(), model(), vec![text("a photo", [1.0, 0.0]); 2]).err(),
        Some(ClassificationError::DuplicateTextFeature)
    );
    assert_eq!(
        CategoryMatcher::new(taxonomy(), model(), vec![text("different", [1.0, 0.0])]).err(),
        Some(ClassificationError::UnexpectedTextFeature)
    );
}

#[test]
fn runtime_or_batch_policy_change_rejects_old_text_features() {
    let changed = ModelCompatibility::new(revision(1), revision(2), revision(4), 2).unwrap();
    assert_eq!(
        CategoryMatcher::new(taxonomy(), changed, features()).err(),
        Some(ClassificationError::IncompatibleFeature)
    );
    assert_eq!(changed.image_identity(), model().image_identity());
    let mut current = features();
    for feature in &mut current {
        feature.identity = changed.text_identity();
    }
    let matcher = CategoryMatcher::new(taxonomy(), changed, current).unwrap();
    assert!(matcher.rank(&image([1.0, 0.0])).is_ok());
}

#[test]
fn rejects_mixed_image_spaces_pipelines_and_dimensions() {
    let matcher = CategoryMatcher::new(taxonomy(), model(), features()).unwrap();
    let mut stale = image([1.0, 0.0]);
    stale.identity.pipeline = revision(4);
    assert_eq!(
        matcher.rank(&stale),
        Err(ClassificationError::IncompatibleFeature)
    );
    stale.identity = model().image_identity();
    stale.identity.embedding_space = revision(4);
    assert_eq!(
        matcher.rank(&stale),
        Err(ClassificationError::IncompatibleFeature)
    );
    stale.identity = model().image_identity();
    stale.embedding = Embedding::normalized(vec![1.0, 0.0, 0.0]).unwrap();
    assert_eq!(
        matcher.rank(&stale),
        Err(ClassificationError::InvalidEmbedding)
    );
    let mut malformed = features();
    malformed[0].embedding = Embedding::normalized(vec![1.0]).unwrap();
    assert_eq!(
        CategoryMatcher::new(taxonomy(), model(), malformed).err(),
        Some(ClassificationError::InvalidEmbedding)
    );
}

#[test]
fn added_category_reuses_unchanged_text_and_image_features() {
    let retained = image([1.0, 0.0]);
    let before = CategoryMatcher::new(taxonomy(), model(), features())
        .unwrap()
        .rank(&retained)
        .unwrap();
    let mut categories = taxonomy().categories().to_vec();
    categories.push(category("documents", &["a document"]));
    let mut current = features();
    current.push(text("a document", [-1.0, 0.0]));
    let after = CategoryMatcher::new(Taxonomy::new(categories).unwrap(), model(), current)
        .unwrap()
        .rank(&retained)
        .unwrap();
    assert_eq!(after.leading(), before.leading());
    assert_eq!(after.model(), before.model());
    assert_ne!(after.rules_revision(), before.rules_revision());
}
