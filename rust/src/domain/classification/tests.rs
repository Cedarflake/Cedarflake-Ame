use super::*;

fn category(id: &str, label: &str, descriptions: &[&str]) -> CategoryDefinition {
    CategoryDefinition::new(
        CategoryId::new(id.to_owned()).unwrap(),
        label.to_owned(),
        descriptions.iter().map(|text| (*text).to_owned()).collect(),
    )
    .unwrap()
}

#[test]
fn display_rename_and_reordering_preserve_semantic_identity() {
    let photo = category(
        "photo",
        "照片",
        &["a landscape photograph", "an everyday photo"],
    );
    let meme = category("meme", "表情包", &["a reaction image"]);
    let original = Taxonomy::new(vec![photo.clone(), meme.clone()]).unwrap();
    let renamed = Taxonomy::new(vec![meme, photo.renamed("日常拍照".to_owned()).unwrap()]).unwrap();
    assert_eq!(original.revision(), renamed.revision());
    assert_eq!(
        original.description_revisions(),
        renamed.description_revisions()
    );
}

#[test]
fn reordered_descriptions_keep_the_same_category_revision() {
    let first = category("photo", "照片", &[" landscape ", "camera photo"]);
    let reordered = category("photo", "照片", &["camera photo", "landscape"]);
    assert_eq!(first.semantic_revision(), reordered.semantic_revision());
    assert_eq!(first.descriptions(), reordered.descriptions());
}

#[test]
fn semantic_boundary_edits_change_rules_without_changing_category_identity() {
    let original = category("meme", "表情包", &["a cartoon reaction sticker"]);
    let revised = category(
        "meme",
        "表情包",
        &["a photographic or drawn reaction image"],
    );
    assert_eq!(original.id(), revised.id());
    assert_ne!(original.semantic_revision(), revised.semantic_revision());
    assert_ne!(
        Taxonomy::new(vec![original]).unwrap().revision(),
        Taxonomy::new(vec![revised]).unwrap().revision()
    );
}

#[test]
fn active_set_changes_preserve_unchanged_description_keys() {
    let photo = category("photo", "照片", &["a photograph"]);
    let original = Taxonomy::new(vec![photo.clone()]).unwrap();
    let expanded = Taxonomy::new(vec![
        photo,
        category("documents", "文档", &["a scanned document"]),
    ])
    .unwrap();
    assert_ne!(original.revision(), expanded.revision());
    assert!(
        original
            .description_revisions()
            .is_subset(&expanded.description_revisions())
    );
    let retired = Taxonomy::new(vec![expanded.categories()[1].clone()]).unwrap();
    assert_eq!(retired.revision(), original.revision());
}

#[test]
fn taxonomy_does_not_restrict_the_product_to_comparison_labels() {
    let categories = (0..32)
        .map(|index| {
            category(
                &format!("custom-{index}"),
                &format!("自定义分类 {index}"),
                &[&format!("description {index}")],
            )
        })
        .collect();
    let taxonomy = Taxonomy::new(categories).unwrap();
    assert_eq!(taxonomy.categories().len(), 32);
    assert_eq!(taxonomy.description_revisions().len(), 32);
}

#[test]
fn rejects_empty_or_unbounded_active_schemes_and_duplicate_identity() {
    assert_eq!(
        Taxonomy::new(Vec::new()),
        Err(ClassificationError::NoCategories)
    );
    let photo = category("photo", "照片", &["a photo"]);
    assert_eq!(
        Taxonomy::new(vec![photo.clone(), photo]),
        Err(ClassificationError::DuplicateCategory)
    );
    let categories = (0..257)
        .map(|index| category(&format!("id-{index}"), "类别", &["description"]))
        .collect();
    assert_eq!(
        Taxonomy::new(categories),
        Err(ClassificationError::TooManyCategories)
    );
}

#[test]
fn rejects_malformed_identity_label_and_descriptions() {
    for id in ["", "../photo", "a b", "类别"] {
        assert_eq!(
            CategoryId::new(id.to_owned()),
            Err(ClassificationError::InvalidCategoryId)
        );
    }
    let id = CategoryId::new("custom-1".to_owned()).unwrap();
    for label in ["  ".to_owned(), "图".repeat(81), "bad\nlabel".to_owned()] {
        assert_eq!(
            CategoryDefinition::new(id.clone(), label, vec!["photo".to_owned()]),
            Err(ClassificationError::InvalidLabel)
        );
    }
    for descriptions in [
        vec![],
        vec![String::new()],
        vec!["photo".to_owned(), " photo ".to_owned()],
        vec!["x".repeat(1025)],
        vec!["x\0y".to_owned()],
        vec!["photo".to_owned(); 9],
    ] {
        assert_eq!(
            CategoryDefinition::new(id.clone(), "类别".to_owned(), descriptions),
            Err(ClassificationError::InvalidDescriptions)
        );
    }
}

#[test]
fn structured_revision_inputs_cannot_collide_by_field_concatenation() {
    let first = category("id", "A", &["ab", "c"]);
    let second = category("id", "A", &["a", "bc"]);
    assert_ne!(first.semantic_revision(), second.semantic_revision());
    assert_ne!(
        ClassificationRevision::description("a b"),
        ClassificationRevision::description("a  b")
    );
}

#[test]
fn normalizes_extreme_finite_values_without_overflow_or_underflow() {
    for input in [
        vec![f32::MAX, f32::MAX],
        vec![f32::MIN_POSITIVE, 0.0],
        vec![f32::from_bits(1), 0.0],
    ] {
        let embedding = Embedding::normalized(input).unwrap();
        assert!((embedding.cosine(&embedding).unwrap() - 1.0).abs() < 0.000001);
    }
}

#[test]
fn rejects_invalid_vectors_before_they_enter_matching() {
    for values in [
        vec![],
        vec![0.0, 0.0],
        vec![f32::NAN, 1.0],
        vec![f32::INFINITY],
        vec![1.0; 4097],
    ] {
        assert_eq!(
            Embedding::normalized(values),
            Err(ClassificationError::InvalidEmbedding)
        );
    }
    let first = Embedding::normalized(vec![1.0, 0.0]).unwrap();
    let second = Embedding::normalized(vec![1.0]).unwrap();
    assert_eq!(
        first.cosine(&second),
        Err(ClassificationError::InvalidEmbedding)
    );
}
