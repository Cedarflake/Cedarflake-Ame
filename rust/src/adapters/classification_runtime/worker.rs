use std::io::{self, Read, Write};
use std::process::ExitCode;

use super::artifacts::ArtifactSet;
use super::encoder::Encoder;
use super::protocol::{self, Failure, Request, Response};
use super::{EncoderKind, RuntimeError};

pub(super) fn run() -> ExitCode {
    let result = serve(&mut io::stdin().lock(), &mut io::stdout().lock());
    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn serve(input: &mut impl Read, output: &mut impl Write) -> Result<(), RuntimeError> {
    let Some(start) = protocol::read(input)? else {
        return Ok(());
    };
    let Request::Start {
        version,
        session,
        encoder: kind,
        models,
        runtime,
    } = start.message
    else {
        return Err(RuntimeError::Protocol);
    };
    if version != protocol::VERSION || session == 0 {
        return Err(RuntimeError::Protocol);
    }
    let loaded = ArtifactSet::open(&models, &runtime, kind)
        .and_then(|artifacts| Encoder::load(artifacts, kind));
    let mut encoder = match loaded {
        Ok(encoder) => encoder,
        Err(error) => {
            fail(output, session, None, &error)?;
            return Err(error);
        }
    };
    protocol::write(
        output,
        &Response::Ready {
            version,
            session,
            contract: super::manifest::contract_revision(),
        },
        &[],
    )?;
    let mut previous_request = 0;
    while let Some(frame) = protocol::read(input)? {
        let (incoming_session, request) = match &frame.message {
            Request::Image { session, request }
            | Request::Text {
                session, request, ..
            }
            | Request::Stop { session, request } => (*session, *request),
            Request::Start { .. } => return Err(RuntimeError::Protocol),
        };
        if incoming_session != session || request <= previous_request {
            return Err(RuntimeError::Protocol);
        }
        previous_request = request;
        let result = match frame.message {
            Request::Stop { .. } => {
                drop(encoder);
                protocol::write(output, &Response::Stopped { session, request }, &[])?;
                return Ok(());
            }
            Request::Image { .. } if kind == EncoderKind::Image => {
                encoder.image(protocol::decode_floats(&frame.payload))
            }
            Request::Text { description, .. } if kind == EncoderKind::Text => {
                encoder.text(&description)
            }
            _ => Err(RuntimeError::InvalidInput),
        };
        match result {
            Ok(embedding) => protocol::write(
                output,
                &Response::Encoded { session, request },
                &protocol::encode_floats(embedding.values()),
            )?,
            Err(error) => {
                fail(output, session, Some(request), &error)?;
                return Err(error);
            }
        }
    }
    Ok(())
}

fn fail(
    output: &mut impl Write,
    session: u64,
    request: Option<u64>,
    error: &RuntimeError,
) -> Result<(), RuntimeError> {
    protocol::write(
        output,
        &Response::Failed {
            session,
            request,
            failure: Failure::from_error(error),
        },
        &[],
    )
}
