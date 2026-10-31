use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::adapters::SqliteClassificationFeatureCache;
use crate::application::classification_analysis::analyze;
use crate::application::classification_matching::CategoryMatcher;
use crate::application::classification_residency::{
    ClassificationEncoderResidency, MonotonicResidencyClock,
};
use crate::application::classification_screening::ScreeningClassifier;
use crate::domain::classification::{ClassificationContent, Embedding, ImageFeature, TextFeature};
use crate::domain::exact_duplicates::FingerprintCandidate;
use crate::ports::classification_analysis::{ClassificationImageLease, ClassificationImageReader};
use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};
use crate::ports::classification_features::ClassificationFeatureRepository;

use super::*;

#[derive(Clone, Copy, Default)]
enum Effect {
    #[default]
    None,
    CancelDecode,
    CancelInference,
    PauseInference,
    ChangeSource,
    FailRevalidation,
}

#[derive(Default)]
struct Observed {
    opens: Cell<u32>,
    decodes: Cell<u32>,
    inferences: Cell<u32>,
    completed_at_release: RefCell<Vec<i64>>,
    effect: Cell<Effect>,
}

struct Reader {
    database: PathBuf,
    observed: Rc<Observed>,
}

struct Source {
    database: PathBuf,
    observed: Rc<Observed>,
}

impl ClassificationImageReader for Reader {
    type Lease = Source;

    fn open(
        &self,
        _: &FingerprintCandidate,
        _: &ClassificationContent,
        _: &AtomicBool,
    ) -> Result<Source, ScanError> {
        self.observed.opens.set(self.observed.opens.get() + 1);
        Ok(Source {
            database: self.database.clone(),
            observed: self.observed.clone(),
        })
    }
}

impl ClassificationImageLease for Source {
    fn prepare_pixels(&mut self, cancelled: &AtomicBool) -> Result<Vec<f32>, ScanError> {
        self.observed.decodes.set(self.observed.decodes.get() + 1);
        if matches!(self.observed.effect.get(), Effect::CancelDecode) {
            cancelled.store(true, Ordering::Release);
        }
        Ok(vec![1.0, 0.0])
    }

    fn revalidate(&self) -> Result<(), ScanError> {
        if matches!(self.observed.effect.get(), Effect::FailRevalidation) {
            Err(ScanError::new("source_changed", "The source changed"))
        } else {
            Ok(())
        }
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        let connection = rusqlite::Connection::open(&self.database).unwrap();
        let completed = connection.query_row(
            "SELECT count(*) FROM classification_work_items WHERE kind = 'content' AND phase = 'done'", [], |row| row.get(0),
        ).unwrap();
        self.observed
            .completed_at_release
            .borrow_mut()
            .push(completed);
    }
}

struct Factory {
    database: PathBuf,
    model: ModelCompatibility,
    observed: Rc<Observed>,
}

struct Encoder {
    database: PathBuf,
    model: ModelCompatibility,
    observed: Rc<Observed>,
    retired: bool,
}

impl ClassificationEncoderFactory for Factory {
    type Encoder = Encoder;
    fn compatibility(&self) -> ModelCompatibility {
        self.model
    }
    fn create(&self, kind: ClassificationEncoderKind) -> Result<Encoder, ScanError> {
        assert_eq!(kind, ClassificationEncoderKind::Image);
        Ok(Encoder {
            database: self.database.clone(),
            model: self.model,
            observed: self.observed.clone(),
            retired: false,
        })
    }
}

impl ClassificationEncoder for Encoder {
    fn initialize(&mut self, _: &AtomicBool) -> Result<(), ScanError> {
        Ok(())
    }
    fn encode_prepared_image(
        &mut self,
        _: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        self.observed
            .inferences
            .set(self.observed.inferences.get() + 1);
        match self.observed.effect.get() {
            Effect::CancelInference => cancelled.store(true, Ordering::Release),
            Effect::PauseInference => {
                let mut catalog = SqliteCatalog::open(self.database.clone())?;
                change(&mut catalog, ClassificationDispatchCommand::Pause);
            }
            Effect::ChangeSource => {
                let catalog = SqliteCatalog::open(self.database.clone())?;
                catalog.connection.execute("UPDATE asset_locations SET source_generation = source_generation + 100 WHERE location_id = 'a'", []).unwrap();
            }
            _ => {}
        }
        Ok(ImageFeature {
            identity: self.model.image_identity(),
            embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
        })
    }
    fn encode_description(&mut self, _: &str, _: &AtomicBool) -> Result<TextFeature, ScanError> {
        panic!("matcher is already prepared")
    }
    fn retire(&mut self) -> Result<(), ScanError> {
        self.retired = true;
        Ok(())
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

fn classifier(run: &ClassificationRun) -> ScreeningClassifier {
    let taxonomy = taxonomy();
    let features = taxonomy
        .categories()
        .iter()
        .enumerate()
        .map(|(index, category)| TextFeature {
            identity: run.model().text_identity(),
            description: ClassificationRevision::description(&category.descriptions()[0]),
            embedding: Embedding::normalized(if index == 0 {
                vec![1.0, 0.0]
            } else {
                vec![0.0, 1.0]
            })
            .unwrap(),
        })
        .collect();
    ScreeningClassifier::uncalibrated(
        CategoryMatcher::new(taxonomy, run.model(), features).unwrap(),
    )
}

fn claimed(
    catalog: &mut SqliteCatalog,
    session: &ClassificationRecordId,
) -> ClassificationWorkLease {
    request(
        catalog,
        session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    verify(catalog, "a", 'a');
    let source = catalog
        .claim_classification_work(session, 10)
        .unwrap()
        .unwrap();
    catalog.finish_classification_verification(&source).unwrap();
    catalog
        .claim_classification_work(session, 10)
        .unwrap()
        .unwrap()
}

fn residency(
    directory: &tempfile::TempDir,
    run: &ClassificationRun,
    observed: Rc<Observed>,
) -> ClassificationEncoderResidency<Factory, MonotonicResidencyClock> {
    ClassificationEncoderResidency::new(
        Factory {
            database: directory.path().join("catalog.sqlite3"),
            model: run.model(),
            observed,
        },
        MonotonicResidencyClock::default(),
        Duration::from_secs(60),
    )
}

#[test]
fn inference_and_checkpoint_publish_together_and_completed_cache_survives_restart() {
    let (directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let lease = claimed(&mut catalog, &session);
    let observed = Rc::new(Observed::default());
    let reader = Reader {
        database: directory.path().join("catalog.sqlite3"),
        observed: observed.clone(),
    };
    let mut residency = residency(&directory, &run, observed.clone());
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    let result = analyze(
        &mut catalog,
        &reader,
        &lease,
        &classifier(&run),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        result,
        ClassificationAnalysisResult::Completed {
            used_cached_feature: false
        }
    );
    assert_eq!(observed.inferences.get(), 1);
    assert_eq!(*observed.completed_at_release.borrow(), vec![1]);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
    let ClassificationWorkTarget::AnalyzeContent { content, .. } = &lease.target else {
        panic!()
    };
    drop(cache);
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    assert!(
        cache
            .load_image_feature(run.model(), content)
            .unwrap()
            .is_some()
    );

    // A retry after lost acknowledgement must leave the single completed prediction intact.
    assert_eq!(
        analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &AtomicBool::new(false)
        )
        .unwrap(),
        ClassificationAnalysisResult::Superseded
    );
    assert_eq!(observed.opens.get(), 1);
    assert_eq!(observed.inferences.get(), 1);
}

#[test]
fn cached_features_skip_decode_and_inference_but_still_hold_source_authority() {
    let (directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let lease = claimed(&mut catalog, &session);
    let ClassificationWorkTarget::AnalyzeContent { content, .. } = &lease.target else {
        panic!()
    };
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    cache
        .store_image_feature(
            run.model(),
            content,
            &ImageFeature {
                identity: run.model().image_identity(),
                embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
            },
        )
        .unwrap();
    let observed = Rc::new(Observed::default());
    let reader = Reader {
        database: directory.path().join("catalog.sqlite3"),
        observed: observed.clone(),
    };
    let mut residency = residency(&directory, &run, observed.clone());
    assert_eq!(
        analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &AtomicBool::new(false)
        )
        .unwrap(),
        ClassificationAnalysisResult::Completed {
            used_cached_feature: true
        }
    );
    assert_eq!(
        (
            observed.opens.get(),
            observed.decodes.get(),
            observed.inferences.get()
        ),
        (1, 0, 0)
    );
    assert_eq!(*observed.completed_at_release.borrow(), vec![1]);
    assert!(residency.resident_kind().is_none());
}

#[test]
fn a_different_image_pipeline_is_rejected_before_opening_the_source() {
    let (directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let lease = claimed(&mut catalog, &session);
    let model = ModelCompatibility::new(
        run.model().image_identity().embedding_space,
        ClassificationRevision::from_bytes([99; 32]),
        run.model().text_identity().pipeline,
        run.model().dimensions(),
    )
    .unwrap();
    let changed =
        ClassificationRun::new(run.id().clone(), model, run.primary_rules(), None).unwrap();
    let observed = Rc::new(Observed::default());
    let reader = Reader {
        database: directory.path().join("catalog.sqlite3"),
        observed: observed.clone(),
    };
    let mut residency = residency(&directory, &run, observed.clone());
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    let error = analyze(
        &mut catalog,
        &reader,
        &lease,
        &classifier(&changed),
        &mut residency,
        &mut cache,
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert_eq!(error.code, "classification_analysis_mismatch");
    assert_eq!(observed.opens.get(), 0);
    assert_eq!(observed.inferences.get(), 0);
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        0
    );
}

#[test]
fn cancellation_pause_source_change_and_validation_failure_publish_nothing() {
    for effect in [
        Effect::CancelDecode,
        Effect::CancelInference,
        Effect::PauseInference,
        Effect::ChangeSource,
        Effect::FailRevalidation,
    ] {
        let (directory, mut catalog) = fixture();
        let (run, session) = start(&mut catalog);
        let lease = claimed(&mut catalog, &session);
        let observed = Rc::new(Observed::default());
        observed.effect.set(effect);
        let reader = Reader {
            database: directory.path().join("catalog.sqlite3"),
            observed: observed.clone(),
        };
        let mut residency = residency(&directory, &run, observed.clone());
        let mut cache =
            SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
        let cancelled = AtomicBool::new(false);
        let result = analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &cancelled,
        );
        match effect {
            Effect::CancelDecode | Effect::CancelInference => {
                assert_eq!(result.unwrap(), ClassificationAnalysisResult::Cancelled)
            }
            Effect::FailRevalidation => assert_eq!(result.unwrap_err().code, "source_changed"),
            _ => assert_eq!(result.unwrap(), ClassificationAnalysisResult::Superseded),
        }
        assert_eq!(cache.feature_cache_stats().unwrap().entries, 0);
        assert_eq!(
            scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
            0
        );
        assert_eq!(*observed.completed_at_release.borrow(), vec![0]);
        if matches!(effect, Effect::CancelDecode) {
            assert_eq!(observed.inferences.get(), 0);
        }
    }
}

#[test]
fn failed_checkpoint_rolls_back_prediction_and_retry_reuses_the_feature() {
    let (directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let lease = claimed(&mut catalog, &session);
    catalog.connection.execute_batch(
        "CREATE TEMP TRIGGER reject_content_completion BEFORE UPDATE OF phase ON classification_work_items
         WHEN NEW.kind = 'content' AND NEW.phase = 'done' BEGIN SELECT RAISE(ABORT, 'checkpoint fixture'); END;",
    ).unwrap();
    let observed = Rc::new(Observed::default());
    let reader = Reader {
        database: directory.path().join("catalog.sqlite3"),
        observed: observed.clone(),
    };
    let mut residency = residency(&directory, &run, observed.clone());
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(
        analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &cancelled
        )
        .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        0
    );
    assert_eq!(cache.feature_cache_stats().unwrap().entries, 1);
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_content_completion")
        .unwrap();
    assert_eq!(
        analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &cancelled
        )
        .unwrap(),
        ClassificationAnalysisResult::Completed {
            used_cached_feature: true
        }
    );
    assert_eq!(observed.inferences.get(), 1);
    assert_eq!(*observed.completed_at_release.borrow(), vec![0, 1]);
}

#[test]
fn cancellation_during_prediction_insert_rolls_back_the_completed_checkpoint() {
    let (directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let lease = claimed(&mut catalog, &session);
    let cancelled = std::sync::Arc::new(AtomicBool::new(false));
    let hook_cancelled = cancelled.clone();
    catalog
        .connection
        .update_hook(Some(
            move |_: rusqlite::hooks::Action, _: &str, table: &str, _: i64| {
                if table == "classification_predictions" {
                    hook_cancelled.store(true, Ordering::Release);
                }
            },
        ))
        .unwrap();
    let observed = Rc::new(Observed::default());
    let reader = Reader {
        database: directory.path().join("catalog.sqlite3"),
        observed: observed.clone(),
    };
    let mut residency = residency(&directory, &run, observed.clone());
    let mut cache =
        SqliteClassificationFeatureCache::open(&directory.path().join("features")).unwrap();
    assert_eq!(
        analyze(
            &mut catalog,
            &reader,
            &lease,
            &classifier(&run),
            &mut residency,
            &mut cache,
            &cancelled
        )
        .unwrap(),
        ClassificationAnalysisResult::Cancelled
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE kind = 'content' AND phase = 'running'"
        ),
        1
    );
    assert_eq!(*observed.completed_at_release.borrow(), vec![0]);
}
