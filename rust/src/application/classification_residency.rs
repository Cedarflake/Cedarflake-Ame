use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::domain::ScanError;
use crate::domain::classification::{ImageFeature, ModelCompatibility, TextFeature};
use crate::ports::classification_encoder::{
    ClassificationEncoder, ClassificationEncoderFactory, ClassificationEncoderKind,
};

pub trait ResidencyClock {
    fn elapsed(&self) -> Duration;
}

pub struct MonotonicResidencyClock(Instant);

impl Default for MonotonicResidencyClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}

impl ResidencyClock for MonotonicResidencyClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

enum EncoderState {
    Initializing,
    Available,
    Retiring,
}

struct Resident<E> {
    encoder: E,
    kind: ClassificationEncoderKind,
    state: EncoderState,
    last_used: Duration,
}

pub struct ClassificationEncoderResidency<F: ClassificationEncoderFactory, C: ResidencyClock> {
    factory: F,
    clock: C,
    idle_retention: Duration,
    resident: Option<Resident<F::Encoder>>,
}

impl<F: ClassificationEncoderFactory, C: ResidencyClock> ClassificationEncoderResidency<F, C> {
    pub fn new(factory: F, clock: C, idle_retention: Duration) -> Self {
        Self {
            factory,
            clock,
            idle_retention,
            resident: None,
        }
    }

    pub fn compatibility(&self) -> ModelCompatibility {
        self.factory.compatibility()
    }

    pub fn resident_kind(&self) -> Option<ClassificationEncoderKind> {
        self.resident.as_ref().map(|resident| resident.kind)
    }

    pub fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, ScanError> {
        let result = self
            .encoder(ClassificationEncoderKind::Image, cancelled)?
            .encode_prepared_image(pixels, cancelled);
        self.complete(result, cancelled)
    }

    pub fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, ScanError> {
        let result = self
            .encoder(ClassificationEncoderKind::Text, cancelled)?
            .encode_description(description, cancelled);
        self.complete(result, cancelled)
    }

    pub fn finish_descriptions(&mut self) -> Result<(), ScanError> {
        if self.resident_kind() == Some(ClassificationEncoderKind::Text) {
            self.retire()?;
        }
        Ok(())
    }

    pub fn idle_delay(&self) -> Option<Duration> {
        self.resident.as_ref().map(|resident| {
            if matches!(resident.state, EncoderState::Retiring) {
                Duration::ZERO
            } else {
                self.idle_retention
                    .saturating_sub(self.clock.elapsed().saturating_sub(resident.last_used))
            }
        })
    }

    pub fn retire_if_idle(&mut self) -> Result<(), ScanError> {
        if let Some(resident) = &self.resident
            && (matches!(resident.state, EncoderState::Retiring)
                || self.clock.elapsed().saturating_sub(resident.last_used) >= self.idle_retention)
        {
            self.retire()?;
        }
        Ok(())
    }

    pub fn retire(&mut self) -> Result<(), ScanError> {
        let Some(resident) = &mut self.resident else {
            return Ok(());
        };
        resident.state = EncoderState::Retiring;
        let result = resident.encoder.retire();
        if resident.encoder.is_retired() {
            self.resident = None;
            return result;
        }
        result?;
        Err(ScanError::new(
            "classification_retirement_incomplete",
            "The previous classification worker has not finished stopping",
        ))
    }

    fn encoder(
        &mut self,
        kind: ClassificationEncoderKind,
        cancelled: &AtomicBool,
    ) -> Result<&mut F::Encoder, ScanError> {
        if cancelled.load(Ordering::Acquire) {
            self.retire()?;
            return Err(cancelled_error());
        }
        if self.resident.as_ref().is_some_and(|resident| {
            resident.kind != kind
                || !matches!(resident.state, EncoderState::Available)
                || resident.encoder.is_retired()
        }) {
            self.retire()?;
        }
        if self.resident.is_none() {
            let encoder = self.factory.create(kind)?;
            self.resident = Some(Resident {
                encoder,
                kind,
                state: EncoderState::Initializing,
                last_used: self.clock.elapsed(),
            });
            let resident = self
                .resident
                .as_mut()
                .expect("the initializing worker is owned");
            if let Err(error) = resident.encoder.initialize(cancelled) {
                self.retire()?;
                return Err(error);
            }
            resident.state = EncoderState::Available;
        }
        if cancelled.load(Ordering::Acquire) {
            self.retire()?;
            return Err(cancelled_error());
        }
        Ok(&mut self.resident.as_mut().expect("the worker is owned").encoder)
    }

    fn complete<T>(
        &mut self,
        result: Result<T, ScanError>,
        cancelled: &AtomicBool,
    ) -> Result<T, ScanError> {
        if result.is_err() || cancelled.load(Ordering::Acquire) {
            self.retire()?;
            return match result {
                Err(error) => Err(error),
                Ok(_) => Err(cancelled_error()),
            };
        }
        if let Some(resident) = &mut self.resident {
            resident.last_used = self.clock.elapsed();
        }
        result
    }
}

fn cancelled_error() -> ScanError {
    ScanError::new("classification_cancelled", "Classification was stopped")
}

#[cfg(test)]
mod tests;
