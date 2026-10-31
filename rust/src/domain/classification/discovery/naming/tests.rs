use super::*;
use crate::domain::classification::discovery::ConceptId;

fn category(description: &str) -> CategoryDefinition {
    CategoryDefinition::new(
        CategoryId::new("reactions".into()).unwrap(),
        "表情包".into(),
        vec![description.into()],
    )
    .unwrap()
}

fn purpose() -> CategoryNaming {
    CategoryNaming::WithinCategory(
        CategoryNamingPurpose::new(
            &category("a reaction image"),
            "{subject}表情".into(),
            "an internet meme or reaction sticker featuring {subject}".into(),
        )
        .unwrap(),
    )
}

fn concept() -> VocabularyConcept {
    VocabularyConcept::new(
        ConceptId::from_bytes([1; 32]),
        "猫".into(),
        "A small domesticated animal".into(),
    )
    .unwrap()
}

#[test]
fn subject_metadata_keeps_raw_features_and_composed_names_retain_purpose() {
    let raw = concept();
    let subject = raw.clone().with_subject("cat".into()).unwrap();
    assert_eq!(raw.description(), subject.description());
    assert_ne!(raw.revision(), subject.revision());
    let named = purpose().project(&subject);
    assert_eq!(named.id(), raw.id());
    assert_eq!(named.label(), "猫表情");
    assert_eq!(
        named.description(),
        "an internet meme or reaction sticker featuring cat"
    );
    assert_eq!(named.subject(), Some("cat"));
    assert_ne!(named.revision(), subject.revision());
    assert!(purpose().admits_description(&named));
    assert!(!purpose().admits_description(&subject));
    assert_eq!(CategoryNaming::Unconditioned.project(&subject), subject);
}

#[test]
fn unavailable_subjects_and_oversized_names_remain_traversable_without_raw_name_fallback() {
    let raw = concept();
    let missing = purpose().project(&raw);
    assert_eq!(missing.naming_scope(), ConceptNamingScope::TraversalOnly);
    assert_eq!(missing.id(), raw.id());
    assert_eq!(missing.description(), raw.description());

    let long_label = VocabularyConcept::new(raw.id(), "猫".repeat(80), "animal".into())
        .unwrap()
        .with_subject("cat".into())
        .unwrap();
    assert_eq!(
        purpose().project(&long_label).naming_scope(),
        ConceptNamingScope::TraversalOnly
    );
    let long_description = CategoryNaming::WithinCategory(
        CategoryNamingPurpose::new(
            &category("a reaction image"),
            "{subject}表情".into(),
            format!("{} {{subject}}", "x".repeat(1014)),
        )
        .unwrap(),
    );
    let subject = raw.with_subject("cat".repeat(80)).unwrap();
    assert_eq!(
        long_description.project(&subject).naming_scope(),
        ConceptNamingScope::TraversalOnly
    );
    let traversal = subject.with_naming_scope(ConceptNamingScope::TraversalOnly);
    assert_eq!(purpose().project(&traversal), traversal);
}

#[test]
fn semantic_parent_changes_invalidate_purpose_but_a_display_rename_does_not() {
    let current = category("a reaction image");
    let renamed = current.renamed("反应图".into()).unwrap();
    assert!(
        purpose()
            .validate(&Taxonomy::new(vec![renamed]).unwrap())
            .is_ok()
    );
    assert_eq!(
        purpose().validate(&Taxonomy::new(vec![category("a photograph")]).unwrap()),
        Err(DiscoveryError::NamingPurposeChanged)
    );
    assert_eq!(
        purpose().validate(
            &Taxonomy::new(vec![
                CategoryDefinition::new(
                    CategoryId::new("different".into()).unwrap(),
                    "Other".into(),
                    vec!["something else".into()],
                )
                .unwrap()
            ])
            .unwrap()
        ),
        Err(DiscoveryError::NamingPurposeChanged)
    );
    assert!(purpose().includes(current.id()));
    assert!(!purpose().includes(&CategoryId::new("photographs".into()).unwrap()));
}

#[test]
fn malformed_patterns_and_subjects_are_rejected_before_composition() {
    for pattern in [
        "cats",
        "{subject}",
        " {subject} ",
        "{subject} {subject}",
        "{other} {subject}",
        "a {subject}\nimage",
    ] {
        assert_eq!(
            CategoryNamingPurpose::new(
                &category("a reaction image"),
                pattern.into(),
                "a meme featuring {subject}".into(),
            ),
            Err(DiscoveryError::InvalidNamingPurpose)
        );
    }
    for subject in ["", " ", "cat\nimage", "{subject}"] {
        assert_eq!(
            concept().with_subject(subject.into()),
            Err(DiscoveryError::InvalidSubject)
        );
    }
}

#[test]
fn composition_identity_changes_with_the_parent_or_either_pattern() {
    let current = category("a reaction image");
    let identities: std::collections::BTreeSet<_> = [
        (&current, "{subject}表情", "a meme featuring {subject}"),
        (&current, "{subject}贴纸", "a meme featuring {subject}"),
        (&current, "{subject}表情", "a sticker featuring {subject}"),
        (
            &category("a photograph"),
            "{subject}表情",
            "a meme featuring {subject}",
        ),
    ]
    .into_iter()
    .map(|(parent, label, description)| {
        CategoryNaming::WithinCategory(
            CategoryNamingPurpose::new(parent, label.into(), description.into()).unwrap(),
        )
        .revision()
    })
    .collect();
    assert_eq!(identities.len(), 4);
    assert!(!identities.contains(&CategoryNaming::Unconditioned.revision()));
}
