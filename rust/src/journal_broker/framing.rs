use thiserror::Error;

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum BrokerFrameError {
    #[error("broker frame length is zero or exceeds the hard limit")]
    LengthRejected,
    #[error("broker frame length cannot be represented")]
    LengthOverflow,
    #[error("broker frame is truncated")]
    Truncated,
    #[error("broker frame contains trailing bytes")]
    TrailingBytes,
}

pub(crate) fn encode_frame(payload: &[u8]) -> Result<Vec<u8>, BrokerFrameError> {
    if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
        return Err(BrokerFrameError::LengthRejected);
    }
    let length = u32::try_from(payload.len()).map_err(|_| BrokerFrameError::LengthOverflow)?;
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

pub(crate) fn decode_frame(frame: &[u8]) -> Result<&[u8], BrokerFrameError> {
    let length_bytes = frame.get(..4).ok_or(BrokerFrameError::Truncated)?;
    let length = decode_frame_length(
        length_bytes
            .try_into()
            .map_err(|_| BrokerFrameError::Truncated)?,
    )?;
    let end = 4_usize
        .checked_add(length)
        .ok_or(BrokerFrameError::LengthOverflow)?;
    let payload = frame.get(4..end).ok_or(BrokerFrameError::Truncated)?;
    if end != frame.len() {
        return Err(BrokerFrameError::TrailingBytes);
    }
    Ok(payload)
}

pub(crate) fn decode_frame_length(header: [u8; 4]) -> Result<usize, BrokerFrameError> {
    let length = usize::try_from(u32::from_le_bytes(header))
        .map_err(|_| BrokerFrameError::LengthOverflow)?;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(BrokerFrameError::LengthRejected);
    }
    Ok(length)
}
