use crate::domain::classification::{CategoryDefinition, CategoryId};

use super::*;

#[test]
fn managed_policy_preserves_the_retained_calibration_inputs() {
    let retained: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../docs/acceptance/fixtures/r5-naming-profile.json"
    ))
    .unwrap();
    let current: serde_json::Value = serde_json::from_str(PROFILE).unwrap();
    for key in [
        "cohort_minimum_pair_cosine",
        "nomination",
        "admission",
        "purposes",
    ] {
        assert_eq!(current[key], retained[key], "{key}");
    }
    let taxonomy = classification_candidate::initial_categories().unwrap();
    let policy = ManagedDiscoveryPolicy::for_taxonomy(&taxonomy).unwrap();
    assert!(policy.is_available());
    assert_eq!(policy.preparation().maximum_contents, 512);
    assert_eq!(policy.naming.len(), taxonomy.categories().len());
    for naming in &policy.naming {
        naming.validate(&taxonomy).unwrap();
    }
}

#[test]
fn semantic_edits_remove_automatic_naming_authority_but_label_changes_do_not() {
    let original = classification_candidate::initial_categories().unwrap();
    let renamed = Taxonomy::new(
        original
            .categories()
            .iter()
            .map(|category| {
                CategoryDefinition::new(
                    category.id().clone(),
                    "Renamed".into(),
                    category.descriptions().to_vec(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(
        ManagedDiscoveryPolicy::for_taxonomy(&renamed)
            .unwrap()
            .naming
            .len(),
        original.categories().len()
    );
    let replaced = Taxonomy::new(
        original
            .categories()
            .iter()
            .map(|category| {
                if category.id().as_str() == "photo" {
                    CategoryDefinition::new(
                        category.id().clone(),
                        "Custom meaning".into(),
                        vec!["a new category meaning".into()],
                    )
                    .unwrap()
                } else {
                    category.clone()
                }
            })
            .collect(),
    )
    .unwrap();
    let policy = ManagedDiscoveryPolicy::for_taxonomy(&replaced).unwrap();
    assert_eq!(policy.naming.len(), original.categories().len() - 1);
    assert!(
        !policy
            .naming
            .iter()
            .any(|naming| naming.includes(&CategoryId::new("photo".into()).unwrap()))
    );
}

#[test]
fn custom_taxonomies_do_not_inherit_unrelated_naming_purposes() {
    let taxonomy = Taxonomy::new(vec![
        CategoryDefinition::new(
            CategoryId::new("personal".into()).unwrap(),
            "Personal".into(),
            vec!["personal collection".into()],
        )
        .unwrap(),
    ])
    .unwrap();
    assert!(
        !ManagedDiscoveryPolicy::for_taxonomy(&taxonomy)
            .unwrap()
            .is_available()
    );
}
