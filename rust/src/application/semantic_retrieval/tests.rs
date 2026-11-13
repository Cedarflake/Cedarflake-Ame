use super::*;

fn selected(relevance: SemanticRelevance, scores: &[f32]) -> Vec<String> {
    let ranked: Vec<_> = scores
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
        .filter(|(score, _)| relevance.accepts(*score))
        .collect();
    ranked
        .into_iter()
        .map(|(_, source)| source.location_id)
        .collect()
}

#[test]
fn descriptions_exclude_weak_results_without_padding_the_gallery() {
    assert_eq!(
        selected(SemanticRelevance::Description, &[0.18, 0.17, 0.14, 0.08]),
        ["location-0", "location-1", "location-2"]
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
        ["location-0", "location-1", "location-2"]
    );
    assert!(selected(SemanticRelevance::Reference, &[0.60, 0.55]).is_empty());
}

#[test]
fn expanding_scope_with_a_stronger_match_keeps_existing_matches() {
    for (relevance, narrower, broader) in [
        (
            SemanticRelevance::Description,
            vec![0.12, 0.11, 0.08],
            vec![0.30, 0.12, 0.11, 0.08],
        ),
        (
            SemanticRelevance::Reference,
            vec![0.72, 0.70, 0.60],
            vec![0.95, 0.72, 0.70, 0.60],
        ),
    ] {
        assert_eq!(selected(relevance, &narrower), ["location-0", "location-1"]);
        assert_eq!(
            selected(relevance, &broader),
            ["location-0", "location-1", "location-2"]
        );
    }
}

#[test]
fn retrieval_retains_all_matches_across_candidate_windows() {
    use crate::adapters::SqliteSemanticFeatureStore;
    use crate::domain::image_features::ImageFeature;
    use crate::domain::semantic_discovery::ImageContentIdentity;

    let directory = tempfile::tempdir().unwrap();
    let mut features = SqliteSemanticFeatureStore::open(directory.path()).unwrap();
    let revision = AnalysisRevision::from_bytes([1; 32]);
    let model = ModelCompatibility::new(revision, revision, revision, 2).unwrap();
    let matching = ImageContentIdentity::new("fixture".to_owned(), [1; 32], 1).unwrap();
    let weak = ImageContentIdentity::new("fixture".to_owned(), [2; 32], 1).unwrap();
    for (content, vector) in [(&matching, vec![1.0, 0.0]), (&weak, vec![0.0, 1.0])] {
        features
            .publish_image(
                model,
                content,
                &ImageFeature {
                    identity: model.image_identity(),
                    embedding: Embedding::normalized(vector).unwrap(),
                },
            )
            .unwrap();
    }
    let catalog = CandidateCatalog { matching, weak };
    let query = GalleryQuery::default();
    let embedding = Embedding::normalized(vec![1.0, 0.0]).unwrap();
    let manifest = retrieve(
        &catalog,
        &features,
        SemanticRetrievalQuery {
            scope: &query,
            model,
            embedding: &embedding,
            identity: revision,
            exclude: None,
            relevance: SemanticRelevance::Description,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(manifest.eligible_items, 770);
    assert_eq!(manifest.indexed_items, 770);
    assert_eq!(manifest.items.len(), 769);
    assert_eq!(manifest.items[0].location_id, "location-0000");
    assert_eq!(manifest.items[768].location_id, "location-0768");
}

struct CandidateCatalog {
    matching: crate::domain::semantic_discovery::ImageContentIdentity,
    weak: crate::domain::semantic_discovery::ImageContentIdentity,
}

impl crate::ports::semantic_catalog::SemanticCatalogSnapshot for &CandidateCatalog {
    fn candidates(
        &self,
        _: &GalleryQuery,
        _: &str,
    ) -> Result<crate::ports::semantic_catalog::SemanticCandidateWindow, ScanError> {
        unreachable!("retrieval uses the consistent streaming snapshot")
    }

    fn scan(
        &self,
        _: &GalleryQuery,
        consume: &mut dyn FnMut(
            Vec<crate::ports::semantic_catalog::SemanticCandidate>,
        ) -> Result<(), ScanError>,
    ) -> Result<u64, ScanError> {
        for start in (0..770).step_by(SEMANTIC_CANDIDATE_WINDOW) {
            let end = (start + SEMANTIC_CANDIDATE_WINDOW).min(770);
            consume(
                (start..end)
                    .map(|index| crate::ports::semantic_catalog::SemanticCandidate {
                        source: OrganizationSelection {
                            asset_id: format!("asset-{index:04}"),
                            location_id: format!("location-{index:04}"),
                            source_generation: index as u64 + 1,
                        },
                        content: Some(if index == 769 {
                            self.weak.clone()
                        } else {
                            self.matching.clone()
                        }),
                    })
                    .collect(),
            )?;
        }
        Ok(1)
    }
}

impl SemanticCatalog for CandidateCatalog {
    fn semantic_snapshot(
        &self,
    ) -> Result<Box<dyn crate::ports::semantic_catalog::SemanticCatalogSnapshot + '_>, ScanError>
    {
        Ok(Box::new(self))
    }

    fn publish_semantic_content(&mut self, _: &OrganizationSelection) -> Result<bool, ScanError> {
        unreachable!("retrieval does not publish content")
    }

    fn semantic_source(
        &self,
        _: &OrganizationSelection,
    ) -> Result<Option<crate::domain::exact_duplicates::FingerprintCandidate>, ScanError> {
        unreachable!("retrieval does not read source files")
    }

    fn semantic_content(
        &self,
        _: &OrganizationSelection,
    ) -> Result<Option<crate::domain::semantic_discovery::ImageContentIdentity>, ScanError> {
        unreachable!("content identity comes from the candidate snapshot")
    }
}
