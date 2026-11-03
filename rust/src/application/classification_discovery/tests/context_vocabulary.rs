use crate::domain::classification::discovery::ConceptNamingScope;

use super::*;

fn context(value: u8, cues: &[u8], broader: &[u8]) -> NamingContext {
    NamingContext {
        concept: VocabularyConcept::new(id(value), "课堂".into(), "a classroom".into())
            .unwrap()
            .with_subject("a lesson in a classroom".into())
            .unwrap(),
        cues: cues.iter().copied().map(id).collect(),
        broader: broader.iter().copied().map(id).collect(),
    }
}

#[test]
fn retrieval_preserves_descriptions_but_only_declared_contexts_can_supply_names() {
    let source = vocabulary();
    let original = source.concepts[&id(1)].clone();
    let catalog =
        NamingContextCatalog::new(vec![context(8, &[1], &[9]), context(9, &[], &[])]).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut vocabulary = ContextualVocabulary::new(source, catalog, &cancelled);
    let page = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, None, 64)
        .unwrap();
    assert_eq!(page.concepts[0].id(), original.id());
    assert_eq!(page.concepts[0].description(), original.description());
    assert!(
        page.concepts
            .iter()
            .all(|row| row.naming_scope() == ConceptNamingScope::TraversalOnly)
    );
    let page = vocabulary
        .read_concepts(ConceptNeighborhood::Broader(id(1)), None, 64)
        .unwrap();
    assert_eq!(
        page.concepts
            .iter()
            .map(VocabularyConcept::id)
            .collect::<Vec<_>>(),
        vec![id(2), id(8)]
    );
    assert_eq!(
        page.concepts[1].naming_scope(),
        ConceptNamingScope::Candidate
    );
    assert_eq!(page.concepts[1].label(), "课堂");
    let page = vocabulary
        .read_concepts(ConceptNeighborhood::Broader(id(8)), None, 64)
        .unwrap();
    assert_eq!(page.concepts[0].id(), id(9));
    assert!(
        vocabulary
            .read_concepts(ConceptNeighborhood::Related(id(8)), None, 64)
            .unwrap()
            .concepts
            .is_empty()
    );
}

#[test]
fn merged_pages_neither_skip_nor_repeat_raw_and_context_neighbors() {
    let mut source = vocabulary();
    for value in [10, 30] {
        source
            .concepts
            .insert(id(value), context(value, &[], &[]).concept);
    }
    source.links.insert(id(1), vec![id(2), id(10), id(30)]);
    let catalog =
        NamingContextCatalog::new(vec![context(8, &[1], &[]), context(20, &[1], &[])]).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut vocabulary = ContextualVocabulary::new(source, catalog, &cancelled);
    let mut after = None;
    let mut ids = Vec::new();
    for _ in 0..3 {
        let page = vocabulary
            .read_concepts(ConceptNeighborhood::Broader(id(1)), after, 2)
            .unwrap();
        ids.extend(page.concepts.iter().map(VocabularyConcept::id));
        after = ids.last().copied();
        assert_eq!(page.has_more, ids.len() < 5);
    }
    assert_eq!(ids, vec![id(2), id(8), id(10), id(20), id(30)]);
    let end = vocabulary
        .read_concepts(ConceptNeighborhood::Broader(id(1)), after, 2)
        .unwrap();
    assert!(end.concepts.is_empty() && !end.has_more);
}

#[test]
fn context_revisions_bind_names_meaning_and_relations_independently_of_order() {
    let first =
        NamingContextCatalog::new(vec![context(8, &[1, 2], &[9]), context(9, &[], &[])]).unwrap();
    let reordered =
        NamingContextCatalog::new(vec![context(9, &[], &[]), context(8, &[2, 1], &[9])]).unwrap();
    assert_eq!(first.revision(), reordered.revision());
    let relinked =
        NamingContextCatalog::new(vec![context(8, &[2], &[9]), context(9, &[], &[])]).unwrap();
    assert_ne!(first.revision(), relinked.revision());
    let without_parent =
        NamingContextCatalog::new(vec![context(8, &[1, 2], &[]), context(9, &[], &[])]).unwrap();
    assert_ne!(first.revision(), without_parent.revision());
    let mut renamed = context(8, &[1, 2], &[9]);
    renamed.concept = VocabularyConcept::new(id(8), "教室".into(), "a classroom".into())
        .unwrap()
        .with_subject("a lesson in a classroom".into())
        .unwrap();
    let renamed = NamingContextCatalog::new(vec![renamed, context(9, &[], &[])]).unwrap();
    assert_ne!(first.revision(), renamed.revision());
    let mut redefined = context(8, &[1, 2], &[9]);
    redefined.concept = redefined
        .concept
        .with_subject("an empty classroom".into())
        .unwrap();
    let redefined = NamingContextCatalog::new(vec![redefined, context(9, &[], &[])]).unwrap();
    assert_ne!(first.revision(), redefined.revision());
}

#[test]
fn malformed_catalogs_cannot_grant_naming_authority() {
    for definitions in [
        vec![context(8, &[1], &[]), context(8, &[2], &[])],
        vec![context(8, &[1, 1], &[])],
        vec![context(8, &[8], &[])],
        vec![context(8, &[1], &[9])],
        vec![context(8, &[1], &[8, 8])],
    ] {
        assert_eq!(
            NamingContextCatalog::new(definitions).err().unwrap().code,
            "classification_context_catalog_invalid"
        );
    }
    let mut disabled = context(8, &[1], &[]);
    disabled.concept = disabled
        .concept
        .with_naming_scope(ConceptNamingScope::TraversalOnly);
    assert!(NamingContextCatalog::new(vec![disabled]).is_err());
    assert!(NamingContextCatalog::new((0..4097).map(|_| context(8, &[], &[])).collect()).is_err());
    let mut excessive = context(8, &[], &[]);
    excessive.cues = vec![id(1); 65_537];
    assert!(NamingContextCatalog::new(vec![excessive]).is_err());
}

#[test]
fn empty_context_catalog_keeps_retrieval_available_without_fabricating_names() {
    let cancelled = AtomicBool::new(false);
    let mut vocabulary = ContextualVocabulary::new(
        vocabulary(),
        NamingContextCatalog::new(vec![]).unwrap(),
        &cancelled,
    );
    let page = vocabulary
        .read_concepts(ConceptNeighborhood::EntryPoints, None, 64)
        .unwrap();
    assert_eq!(page.concepts.len(), 2);
    assert!(
        page.concepts
            .iter()
            .all(|row| row.naming_scope() == ConceptNamingScope::TraversalOnly)
    );
}

#[test]
fn source_changes_invalid_windows_and_identity_collisions_remain_errors() {
    let cancelled = AtomicBool::new(false);
    for failure in ["revision", "page", "identity"] {
        let mut source = vocabulary();
        let mut definition = context(8, &[1], &[]);
        match failure {
            "revision" => source.changed_on_read = true,
            "page" => source.invalid_page = true,
            "identity" => definition = context(2, &[1], &[]),
            _ => unreachable!(),
        }
        let catalog = NamingContextCatalog::new(vec![definition]).unwrap();
        let mut vocabulary = ContextualVocabulary::new(source, catalog, &cancelled);
        let before = vocabulary.revision();
        let error = vocabulary
            .read_concepts(ConceptNeighborhood::EntryPoints, None, 64)
            .err()
            .unwrap();
        let expected = match failure {
            "revision" => {
                assert_ne!(before, vocabulary.revision());
                "classification_vocabulary_changed"
            }
            "page" => "classification_vocabulary_invalid",
            "identity" => "classification_context_identity_conflict",
            _ => unreachable!(),
        };
        assert_eq!(error.code, expected);
    }
}

#[test]
fn cancellation_and_window_bounds_apply_to_context_only_reads() {
    let cancelled = AtomicBool::new(false);
    let catalog =
        NamingContextCatalog::new(vec![context(8, &[1], &[9]), context(9, &[], &[])]).unwrap();
    let mut vocabulary = ContextualVocabulary::new(vocabulary(), catalog, &cancelled);
    for limit in [0, 65] {
        assert_eq!(
            vocabulary
                .read_concepts(ConceptNeighborhood::Broader(id(8)), None, limit)
                .err()
                .unwrap()
                .code,
            "classification_vocabulary_invalid"
        );
    }
    cancelled.store(true, Ordering::Release);
    assert_eq!(
        vocabulary
            .read_concepts(ConceptNeighborhood::Broader(id(8)), None, 1)
            .err()
            .unwrap()
            .code,
        "classification_cancelled"
    );
}
