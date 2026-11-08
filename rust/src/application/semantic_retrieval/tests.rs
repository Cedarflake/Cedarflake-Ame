use super::*;

fn selected(relevance: SemanticRelevance, scores: &[f32]) -> Vec<String> {
    let mut ranked = scores
        .iter()
        .enumerate()
        .map(|(index, score)| {
            (
                *score,
                OrganizationSelection {
                    asset_id: format!("asset-{index}"),
                    location_id: format!("location-{index}"),
                    source_generation: index as u64 + 1,
                },
            )
        })
        .collect();
    relevance.retain(&mut ranked);
    ranked
        .into_iter()
        .map(|(_, source)| source.location_id)
        .collect()
}

#[test]
fn descriptions_exclude_weak_results_without_padding_the_gallery() {
    assert_eq!(
        selected(SemanticRelevance::Description, &[0.18, 0.17, 0.14, 0.08]),
        ["location-0", "location-1"]
    );
    assert_eq!(
        selected(SemanticRelevance::Description, &[0.10, 0.095, 0.085]),
        ["location-0", "location-1"]
    );
    assert!(selected(SemanticRelevance::Description, &[0.08, 0.07]).is_empty());
    assert!(selected(SemanticRelevance::Description, &[]).is_empty());
}

#[test]
fn reference_images_use_their_own_relevance_scale() {
    assert_eq!(
        selected(SemanticRelevance::Reference, &[0.83, 0.80, 0.74, 0.60]),
        ["location-0", "location-1"]
    );
    assert!(selected(SemanticRelevance::Reference, &[0.60, 0.55]).is_empty());
}
