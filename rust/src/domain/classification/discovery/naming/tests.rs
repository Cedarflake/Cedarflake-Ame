use super::*;
use crate::domain::classification::discovery::{ConceptId, ConceptPurposeLabels};

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

#[test]
fn curated_names_require_the_declared_purpose_even_with_matching_description() {
    let restricted = concept()
        .with_subject("cat".into())
        .unwrap()
        .with_purpose_labels(
            ConceptPurposeLabels::new(vec![(
                CategoryId::new("photos".into()).unwrap(),
                "猫咪".into(),
            )])
            .unwrap(),
        );
    for naming in [purpose(), CategoryNaming::Unconditioned] {
        let projected = naming.project(&restricted);
        assert_eq!(projected.naming_scope(), ConceptNamingScope::TraversalOnly);
        assert!(!naming.admits_description(&projected));
        let comparison = naming.project_comparison(&restricted);
        assert_eq!(comparison.naming_scope(), ConceptNamingScope::Candidate);
        assert!(!naming.admits_description(&comparison));
    }
    let mut forged = restricted;
    forged.description = "an internet meme or reaction sticker featuring cat".into();
    assert!(!purpose().admits_description(&forged));
}

#[test]
fn admitted_curated_label_preserves_semantic_composition_and_feature_identity() {
    let ordinary = concept().with_subject("cat".into()).unwrap();
    let restricted = ordinary.clone().with_purpose_labels(
        ConceptPurposeLabels::new(vec![(
            category("a reaction image").id().clone(),
            "猫咪表情".into(),
        )])
        .unwrap(),
    );
    let projected = purpose().project(&restricted);
    assert_eq!(projected.label(), "猫咪表情");
    assert_eq!(
        projected.description(),
        purpose().project(&ordinary).description()
    );
    assert!(purpose().admits_description(&projected));
    assert_ne!(restricted.revision(), ordinary.revision());
}

#[test]
fn curated_purpose_labels_are_canonical_bounded_and_unique() {
    let reaction = CategoryId::new("reactions".into()).unwrap();
    let photo = CategoryId::new("photos".into()).unwrap();
    let entries = vec![
        (reaction.clone(), "猫咪表情".into()),
        (photo.clone(), "猫咪".into()),
    ];
    let first = ConceptPurposeLabels::new(entries.clone()).unwrap();
    let reversed = ConceptPurposeLabels::new(entries.into_iter().rev().collect()).unwrap();
    assert_eq!(first.revision(), reversed.revision());
    assert_ne!(
        first.revision(),
        ConceptPurposeLabels::new(vec![(photo, "猫咪".into())])
            .unwrap()
            .revision()
    );
    for entries in [
        vec![],
        vec![(reaction.clone(), "".into())],
        vec![(reaction.clone(), "x".repeat(81))],
        vec![(reaction.clone(), "猫\n咪".into())],
        vec![(reaction.clone(), "猫咪".into()), (reaction, "重复".into())],
    ] {
        assert!(ConceptPurposeLabels::new(entries).is_err());
    }
}
