use std::io::Cursor;

use super::*;

#[test]
fn superseded_session_request_phase_and_version_are_rejected() {
    let encoded = Response::Encoded {
        session: 7,
        request: 19,
    };
    encoded.validate(7, ExpectedResponse::Encoded(19)).unwrap();
    for (session, expected) in [
        (8, ExpectedResponse::Encoded(19)),
        (7, ExpectedResponse::Encoded(18)),
        (7, ExpectedResponse::Stopped(19)),
        (7, ExpectedResponse::Ready),
    ] {
        assert!(matches!(
            encoded.validate(session, expected),
            Err(RuntimeError::Protocol)
        ));
    }
    assert!(matches!(
        Response::Ready {
            version: VERSION + 1,
            session: 7,
            contract: super::super::manifest::contract_revision(),
        }
        .validate(7, ExpectedResponse::Ready),
        Err(RuntimeError::Protocol)
    ));
    let stale_failure = Response::Failed {
        session: 7,
        request: Some(18),
        failure: Failure::Inference,
    };
    assert!(matches!(
        stale_failure.validate(7, ExpectedResponse::Encoded(19)),
        Err(RuntimeError::Protocol)
    ));
    assert!(matches!(
        Response::Ready {
            version: VERSION,
            session: 7,
            contract: [0; 32]
        }
        .validate(7, ExpectedResponse::Ready),
        Err(RuntimeError::Protocol)
    ));
}

#[test]
fn image_frame_preserves_session_request_and_exact_pixel_bytes() {
    let pixels: Vec<f32> = (0..IMAGE_VALUES)
        .map(|index| (index % 251) as f32 / 250.0)
        .collect();
    let mut stream = Vec::new();
    write(
        &mut stream,
        &Request::Image {
            session: 41,
            request: 12,
        },
        &encode_floats(&pixels),
    )
    .unwrap();
    let decoded = read::<Request>(&mut Cursor::new(stream)).unwrap().unwrap();
    assert!(matches!(
        decoded.message,
        Request::Image {
            session: 41,
            request: 12
        }
    ));
    assert_eq!(decode_floats(&decoded.payload), pixels);
}

#[test]
fn oversized_control_is_rejected_before_allocating_or_reading_it() {
    let mut bytes = Vec::from(u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    let mut stream = Cursor::new(bytes);
    assert!(matches!(
        read::<Request>(&mut stream),
        Err(RuntimeError::Protocol)
    ));
    assert_eq!(stream.position(), 8);
}

#[test]
fn payload_size_must_match_message_before_reading_tensor() {
    let control = br#"{"type":"image","session":1,"request":1}"#;
    let mut bytes = Vec::from((control.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(control);
    assert!(matches!(
        read::<Request>(&mut Cursor::new(bytes)),
        Err(RuntimeError::Protocol)
    ));
}

#[test]
fn unknown_fields_and_variants_do_not_change_protocol_meaning() {
    for control in [
        br#"{"type":"stop","session":1,"request":2,"path":"ignored"}"#.as_slice(),
        br#"{"type":"download","session":1}"#.as_slice(),
    ] {
        let mut bytes = Vec::from((control.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(control);
        assert!(matches!(
            read::<Request>(&mut Cursor::new(bytes)),
            Err(RuntimeError::Protocol)
        ));
    }
}

#[test]
fn eof_is_clean_only_between_complete_frames() {
    assert!(
        read::<Request>(&mut Cursor::new(Vec::new()))
            .unwrap()
            .is_none()
    );
    let mut bytes = Vec::new();
    write(
        &mut bytes,
        &Response::Encoded {
            session: 2,
            request: 3,
        },
        &vec![0; EMBEDDING_VALUES * 4],
    )
    .unwrap();
    for length in [1, 7, 9, bytes.len() - 1] {
        assert!(read::<Response>(&mut Cursor::new(&bytes[..length])).is_err());
    }
}

#[test]
fn descriptions_are_bounded_by_the_control_frame_even_before_model_validation() {
    let message = Request::Text {
        session: 1,
        request: 2,
        description: "a".repeat(MAX_CONTROL_BYTES),
    };
    let mut output = Vec::new();
    assert!(matches!(
        write(&mut output, &message, &[]),
        Err(RuntimeError::Protocol)
    ));
    assert!(output.is_empty());
}
