use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use crate::domain::classification::{
    ClassificationRevision, Embedding, ImageFeature, ModelCompatibility, TextFeature,
};

use super::encoder::valid_description;
use super::manifest;
use super::protocol::{self, Frame, Request, Response};
use super::transport::Transport;
use super::{EncoderKind, RuntimeError};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

pub struct WorkerConfiguration {
    pub executable: PathBuf,
    pub model_directory: PathBuf,
    pub runtime_directory: PathBuf,
    pub encoder: EncoderKind,
}

pub struct WorkerSession {
    transport: Transport,
    session: u64,
    next_request: u64,
    encoder: EncoderKind,
}

impl WorkerSession {
    pub fn start(
        configuration: WorkerConfiguration,
        cancelled: &AtomicBool,
    ) -> Result<Self, RuntimeError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(RuntimeError::Cancelled);
        }
        let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        if session == 0 {
            return Err(RuntimeError::Configuration);
        }
        let mut worker = Self {
            transport: Transport::start(&configuration.executable)?,
            session,
            next_request: 1,
            encoder: configuration.encoder,
        };
        worker.exchange(
            Request::Start {
                version: protocol::VERSION,
                session,
                encoder: configuration.encoder,
                models: configuration.model_directory,
                runtime: configuration.runtime_directory,
            },
            Vec::new(),
            Duration::from_secs(30),
            cancelled,
        )?;
        Ok(worker)
    }

    pub fn compatibility(&self) -> ModelCompatibility {
        manifest::compatibility()
    }

    pub fn process_id(&self) -> u32 {
        self.transport.process_id()
    }

    pub fn is_retired(&self) -> bool {
        self.transport.is_retired()
    }

    pub fn encode_prepared_image(
        &mut self,
        pixels: &[f32],
        cancelled: &AtomicBool,
    ) -> Result<ImageFeature, RuntimeError> {
        if self.encoder != EncoderKind::Image
            || pixels.len() != manifest::IMAGE_VALUES
            || pixels.iter().any(|value| !(-1.0..=1.0).contains(value))
        {
            return Err(RuntimeError::InvalidInput);
        }
        let request = self.allocate_request()?;
        let frame = self.exchange(
            Request::Image {
                session: self.session,
                request,
            },
            protocol::encode_floats(pixels),
            Duration::from_secs(10),
            cancelled,
        )?;
        Ok(ImageFeature {
            identity: self.compatibility().image_identity(),
            embedding: self.embedding(frame)?,
        })
    }

    pub fn encode_description(
        &mut self,
        description: &str,
        cancelled: &AtomicBool,
    ) -> Result<TextFeature, RuntimeError> {
        if self.encoder != EncoderKind::Text || !valid_description(description) {
            return Err(RuntimeError::InvalidInput);
        }
        let request = self.allocate_request()?;
        let frame = self.exchange(
            Request::Text {
                session: self.session,
                request,
                description: description.to_owned(),
            },
            Vec::new(),
            Duration::from_secs(10),
            cancelled,
        )?;
        Ok(TextFeature {
            identity: self.compatibility().text_identity(),
            description: ClassificationRevision::description(description),
            embedding: self.embedding(frame)?,
        })
    }

    pub fn shutdown(&mut self) -> Result<(), RuntimeError> {
        let request = self.allocate_request()?;
        let result = self.exchange(
            Request::Stop {
                session: self.session,
                request,
            },
            Vec::new(),
            Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        self.transport.retire()?;
        result.map(|_| ())
    }

    fn allocate_request(&mut self) -> Result<u64, RuntimeError> {
        let request = self.next_request;
        self.next_request = request.checked_add(1).ok_or(RuntimeError::Protocol)?;
        Ok(request)
    }

    fn embedding(&mut self, frame: Frame<Response>) -> Result<Embedding, RuntimeError> {
        let embedding = Embedding::normalized(protocol::decode_floats(&frame.payload));
        match embedding {
            Ok(embedding) => Ok(embedding),
            Err(_) => {
                self.transport.retire()?;
                Err(RuntimeError::Protocol)
            }
        }
    }

    fn exchange(
        &mut self,
        message: Request,
        payload: Vec<u8>,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Frame<Response>, RuntimeError> {
        let expected = message.expected_response();
        let result = self
            .transport
            .request(Frame { message, payload }, timeout, cancelled);
        let result = result.and_then(|frame| {
            frame.message.validate(self.session, expected)?;
            if cancelled.load(Ordering::Acquire) {
                return Err(RuntimeError::Cancelled);
            }
            Ok(frame)
        });
        if result.is_err() {
            self.transport.retire()?;
        }
        result
    }
}
