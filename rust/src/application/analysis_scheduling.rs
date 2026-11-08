use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::domain::ScanError;
use crate::domain::image_features::{ImageFeature, ModelCompatibility, TextFeature};
use crate::ports::analysis_capacity::AnalysisCapacityProvider;
use crate::ports::classification_encoder::{
    ClassificationEncoderFactory, ClassificationEncoderKind,
};
use crate::ports::image_feature_encoding::ImageFeatureEncoding;

use super::classification_residency::{ClassificationEncoderResidency, MonotonicResidencyClock};

mod admission;
pub(crate) use admission::ForegroundSession;
use admission::{Admission, index, other};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Priority {
    Background,
    Foreground,
}

type Residency<F> = ClassificationEncoderResidency<F, MonotonicResidencyClock>;

pub(crate) struct AnalysisScheduler<F: ClassificationEncoderFactory> {
    admission: Admission,
    slots: [Mutex<Residency<F>>; 2],
    model: ModelCompatibility,
}

pub(crate) struct ScheduledEncoding<F: ClassificationEncoderFactory> {
    scheduler: Arc<AnalysisScheduler<F>>,
    priority: Priority,
}

impl<F: ClassificationEncoderFactory + Clone> AnalysisScheduler<F> {
    pub(crate) fn new(
        factory: F,
        capacity: Box<dyn AnalysisCapacityProvider>,
        gpu: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            admission: Admission::new(capacity, gpu),
            model: factory.compatibility(),
            slots: [
                Mutex::new(Residency::new(
                    factory.clone(),
                    MonotonicResidencyClock::default(),
                    Duration::from_secs(60),
                )),
                Mutex::new(Residency::new(
                    factory,
                    MonotonicResidencyClock::default(),
                    Duration::from_secs(15),
                )),
            ],
        })
    }
}

impl<F: ClassificationEncoderFactory> AnalysisScheduler<F> {
    pub(crate) fn client(self: &Arc<Self>, priority: Priority) -> ScheduledEncoding<F> {
        ScheduledEncoding {
            scheduler: self.clone(),
            priority,
        }
    }

    pub(crate) fn foreground(
        &self,
        kind: ClassificationEncoderKind,
    ) -> Result<ForegroundSession<'_>, ScanError> {
        self.admission.foreground(kind)
    }

    pub(crate) fn background_delay(&self) -> Result<Option<Duration>, ScanError> {
        self.admission.background_delay()
    }

    pub(crate) fn retire(&self) -> Result<(), ScanError> {
        self.admission.close()?;
        let image = self.slots[0].lock().map_err(|_| unavailable())?.retire();
        let text = self.slots[1].lock().map_err(|_| unavailable())?.retire();
        image.and(text)
    }

    fn perform<T>(
        &self,
        kind: ClassificationEncoderKind,
        priority: Priority,
        cancelled: &AtomicBool,
        operation: impl FnOnce(&mut Residency<F>) -> Result<T, ScanError>,
    ) -> Result<T, ScanError> {
        let admission = self.admission.acquire(kind, priority, cancelled)?;
        if admission.serial {
            self.slots[other(kind)]
                .lock()
                .map_err(|_| unavailable())?
                .retire()?;
        }
        let mut slot = self.slots[index(kind)].lock().map_err(|_| unavailable())?;
        let result = operation(&mut slot);
        drop(slot);
        drop(admission);
        result
    }

    fn maintain(&self, idle_only: bool) -> Result<(), ScanError> {
        for kind in [
            ClassificationEncoderKind::Image,
            ClassificationEncoderKind::Text,
        ] {
            if let Some(_admission) = self.admission.maintenance(kind)? {
                let mut slot = self.slots[index(kind)].lock().map_err(|_| unavailable())?;
                if idle_only && !self.admission.should_release(kind)? {
                    slot.retire_if_idle()?;
                } else {
                    slot.retire()?;
                }
            }
        }
        Ok(())
    }
}

impl<F: ClassificationEncoderFactory> ScheduledEncoding<F> {
    pub(crate) fn retire(&self) -> Result<(), ScanError> {
        self.scheduler.maintain(false)
    }
    pub(crate) fn retire_if_idle(&self) -> Result<(), ScanError> {
        self.scheduler.maintain(true)
    }
    pub(crate) fn scheduler(&self) -> Arc<AnalysisScheduler<F>> {
        self.scheduler.clone()
    }
}

impl<F: ClassificationEncoderFactory> ImageFeatureEncoding for ScheduledEncoding<F> {
    fn compatibility(&self) -> ModelCompatibility {
        self.scheduler.model
    }

    fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        self.scheduler.perform(
            ClassificationEncoderKind::Image,
            self.priority,
            cancelled,
            |resident| resident.encode_prepared_image(pixels, cancelled),
        )
    }

    fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        self.scheduler.perform(
            ClassificationEncoderKind::Text,
            self.priority,
            cancelled,
            |resident| resident.encode_description(description, cancelled),
        )
    }

    fn finish_descriptions(&mut self) -> Result<(), ScanError> {
        Ok(())
    }
}

fn unavailable() -> ScanError {
    ScanError::new(
        "analysis_scheduler_unavailable",
        "Analysis scheduling ownership is unavailable",
    )
}

#[cfg(test)]
mod tests;
