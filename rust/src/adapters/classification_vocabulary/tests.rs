use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::application::classification_discovery::{ContextualVocabulary, NamingContextCatalog};

fn fixture() -> Value {
    json!({
        "format_version": 1,
        "source_namespace": "test.lexicon",
        "entries": ["kitten", "cat", "animal"],
        "concepts": [
            {"id": "cat", "definition": {"label": "猫", "description": "An image of a cat.", "subject": "cats"}, "related": ["kitten", "animal"], "broader": ["animal"]},
            {"id": "kitten", "definition": {"label": "小猫", "description": "An image of a kitten."}, "related": ["cat"], "broader": ["cat"]},
            {"id": "animal", "definition": {"label": "动物", "description": "An image of an animal."}, "related": ["cat"], "broader": []}
        ],
        "contexts": [
            {"id": "cats", "definition": {"label": "猫咪", "description": "An image featuring cats.", "subject": "cats"}, "cues": ["cat", "kitten"], "broader": ["animals"]},
            {"id": "animals", "definition": {"label": "动物", "description": "An image featuring animals."}, "cues": ["animal"], "broader": []}
        ]
    })
}

fn open(value: &Value) -> Result<(LocalClassificationVocabulary, Vec<NamingContext>), ScanError> {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("词条.json");
    let bytes = serde_json::to_vec(value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    LocalClassificationVocabulary::open(
        &path,
        blake3::hash(&bytes).as_bytes(),
        &AtomicBool::new(false),
    )
}

fn source_id(value: &str) -> ConceptId {
    resource::identity("test.lexicon", value).unwrap()
}

#[test]
fn paged_resource_preserves_graph_and_separates_naming_authority() {
    let (mut vocabulary, definitions) = open(&fixture()).unwrap();
    let first = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, None, 2)
        .unwrap();
    assert!(first.has_more);
    assert_eq!(first.concepts.len(), 2);
    let last = first.concepts.last().unwrap().id();
    let rest = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, Some(last), 2)
        .unwrap();
    assert!(!rest.has_more);
    assert_eq!(rest.concepts.len(), 1);
    assert!(rest.concepts[0].id() > last);
    assert!(
        first
            .concepts
            .iter()
            .chain(&rest.concepts)
            .all(|concept| concept.naming_scope() == ConceptNamingScope::TraversalOnly)
    );

    let neighbors = vocabulary
        .read_concepts(ConceptNeighborhood::Related(source_id("cat")), None, 64)
        .unwrap();
    let mut expected = vec![source_id("animal"), source_id("kitten")];
    expected.sort_unstable();
    assert_eq!(
        neighbors
            .concepts
            .iter()
            .map(VocabularyConcept::id)
            .collect::<Vec<_>>(),
        expected
    );
    let broader = vocabulary
        .read_concepts(ConceptNeighborhood::Broader(source_id("cat")), None, 64)
        .unwrap();
    assert_eq!(broader.concepts[0].id(), source_id("animal"));

    let cancelled = AtomicBool::new(false);
    let cats = definitions
        .iter()
        .find(|context| context.concept.label() == "猫咪")
        .unwrap()
        .concept
        .id();
    let catalog = NamingContextCatalog::new(definitions).unwrap();
    let mut composed = ContextualVocabulary::new(vocabulary, catalog, &cancelled);
    let parents = composed
        .read_concepts(ConceptNeighborhood::Broader(source_id("cat")), None, 64)
        .unwrap();
    assert_eq!(parents.concepts.len(), 2);
    let candidate = parents
        .concepts
        .iter()
        .find(|concept| concept.id() == cats)
        .unwrap();
    assert_eq!(candidate.subject(), Some("cats"));
    assert_eq!(candidate.naming_scope(), ConceptNamingScope::Candidate);
    assert_eq!(
        composed
            .read_concepts(ConceptNeighborhood::Broader(cats), None, 64)
            .unwrap()
            .concepts[0]
            .label(),
        "动物"
    );
}

#[test]
fn revisions_are_order_independent_and_bind_every_relation() {
    let original = fixture();
    let (before, contexts) = open(&original).unwrap();
    let revision = before.revision();
    let context_revision = NamingContextCatalog::new(contexts).unwrap().revision();
    let mut reordered = original.clone();
    for key in ["entries", "concepts", "contexts"] {
        reordered[key].as_array_mut().unwrap().reverse();
    }
    reordered["concepts"][2]["related"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let (after, contexts) = open(&reordered).unwrap();
    assert_eq!(after.revision(), revision);
    assert_eq!(
        NamingContextCatalog::new(contexts).unwrap().revision(),
        context_revision
    );

    let mut changed = original.clone();
    changed["concepts"][0]["broader"] = json!([]);
    assert_ne!(open(&changed).unwrap().0.revision(), revision);
    changed = original.clone();
    changed["entries"] = json!(["cat"]);
    assert_ne!(open(&changed).unwrap().0.revision(), revision);
    changed = original.clone();
    changed["concepts"][0]["definition"]["label"] = json!("家猫");
    let (renamed, _) = open(&changed).unwrap();
    assert_ne!(renamed.revision(), revision);
    assert_eq!(
        renamed.concepts[&source_id("cat")].description(),
        before.concepts[&source_id("cat")].description()
    );
    changed = original;
    changed["contexts"][0]["definition"]["label"] = json!("猫");
    let (source, contexts) = open(&changed).unwrap();
    assert_eq!(source.revision(), revision);
    assert_ne!(
        NamingContextCatalog::new(contexts).unwrap().revision(),
        context_revision
    );
}

#[test]
fn invalid_references_and_duplicate_authority_reject_the_whole_resource() {
    let original = fixture();
    let mutations = [
        ("/concepts/0/id", json!("animal")),
        ("/concepts/0/related", json!(["missing"])),
        ("/concepts/0/broader", json!(["missing"])),
        ("/concepts/0/related", json!(["kitten", "kitten"])),
        ("/entries", json!(["missing"])),
        ("/entries", json!(["cat", "cat"])),
        ("/contexts/0/id", json!("animals")),
        ("/contexts/0/cues", json!(["missing"])),
        ("/contexts/0/cues", json!(["cat", "cat"])),
        ("/contexts/0/broader", json!(["missing"])),
        ("/concepts/0/definition/label", json!("")),
        ("/concepts/0/definition/subject", json!("{cat}")),
        ("/source_namespace", json!("ame.managed-naming-context")),
        ("/source_namespace", json!("invalid namespace")),
        ("/format_version", json!(2)),
    ];
    for (pointer, value) in mutations {
        let mut input = original.clone();
        *input.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            open(&input).err().unwrap().code,
            "classification_vocabulary_invalid",
            "{pointer}"
        );
    }
}

#[test]
fn integrity_cancellation_and_size_are_checked_before_publication() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("resource.json");
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    let expected = *blake3::hash(&bytes).as_bytes();
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        LocalClassificationVocabulary::open(&path, &[0; 32], &AtomicBool::new(false))
            .err()
            .unwrap()
            .code,
        "classification_vocabulary_invalid"
    );
    let (mut admitted, _) =
        LocalClassificationVocabulary::open(&path, &expected, &AtomicBool::new(false)).unwrap();
    std::fs::write(&path, b"replaced").unwrap();
    assert_eq!(
        admitted
            .read_concepts(ConceptNeighborhood::EntryPoints, None, 64)
            .unwrap()
            .concepts
            .len(),
        3
    );
    assert_eq!(
        LocalClassificationVocabulary::open(&path, &expected, &AtomicBool::new(false))
            .err()
            .unwrap()
            .code,
        "classification_vocabulary_invalid"
    );
    assert_eq!(
        LocalClassificationVocabulary::open(
            &directory.path().join("missing"),
            &expected,
            &AtomicBool::new(true)
        )
        .err()
        .unwrap()
        .code,
        "classification_cancelled"
    );
    std::fs::File::create(&path)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        LocalClassificationVocabulary::open(&path, &expected, &AtomicBool::new(false))
            .err()
            .unwrap()
            .code,
        "classification_vocabulary_invalid"
    );
}

#[test]
fn empty_vocabulary_remains_empty_and_page_limits_are_enforced() {
    let value = json!({"format_version": 1, "source_namespace": "test.empty", "entries": [], "concepts": [], "contexts": []});
    let (mut vocabulary, contexts) = open(&value).unwrap();
    assert!(contexts.is_empty());
    let page = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, None, 64)
        .unwrap();
    assert!(page.concepts.is_empty());
    assert!(!page.has_more);
    for limit in [0, 65] {
        assert!(
            vocabulary
                .read_concepts(ConceptNeighborhood::EntryPoints, None, limit)
                .is_err()
        );
    }
    let mut too_many = fixture();
    too_many["entries"] = json!(vec!["cat"; 4097]);
    assert!(open(&too_many).is_err());
    let mut too_many = fixture();
    too_many["concepts"][0]["related"] = json!(vec!["cat"; 16_385]);
    assert!(open(&too_many).is_err());
}
