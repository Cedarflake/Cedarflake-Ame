use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::manifest::{EMBEDDING_VALUES, IMAGE_VALUES};
use super::{EncoderKind, RuntimeError};

pub(super) const VERSION: u32 = 1;
const MAX_CONTROL_BYTES: usize = 32 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Request {
    Start {
        version: u32,
        session: u64,
        encoder: EncoderKind,
        models: PathBuf,
        runtime: PathBuf,
    },
    Image {
        session: u64,
        request: u64,
    },
    Text {
        session: u64,
        request: u64,
        description: String,
    },
    Stop {
        session: u64,
        request: u64,
    },
}

#[derive(Clone, Copy)]
pub(super) enum ExpectedResponse {
    Ready,
    Encoded(u64),
    Stopped(u64),
}

impl Request {
    pub fn expected_response(&self) -> ExpectedResponse {
        match self {
            Self::Start { .. } => ExpectedResponse::Ready,
            Self::Image { request, .. } | Self::Text { request, .. } => {
                ExpectedResponse::Encoded(*request)
            }
            Self::Stop { request, .. } => ExpectedResponse::Stopped(*request),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Failure {
    Artifact,
    Configuration,
    Input,
    Runtime,
    Inference,
    Protocol,
}

impl Failure {
    pub fn from_error(error: &RuntimeError) -> Self {
        match error {
            RuntimeError::ArtifactMismatch | RuntimeError::UnsafeArtifact | RuntimeError::Io(_) => {
                Self::Artifact
            }
            RuntimeError::Configuration => Self::Configuration,
            RuntimeError::InvalidInput => Self::Input,
            RuntimeError::RuntimeUnavailable => Self::Runtime,
            RuntimeError::Inference => Self::Inference,
            _ => Self::Protocol,
        }
    }

    pub fn into_error(self) -> RuntimeError {
        match self {
            Self::Artifact => RuntimeError::ArtifactMismatch,
            Self::Configuration => RuntimeError::Configuration,
            Self::Input => RuntimeError::InvalidInput,
            Self::Runtime => RuntimeError::RuntimeUnavailable,
            Self::Inference => RuntimeError::Inference,
            Self::Protocol => RuntimeError::Protocol,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Response {
    Ready {
        version: u32,
        session: u64,
        contract: [u8; 32],
    },
    Encoded {
        session: u64,
        request: u64,
    },
    Stopped {
        session: u64,
        request: u64,
    },
    Failed {
        session: u64,
        request: Option<u64>,
        failure: Failure,
    },
}

impl Response {
    pub fn validate(&self, session: u64, expected: ExpectedResponse) -> Result<(), RuntimeError> {
        match (self, expected) {
            (
                Self::Ready {
                    version,
                    session: incoming,
                    contract,
                },
                ExpectedResponse::Ready,
            ) if *version == VERSION
                && *incoming == session
                && *contract == super::manifest::contract_revision() =>
            {
                Ok(())
            }
            (
                Self::Encoded {
                    session: incoming,
                    request,
                },
                ExpectedResponse::Encoded(expected),
            )
            | (
                Self::Stopped {
                    session: incoming,
                    request,
                },
                ExpectedResponse::Stopped(expected),
            ) if *incoming == session && *request == expected => Ok(()),
            (
                Self::Failed {
                    session: incoming,
                    request,
                    failure,
                },
                expected,
            ) if *incoming == session => {
                let expected_request = match expected {
                    ExpectedResponse::Ready => None,
                    ExpectedResponse::Encoded(request) | ExpectedResponse::Stopped(request) => {
                        Some(request)
                    }
                };
                if *request == expected_request {
                    Err(failure.into_error())
                } else {
                    Err(RuntimeError::Protocol)
                }
            }
            _ => Err(RuntimeError::Protocol),
        }
    }
}

pub(super) trait Message: Serialize + DeserializeOwned {
    fn payload_length(&self) -> usize;
}

impl Message for Request {
    fn payload_length(&self) -> usize {
        if matches!(self, Self::Image { .. }) {
            IMAGE_VALUES * 4
        } else {
            0
        }
    }
}

impl Message for Response {
    fn payload_length(&self) -> usize {
        if matches!(self, Self::Encoded { .. }) {
            EMBEDDING_VALUES * 4
        } else {
            0
        }
    }
}

#[derive(Debug)]
pub(super) struct Frame<T> {
    pub message: T,
    pub payload: Vec<u8>,
}

pub(super) fn write<T: Message>(
    stream: &mut impl Write,
    message: &T,
    payload: &[u8],
) -> Result<(), RuntimeError> {
    let control = serde_json::to_vec(message).map_err(|_| RuntimeError::Protocol)?;
    if control.len() > MAX_CONTROL_BYTES || payload.len() != message.payload_length() {
        return Err(RuntimeError::Protocol);
    }
    stream.write_all(&(control.len() as u32).to_le_bytes())?;
    stream.write_all(&(payload.len() as u32).to_le_bytes())?;
    stream.write_all(&control)?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

pub(super) fn read<T: Message>(stream: &mut impl Read) -> Result<Option<Frame<T>>, RuntimeError> {
    let mut lengths = [0; 8];
    if stream.read(&mut lengths[..1])? == 0 {
        return Ok(None);
    }
    stream.read_exact(&mut lengths[1..])?;
    let control_length = u32::from_le_bytes(lengths[..4].try_into().unwrap()) as usize;
    let payload_length = u32::from_le_bytes(lengths[4..].try_into().unwrap()) as usize;
    if control_length == 0 || control_length > MAX_CONTROL_BYTES {
        return Err(RuntimeError::Protocol);
    }
    let mut control = vec![0; control_length];
    stream.read_exact(&mut control)?;
    let message: T = serde_json::from_slice(&control).map_err(|_| RuntimeError::Protocol)?;
    if payload_length != message.payload_length() {
        return Err(RuntimeError::Protocol);
    }
    let mut payload = vec![0; payload_length];
    stream.read_exact(&mut payload)?;
    Ok(Some(Frame { message, payload }))
}

pub(super) fn decode_floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|part| f32::from_le_bytes(part.try_into().unwrap()))
        .collect()
}

pub(super) fn encode_floats(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[cfg(test)]
mod tests;
