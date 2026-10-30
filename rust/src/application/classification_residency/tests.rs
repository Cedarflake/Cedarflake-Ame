use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::domain::classification::{ClassificationRevision, Embedding};

use super::*;

#[derive(Clone, Copy, Default)]
enum Retirement {
    #[default]
    Complete,
    Fail,
    Incomplete,
}

#[derive(Default)]
struct Scenario {
    events: Vec<String>,
    active: usize,
    retirement: Retirement,
    fail_initialization: bool,
    fail_inference: bool,
    cancel_after_inference: bool,
    inference_duration: Duration,
}

#[derive(Clone, Default)]
struct Clock(Rc<Cell<Duration>>);

impl ResidencyClock for Clock {
    fn elapsed(&self) -> Duration {
        self.0.get()
    }
}

struct Factory(Rc<RefCell<Scenario>>, Clock);

impl ClassificationEncoderFactory for Factory {
    type Encoder = Encoder;

    fn compatibility(&self) -> ModelCompatibility {
        let revision = ClassificationRevision::description("test encoder");
        ModelCompatibility::new(revision, revision, revision, 2).unwrap()
    }

    fn create(&self, kind: ClassificationEncoderKind) -> Result<Encoder, ScanError> {
        Ok(Encoder {
            scenario: Rc::clone(&self.0),
            clock: self.1.clone(),
            kind,
            model: self.compatibility(),
            retired: false,
        })
    }
}

struct Encoder {
    scenario: Rc<RefCell<Scenario>>,
    clock: Clock,
    kind: ClassificationEncoderKind,
    model: ModelCompatibility,
    retired: bool,
}

impl Encoder {
    fn infer(&self, cancelled: &AtomicBool) -> Result<Embedding, ScanError> {
        let scenario = self.scenario.borrow();
        self.clock
            .0
            .set(self.clock.elapsed() + scenario.inference_duration);
        if scenario.cancel_after_inference {
            cancelled.store(true, Ordering::Release);
        }
        if scenario.fail_inference {
            return Err(ScanError::new("failed", "injected inference failure"));
        }
        Embedding::normalized(vec![1.0, 0.0])
            .map_err(|error| ScanError::new("invalid", error.to_string()))
    }
}

impl ClassificationEncoder for Encoder {
    fn initialize(&mut self, _: &AtomicBool) -> Result<(), ScanError> {
        let mut scenario = self.scenario.borrow_mut();
        assert_eq!(scenario.active, 0, "workers must never overlap");
        scenario.active += 1;
        scenario.events.push(format!("start:{:?}", self.kind));
        if scenario.fail_initialization {
            return Err(ScanError::new(
                "initialization",
                "injected model load failure",
            ));
        }
        Ok(())
    }

    fn encode_prepared_image(
        &mut self,
        _: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        assert_eq!(self.kind, ClassificationEncoderKind::Image);
        Ok(ImageFeature {
            identity: self.model.image_identity(),
            embedding: self.infer(cancelled)?,
        })
    }

    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        assert_eq!(self.kind, ClassificationEncoderKind::Text);
        Ok(TextFeature {
            identity: self.model.text_identity(),
            description: ClassificationRevision::description(description),
            embedding: self.infer(cancelled)?,
        })
    }

    fn retire(&mut self) -> Result<(), ScanError> {
        let mut scenario = self.scenario.borrow_mut();
        scenario.events.push(format!("retire:{:?}", self.kind));
        match scenario.retirement {
            Retirement::Complete => {
                if !self.retired {
                    scenario.active -= 1;
                    self.retired = true;
                }
                Ok(())
            }
            Retirement::Fail => Err(ScanError::new("retire", "injected retirement failure")),
            Retirement::Incomplete => Ok(()),
        }
    }

    fn is_retired(&self) -> bool {
        self.retired
    }
}

type Owner = ClassificationEncoderResidency<Factory, Clock>;

#[test]
fn failed_model_load_retains_its_process_until_retirement_is_confirmed() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    scenario.borrow_mut().fail_initialization = true;
    scenario.borrow_mut().retirement = Retirement::Fail;
    assert!(owner.encode_prepared_image(&[], &cancelled).is_err());
    assert_eq!(scenario.borrow().active, 1);
    scenario.borrow_mut().fail_initialization = false;
    assert!(owner.encode_description("replacement", &cancelled).is_err());
    assert_eq!(scenario.borrow().active, 1);
    scenario.borrow_mut().retirement = Retirement::Complete;
    owner.encode_description("replacement", &cancelled).unwrap();
    assert_eq!(scenario.borrow().active, 1);
    owner.retire().unwrap();
    assert_eq!(scenario.borrow().active, 0);
}

fn fixture() -> (Owner, Rc<RefCell<Scenario>>, Clock) {
    let scenario = Rc::new(RefCell::new(Scenario::default()));
    let clock = Clock::default();
    let owner = Owner::new(
        Factory(Rc::clone(&scenario), clock.clone()),
        clock.clone(),
        Duration::from_secs(60),
    );
    (owner, scenario, clock)
}

#[test]
fn image_residency_expires_from_the_last_completed_inference() {
    let (mut owner, scenario, clock) = fixture();
    let cancelled = AtomicBool::new(false);
    scenario.borrow_mut().inference_duration = Duration::from_secs(5);
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    clock.0.set(Duration::from_secs(64));
    owner.retire_if_idle().unwrap();
    assert_eq!(scenario.borrow().active, 1);
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    assert_eq!(scenario.borrow().events, ["start:Image"]);
    clock.0.set(Duration::from_secs(129));
    owner.retire_if_idle().unwrap();
    assert_eq!(owner.resident_kind(), None);
    assert_eq!(scenario.borrow().active, 0);
}

#[test]
fn text_batch_reuses_one_encoder_and_releases_it_before_image_work() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    owner.encode_description("one", &cancelled).unwrap();
    owner.encode_description("two", &cancelled).unwrap();
    owner.finish_descriptions().unwrap();
    assert_eq!(scenario.borrow().active, 0);
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    owner.finish_descriptions().unwrap();
    assert_eq!(
        scenario.borrow().events,
        ["start:Text", "retire:Text", "start:Image"]
    );
    owner.retire().unwrap();
}

#[test]
fn switching_encoders_always_retires_the_previous_process_first() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    owner
        .encode_description("changed description", &cancelled)
        .unwrap();
    assert_eq!(
        scenario.borrow().events,
        ["start:Image", "retire:Image", "start:Text"]
    );
    owner.retire().unwrap();
}

#[test]
fn failed_or_incomplete_retirement_blocks_replacement_and_reuse() {
    for retirement in [Retirement::Fail, Retirement::Incomplete] {
        let (mut owner, scenario, _) = fixture();
        let cancelled = AtomicBool::new(false);
        owner.encode_prepared_image(&[], &cancelled).unwrap();
        scenario.borrow_mut().retirement = retirement;
        assert!(owner.retire().is_err());
        assert!(owner.encode_prepared_image(&[], &cancelled).is_err());
        assert!(owner.encode_description("next", &cancelled).is_err());
        assert_eq!(scenario.borrow().active, 1);
        assert_eq!(
            scenario
                .borrow()
                .events
                .iter()
                .filter(|value| value.starts_with("start:"))
                .count(),
            1
        );
        scenario.borrow_mut().retirement = Retirement::Complete;
        owner.encode_description("next", &cancelled).unwrap();
        assert_eq!(owner.resident_kind(), Some(ClassificationEncoderKind::Text));
        owner.retire().unwrap();
    }
}

#[test]
fn cancelled_result_never_leaves_a_warm_worker_or_successful_feature() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    scenario.borrow_mut().cancel_after_inference = true;
    assert_eq!(
        owner
            .encode_prepared_image(&[], &cancelled)
            .unwrap_err()
            .code,
        "classification_cancelled"
    );
    assert_eq!(scenario.borrow().active, 0);
    assert_eq!(owner.resident_kind(), None);
}

#[test]
fn cancellation_before_admission_never_loads_a_model() {
    let (mut owner, scenario, _) = fixture();
    assert!(
        owner
            .encode_prepared_image(&[], &AtomicBool::new(true))
            .is_err()
    );
    assert!(scenario.borrow().events.is_empty());
}

#[test]
fn cancellation_before_a_warm_request_releases_the_previous_encoder() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    cancelled.store(true, Ordering::Release);
    assert!(owner.encode_prepared_image(&[], &cancelled).is_err());
    assert_eq!(scenario.borrow().active, 0);
    assert_eq!(scenario.borrow().events, ["start:Image", "retire:Image"]);
}

#[test]
fn inference_failure_retires_the_encoder_before_an_explicit_retry() {
    let (mut owner, scenario, _) = fixture();
    let cancelled = AtomicBool::new(false);
    scenario.borrow_mut().fail_inference = true;
    assert_eq!(
        owner
            .encode_prepared_image(&[], &cancelled)
            .unwrap_err()
            .code,
        "failed"
    );
    assert_eq!(scenario.borrow().active, 0);
    scenario.borrow_mut().fail_inference = false;
    owner.encode_prepared_image(&[], &cancelled).unwrap();
    assert_eq!(
        scenario.borrow().events,
        ["start:Image", "retire:Image", "start:Image"]
    );
    owner.retire().unwrap();
}
