use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;

use crate::domain::analysis_capacity::AnalysisCapacity;
use crate::domain::image_features::{AnalysisRevision, Embedding};
use crate::ports::classification_encoder::ClassificationEncoder;

use super::*;

#[derive(Default)]
struct Calls {
    live: [AtomicUsize; 2],
    starts: [AtomicUsize; 2],
    fail_retirement: AtomicBool,
    image_latch: Mutex<Option<(SyncSender<()>, Receiver<()>)>>,
}

struct Capacity(Arc<AtomicBool>);
impl AnalysisCapacityProvider for Capacity {
    fn capacity(&self) -> Option<AnalysisCapacity> {
        Some(AnalysisCapacity {
            processors: 16,
            physical_bytes: 16 << 30,
            available_bytes: 4 << 30,
            commit_headroom_bytes: 4 << 30,
        })
    }
    fn memory_pressure(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
struct Factory(Arc<Calls>);
impl ClassificationEncoderFactory for Factory {
    type Encoder = Encoder;
    fn compatibility(&self) -> ModelCompatibility {
        let revision = AnalysisRevision::description("scheduler fixture");
        ModelCompatibility::new(revision, revision, revision, 2).unwrap()
    }
    fn create(&self, kind: ClassificationEncoderKind) -> Result<Encoder, ScanError> {
        Ok(Encoder {
            calls: self.0.clone(),
            kind,
            retired: true,
            model: self.compatibility(),
        })
    }
}

struct Encoder {
    calls: Arc<Calls>,
    kind: ClassificationEncoderKind,
    retired: bool,
    model: ModelCompatibility,
}
impl ClassificationEncoder for Encoder {
    fn initialize(&mut self, _: &AtomicBool) -> Result<(), ScanError> {
        assert_eq!(
            self.calls.live[index(self.kind)].fetch_add(1, Ordering::AcqRel),
            0
        );
        self.calls.starts[index(self.kind)].fetch_add(1, Ordering::AcqRel);
        self.retired = false;
        Ok(())
    }
    fn encode_prepared_image(
        &mut self,
        _: &[f32],
        _: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        let latch = self.calls.image_latch.lock().unwrap().take();
        if let Some((entered, release)) = latch {
            entered.send(()).unwrap();
            release
                .recv_timeout(Duration::from_secs(5))
                .map_err(|_| unavailable())?;
        }
        Ok(ImageFeature {
            identity: self.model.image_identity(),
            embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
        })
    }
    fn encode_description(
        &mut self,
        description: &str,
        _: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        Ok(TextFeature {
            identity: self.model.text_identity(),
            description: AnalysisRevision::description(description),
            embedding: Embedding::normalized(vec![1.0, 0.0]).unwrap(),
        })
    }
    fn retire(&mut self) -> Result<(), ScanError> {
        if self.calls.fail_retirement.load(Ordering::Acquire) {
            return Err(unavailable());
        }
        if !self.retired {
            self.calls.live[index(self.kind)].fetch_sub(1, Ordering::AcqRel);
        }
        self.retired = true;
        Ok(())
    }
    fn is_retired(&self) -> bool {
        self.retired
    }
}

#[test]
fn concurrent_search_preserves_image_work_and_pressure_switches_to_serial() {
    let calls = Arc::new(Calls::default());
    let pressure = Arc::new(AtomicBool::new(false));
    let scheduler = AnalysisScheduler::new(
        Factory(calls.clone()),
        Box::new(Capacity(pressure.clone())),
        true,
    );
    let (entered, image_started) = mpsc::sync_channel(1);
    let (release_image, release) = mpsc::sync_channel(1);
    *calls.image_latch.lock().unwrap() = Some((entered, release));
    let mut background = scheduler.client(Priority::Background);
    let image =
        thread::spawn(move || background.encode_prepared_image(&[], &AtomicBool::new(false)));
    image_started.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut foreground = scheduler.client(Priority::Foreground);
    let session = scheduler
        .foreground(ClassificationEncoderKind::Text)
        .unwrap();
    let searched = foreground.encode_description("first", &AtomicBool::new(false));
    let concurrent = calls
        .live
        .each_ref()
        .map(|count| count.load(Ordering::Acquire));
    let cancelled = foreground.encode_description("cancelled", &AtomicBool::new(true));
    let image_survived = calls.live[0].load(Ordering::Acquire);
    release_image.send(()).unwrap();
    let completed = image.join().unwrap();
    drop(session);
    assert!(searched.is_ok());
    assert_eq!(concurrent, [1, 1]);
    assert!(cancelled.is_err());
    assert_eq!(image_survived, 1);
    assert!(completed.is_ok());

    pressure.store(true, Ordering::Release);
    let session = scheduler
        .foreground(ClassificationEncoderKind::Text)
        .unwrap();
    let error = scheduler
        .client(Priority::Background)
        .encode_prepared_image(&[], &AtomicBool::new(false))
        .unwrap_err();
    assert_eq!(error.code, "analysis_yielded");
    foreground
        .encode_description("under pressure", &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        calls
            .live
            .each_ref()
            .map(|count| count.load(Ordering::Acquire)),
        [0, 1]
    );
    drop(session);
    scheduler.retire().unwrap();
    assert_eq!(
        calls
            .live
            .each_ref()
            .map(|count| count.load(Ordering::Acquire)),
        [0, 0]
    );
}

#[test]
fn serial_replacement_waits_for_confirmed_retirement() {
    let calls = Arc::new(Calls::default());
    let scheduler = AnalysisScheduler::new(
        Factory(calls.clone()),
        Box::new(Capacity(Arc::new(AtomicBool::new(true)))),
        false,
    );
    scheduler
        .client(Priority::Background)
        .encode_prepared_image(&[], &AtomicBool::new(false))
        .unwrap();
    calls.fail_retirement.store(true, Ordering::Release);
    let session = scheduler
        .foreground(ClassificationEncoderKind::Text)
        .unwrap();
    let mut foreground = scheduler.client(Priority::Foreground);
    assert!(
        foreground
            .encode_description("blocked", &AtomicBool::new(false))
            .is_err()
    );
    assert_eq!(calls.starts[1].load(Ordering::Acquire), 0);
    assert_eq!(calls.live[0].load(Ordering::Acquire), 1);
    calls.fail_retirement.store(false, Ordering::Release);
    foreground
        .encode_description("replacement", &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        calls
            .live
            .each_ref()
            .map(|count| count.load(Ordering::Acquire)),
        [0, 1]
    );
    drop(session);
    scheduler.retire().unwrap();
}
