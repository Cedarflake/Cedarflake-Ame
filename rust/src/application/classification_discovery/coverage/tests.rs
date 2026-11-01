use std::sync::atomic::Ordering;

use crate::domain::classification::maintenance::{
    CategoryCoverageEntry, CategoryCoveragePage, CategoryCoverageSnapshot,
};
use crate::domain::classification::work::{
    ClassificationDispatchMode, ClassificationDispatchState,
};
use crate::domain::classification::{
    CategoryDefinition, CategoryId, CategoryScore, ClassificationRecordId, ClassificationRun,
    ModelCompatibility, PredictionEvidence, RankedPrediction, ReviewRecommendation, Taxonomy,
};
use crate::domain::classification_catalog::ClassificationCatalogState;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

use super::*;

#[derive(Clone, Copy)]
enum Fault {
    None,
    Duplicate,
    EmptyMore,
    Oversized,
    WrongContent,
    FinalChange,
    Cancel,
}

struct MemoryCoverage {
    snapshot: CategoryCoverageSnapshot,
    entries: Vec<CategoryCoverageEntry>,
    fault: Fault,
}

fn id(value: &str) -> ClassificationRecordId {
    ClassificationRecordId::new(value.into()).unwrap()
}
fn category(value: &str) -> CategoryId {
    CategoryId::new(value.into()).unwrap()
}

fn fixture() -> MemoryCoverage {
    let taxonomy = Taxonomy::new(
        ["drawing", "photo", "unused"]
            .map(|name| {
                CategoryDefinition::new(
                    category(name),
                    name.into(),
                    vec![format!("image of {name}")],
                )
                .unwrap()
            })
            .into(),
    )
    .unwrap();
    let model = ModelCompatibility::new(
        ClassificationRevision::from_bytes([1; 32]),
        ClassificationRevision::from_bytes([2; 32]),
        ClassificationRevision::from_bytes([3; 32]),
        2,
    )
    .unwrap();
    let run = ClassificationRun::new(id("run"), model, taxonomy.revision(), None).unwrap();
    let entries = (1..=6)
        .map(|index| {
            let content = ClassificationContent {
                scheme: id("fixture"),
                digest: [index; 32],
                byte_length: 20,
            };
            let winner = if index == 6 { "photo" } else { "drawing" };
            let prediction = PredictionEvidence::new(
                id(&format!("p{index}")),
                run.clone(),
                content.clone(),
                RankedPrediction::new(
                    taxonomy.revision(),
                    model,
                    taxonomy
                        .categories()
                        .iter()
                        .map(|definition| CategoryScore {
                            category_id: definition.id().clone(),
                            similarity: if definition.id().as_str() == winner {
                                0.8
                            } else {
                                0.1
                            },
                        })
                        .collect(),
                )
                .unwrap(),
                None,
                ReviewRecommendation::Uncalibrated,
            )
            .unwrap();
            CategoryCoverageEntry {
                content,
                prediction: Some(prediction),
            }
        })
        .collect();
    MemoryCoverage {
        snapshot: CategoryCoverageSnapshot {
            catalog_revision: 5,
            configuration: ClassificationConfigurationSnapshot {
                catalog: ClassificationCatalogState {
                    revision: 1,
                    taxonomy: Some(taxonomy),
                    configuration: Some(run.configuration()),
                    active_run_id: Some("run".into()),
                },
                dispatch: ClassificationDispatchState {
                    revision: 1,
                    epoch: 1,
                    mode: ClassificationDispatchMode::Automatic,
                    run_id: Some(run.id().clone()),
                    session_id: None,
                },
            },
            model,
            unverified_sources: false,
        },
        entries,
        fault: Fault::None,
    }
}

impl CategoryCoverageRepository for MemoryCoverage {
    fn begin_category_coverage(
        &self,
        _: &Arc<AtomicBool>,
    ) -> Result<CategoryCoverageSnapshot, ScanError> {
        Ok(self.snapshot.clone())
    }

    fn read_category_coverage(
        &self,
        _: &CategoryCoverageSnapshot,
        after: Option<&ClassificationContent>,
        limit: usize,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryCoveragePage, ScanError> {
        let start = self
            .entries
            .partition_point(|entry| after.is_some_and(|after| key(&entry.content) <= key(after)));
        let end = (start + limit).min(self.entries.len());
        let mut entries = self.entries[start..end].to_vec();
        let mut has_more = end < self.entries.len();
        match self.fault {
            Fault::Duplicate => entries[1] = entries[0].clone(),
            Fault::EmptyMore => {
                entries.clear();
                has_more = true;
            }
            Fault::Oversized => entries = self.entries.clone(),
            Fault::WrongContent => entries[0].content.digest = [0; 32],
            Fault::Cancel => cancelled.store(true, Ordering::Release),
            Fault::None | Fault::FinalChange => {}
        }
        Ok(CategoryCoveragePage { entries, has_more })
    }

    fn validate_category_coverage(
        &self,
        _: &CategoryCoverageSnapshot,
        _: &Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        if matches!(self.fault, Fault::FinalChange) {
            Err(ScanError::new("classification_changed", "Changed"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn coverage_is_page_independent_and_retains_only_bounded_content_examples() {
    let source = fixture();
    let cancelled = Arc::new(AtomicBool::new(false));
    let expected = survey_category_coverage(&source, 1, 2, &cancelled).unwrap();
    assert!(expected.is_complete());
    assert_eq!(expected.classified_contents, 6);
    assert_eq!(expected.categories[&category("drawing")].contents, 5);
    assert_eq!(
        expected.categories[&category("drawing")]
            .retained_members
            .len(),
        2
    );
    assert_eq!(expected.categories[&category("photo")].contents, 1);
    assert_eq!(expected.categories[&category("unused")].contents, 0);
    assert!(
        expected.categories[&category("drawing")]
            .retained_members
            .iter()
            .all(|member| member.alternative.is_none())
    );
    for size in [2, 4, 64] {
        let actual = survey_category_coverage(&source, size, 2, &cancelled).unwrap();
        assert_eq!(actual.evidence, expected.evidence);
        assert_eq!(actual.categories, expected.categories);
    }
    assert_ne!(
        survey_category_coverage(&source, 2, 1, &cancelled)
            .unwrap()
            .evidence,
        expected.evidence
    );
}

#[test]
fn missing_predictions_and_unverified_sources_remain_incomplete() {
    let mut source = fixture();
    let cancelled = Arc::new(AtomicBool::new(false));
    source.entries[0].prediction = None;
    let report = survey_category_coverage(&source, 2, 1, &cancelled).unwrap();
    assert!(!report.is_complete());
    assert_eq!(report.unclassified_contents, 1);
    assert_eq!(report.classified_contents, 5);
    source.entries.clear();
    source.snapshot.unverified_sources = true;
    let report = survey_category_coverage(&source, 2, 1, &cancelled).unwrap();
    assert!(!report.is_complete());
    assert_eq!(report.classified_contents, 0);
}

#[test]
fn malformed_pages_and_changed_publication_return_no_partial_report() {
    for fault in [
        Fault::Duplicate,
        Fault::EmptyMore,
        Fault::Oversized,
        Fault::WrongContent,
        Fault::FinalChange,
        Fault::Cancel,
    ] {
        let mut source = fixture();
        source.fault = fault;
        let error =
            survey_category_coverage(&source, 2, 2, &Arc::new(AtomicBool::new(false))).unwrap_err();
        assert_eq!(
            error.code,
            match fault {
                Fault::FinalChange => "classification_changed",
                Fault::Cancel => "classification_cancelled",
                _ => "classification_coverage_invalid",
            }
        );
    }
}

#[test]
fn invalid_bounds_and_cancelled_start_reject_before_reading() {
    let source = fixture();
    for (page, retained) in [(0, 1), (65, 1), (1, 0), (1, 65)] {
        assert_eq!(
            survey_category_coverage(&source, page, retained, &Arc::new(AtomicBool::new(false)))
                .unwrap_err()
                .code,
            "classification_coverage_invalid"
        );
    }
    assert_eq!(
        survey_category_coverage(&source, 2, 2, &Arc::new(AtomicBool::new(true)))
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
}

#[test]
fn compatible_configuration_requires_every_category_and_matching_content() {
    let mut source = fixture();
    let original = source.entries[0].prediction.as_ref().unwrap();
    source.entries[0].prediction = Some(
        PredictionEvidence::new(
            id("incomplete"),
            original.run().clone(),
            original.content().clone(),
            RankedPrediction::new(
                original.run().primary_rules(),
                original.run().model(),
                original.initial().categories()[..2].to_vec(),
            )
            .unwrap(),
            None,
            ReviewRecommendation::Uncalibrated,
        )
        .unwrap(),
    );
    assert_eq!(
        survey_category_coverage(&source, 2, 2, &Arc::new(AtomicBool::new(false)))
            .unwrap_err()
            .code,
        "classification_coverage_invalid"
    );
}

#[test]
fn coverage_counts_the_refined_suggestion_when_primary_and_refinement_disagree() {
    let mut source = fixture();
    let original = source.entries[0].prediction.as_ref().unwrap().run();
    let refinement = ClassificationRevision::from_bytes([9; 32]);
    let run = ClassificationRun::new(
        id("refined"),
        original.model(),
        original.primary_rules(),
        Some(crate::domain::classification::RefinementIdentity {
            rules: refinement,
            profile: ClassificationRevision::from_bytes([10; 32]),
        }),
    )
    .unwrap();
    source.snapshot.configuration.catalog.configuration = Some(run.configuration());
    source.snapshot.configuration.catalog.active_run_id = Some("refined".into());
    source.snapshot.configuration.dispatch.run_id = Some(run.id().clone());
    for entry in &mut source.entries {
        let original = entry.prediction.take().unwrap();
        let scores = original
            .initial()
            .categories()
            .iter()
            .map(|score| CategoryScore {
                category_id: score.category_id.clone(),
                similarity: if score.category_id.as_str() == "photo" {
                    0.9
                } else if score.category_id.as_str() == "drawing" {
                    0.5
                } else {
                    0.1
                },
            })
            .collect();
        entry.prediction = Some(
            PredictionEvidence::new(
                original.id().clone(),
                run.clone(),
                entry.content.clone(),
                original.initial().clone(),
                Some(RankedPrediction::new(refinement, run.model(), scores).unwrap()),
                ReviewRecommendation::RoutineSampling,
            )
            .unwrap(),
        );
    }
    let result =
        survey_category_coverage(&source, 2, 1, &Arc::new(AtomicBool::new(false))).unwrap();
    assert!(result.is_complete());
    assert_eq!(result.categories[&category("photo")].contents, 6);
    assert_eq!(result.categories[&category("drawing")].contents, 0);
    let alternative = result.categories[&category("photo")].retained_members[0]
        .alternative
        .as_ref()
        .unwrap();
    assert_eq!(alternative.category_id, category("drawing"));
    assert_eq!(alternative.similarity, 0.5);
}
