use serde_json::json;

use crate::domain::classification::discovery::{CategoryNaming, CategoryNamingPurpose};
use crate::domain::classification::{CategoryDefinition, CategoryId};

use super::*;

fn fixture() -> serde_json::Value {
    json!([
        {"id": "programming", "label": "代码", "subject": "programming code in an editor or terminal",
         "purposes": [{"id": "screenshot", "label": "代码截图"}]},
        {"id": "cats", "label": "猫咪", "subject": "cats",
         "purposes": [{"id": "photo", "label": "猫咪"}, {"id": "meme", "label": "猫咪表情"}]}
    ])
}

fn parse(value: serde_json::Value) -> Result<CuratedClassificationVocabulary, ScanError> {
    CuratedClassificationVocabulary::parse(&value.to_string(), &AtomicBool::new(false))
}

fn naming(category: &str) -> CategoryNaming {
    let parent = CategoryDefinition::new(
        CategoryId::new(category.into()).unwrap(),
        category.into(),
        vec![format!("a {category}")],
    )
    .unwrap();
    CategoryNaming::WithinCategory(
        CategoryNamingPurpose::new(
            &parent,
            "{subject} result".into(),
            format!("a {category} showing {{subject}}"),
        )
        .unwrap(),
    )
}

#[test]
fn curated_topics_keep_existing_ids_and_reject_unlisted_combinations() {
    let vocabulary = parse(fixture()).unwrap();
    let programming = &vocabulary.concepts
        [&resource::identity("ame.managed-naming-context", "topic-programming").unwrap()];
    assert_eq!(
        naming("screenshot").project(programming).label(),
        "代码截图"
    );
    assert_eq!(
        naming("illustration").project(programming).naming_scope(),
        crate::domain::classification::discovery::ConceptNamingScope::TraversalOnly
    );
    assert!(!CategoryNaming::Unconditioned.admits_description(programming));
    assert!(
        vocabulary
            .naming_contexts()
            .iter()
            .all(|context| context.cues.is_empty() && context.broader.is_empty())
    );
}

#[test]
fn curated_windows_are_ordered_complete_and_have_no_lexical_neighbors() {
    let mut vocabulary = parse(fixture()).unwrap();
    let first = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, None, 1)
        .unwrap();
    assert_eq!(first.concepts.len(), 1);
    assert!(first.has_more);
    let cursor = first.concepts[0].id();
    let second = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, Some(cursor), 1)
        .unwrap();
    assert_eq!(second.concepts.len(), 1);
    assert!(!second.has_more);
    assert!(second.concepts[0].id() > cursor);
    for neighborhood in [
        ConceptNeighborhood::Related(cursor),
        ConceptNeighborhood::Broader(cursor),
    ] {
        let page = vocabulary.read_concepts(neighborhood, None, 64).unwrap();
        assert!(page.concepts.is_empty());
        assert!(!page.has_more);
    }
    assert!(
        vocabulary
            .read_concepts(ConceptNeighborhood::EntryPoints, None, 65)
            .is_err()
    );
}

#[test]
fn catalog_authority_binds_purposes_but_preserves_reordered_features() {
    let original = fixture();
    let first = parse(original.clone()).unwrap();
    let mut reordered = original.clone();
    reordered.as_array_mut().unwrap().reverse();
    reordered[0]["purposes"].as_array_mut().unwrap().reverse();
    assert_eq!(first.revision(), parse(reordered).unwrap().revision());
    let mut changed = original;
    changed[0]["purposes"][0]["id"] = json!("illustration");
    let changed = parse(changed).unwrap();
    assert_ne!(first.revision(), changed.revision());
    for (id, concept) in &first.concepts {
        assert_eq!(concept.description(), changed.concepts[id].description());
    }
}

#[test]
fn malformed_or_cancelled_catalogs_publish_no_partial_vocabulary() {
    let source = fixture();
    for field in ["id", "label", "subject"] {
        let mut value = source.clone();
        value[0][field] = json!("");
        assert!(parse(value).is_err(), "empty {field} must be rejected");
    }
    let mut duplicate = source.clone();
    duplicate.as_array_mut().unwrap().push(source[0].clone());
    assert!(parse(duplicate).is_err());
    let mut empty_purposes = source.clone();
    empty_purposes[0]["purposes"] = json!([]);
    assert!(parse(empty_purposes).is_err());
    let mut duplicate_purpose = source.clone();
    duplicate_purpose[0]["purposes"]
        .as_array_mut()
        .unwrap()
        .push(source[0]["purposes"][0].clone());
    assert!(parse(duplicate_purpose).is_err());
    assert!(parse(json!([])).is_err());
    assert!(
        CuratedClassificationVocabulary::parse(&source.to_string(), &AtomicBool::new(true))
            .is_err()
    );
}

#[test]
fn bundled_catalog_resolves_to_known_purposes_without_external_resources() {
    let taxonomy = crate::application::classification_candidate::initial_categories().unwrap();
    let vocabulary = CuratedClassificationVocabulary::bundled(&AtomicBool::new(false)).unwrap();
    assert_eq!(vocabulary.concepts.len(), 38);
    let topics: Vec<Topic> = serde_json::from_str(CATALOG).unwrap();
    for topic in topics {
        for purpose in topic.purposes {
            assert!(
                taxonomy
                    .categories()
                    .iter()
                    .any(|category| category.id().as_str() == purpose.id)
            );
        }
    }
}
