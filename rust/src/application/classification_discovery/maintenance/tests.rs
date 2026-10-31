use crate::domain::classification::discovery::CategoryNamingPurpose;

use super::*;

#[test]
fn identity_depends_on_concept_and_parent_not_parent_wording() {
    let parent = CategoryDefinition::new(
        CategoryId::new("photo".into()).unwrap(),
        "Photo".into(),
        vec!["a photograph".into()],
    )
    .unwrap();
    let changed = CategoryDefinition::new(
        parent.id().clone(),
        "Camera".into(),
        vec!["a real camera photograph".into()],
    )
    .unwrap();
    let purpose = |category: &CategoryDefinition| {
        CategoryNaming::WithinCategory(
            CategoryNamingPurpose::new(
                category,
                "{subject} photos".into(),
                "a photo of {subject}".into(),
            )
            .unwrap(),
        )
    };
    let concept = ConceptId::from_bytes([42; 32]);
    let original = managed_category_id(concept, &purpose(&parent)).unwrap();
    assert_eq!(
        original,
        managed_category_id(concept, &purpose(&changed)).unwrap()
    );
    assert_ne!(
        original,
        managed_category_id(concept, &CategoryNaming::Unconditioned).unwrap()
    );
    assert_ne!(
        original,
        managed_category_id(ConceptId::from_bytes([43; 32]), &purpose(&parent)).unwrap()
    );
    let other = CategoryDefinition::new(
        CategoryId::new("drawing".into()).unwrap(),
        "Drawing".into(),
        vec!["a drawing".into()],
    )
    .unwrap();
    assert_ne!(
        original,
        managed_category_id(concept, &purpose(&other)).unwrap()
    );
}
