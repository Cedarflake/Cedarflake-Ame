use super::{FileReference, ParsedUsnRecord};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UsnParseError {
    BufferMalformed,
    RecordMalformed,
    VersionUnsupported,
    NameInvalid,
}

#[derive(Debug)]
pub(super) struct DecodedUsnRecord {
    pub(super) file_reference: FileReference,
    pub(super) parent_reference: FileReference,
    pub(super) usn: i64,
    pub(super) timestamp_100ns: i64,
    pub(super) reason: u32,
    pub(super) file_attributes: u32,
    pub(super) name: Option<String>,
}

pub(crate) fn parse_journal_buffer(
    buffer: &[u8],
) -> Result<(i64, Vec<ParsedUsnRecord>), UsnParseError> {
    decode_journal_buffer(buffer, |record| {
        Ok(ParsedUsnRecord {
            file_reference: record.file_reference,
            parent_reference: record.parent_reference,
            usn: record.usn,
            timestamp_100ns: record.timestamp_100ns,
            reason: record.reason,
            file_attributes: record.file_attributes,
            name: record.name.ok_or(UsnParseError::NameInvalid)?,
        })
    })
}

// Record admission keeps redacted identity evidence out of named-path reconstruction without
// retaining a second decoded page or manufacturing a historical filename from a current path.
pub(super) fn decode_journal_buffer<Record>(
    buffer: &[u8],
    mut admit_record: impl FnMut(DecodedUsnRecord) -> Result<Record, UsnParseError>,
) -> Result<(i64, Vec<Record>), UsnParseError> {
    if buffer.len() < 8 {
        return Err(UsnParseError::BufferMalformed);
    }
    let next_usn = read_i64(buffer, 0).map_err(|_| UsnParseError::BufferMalformed)?;
    let mut offset = 8_usize;
    let mut records = Vec::new();
    while offset < buffer.len() {
        let remaining = buffer.get(offset..).ok_or(UsnParseError::BufferMalformed)?;
        if remaining.len() < 8 {
            return Err(UsnParseError::BufferMalformed);
        }
        let record_length = read_u32(remaining, 0)? as usize;
        if record_length < 60 || !record_length.is_multiple_of(8) || record_length > remaining.len()
        {
            return Err(UsnParseError::RecordMalformed);
        }
        let record = decode_usn_record(
            remaining
                .get(..record_length)
                .ok_or(UsnParseError::RecordMalformed)?,
        )?;
        records.push(admit_record(record)?);
        offset = offset
            .checked_add(record_length)
            .ok_or(UsnParseError::RecordMalformed)?;
    }
    Ok((next_usn, records))
}

fn decode_usn_record(record: &[u8]) -> Result<DecodedUsnRecord, UsnParseError> {
    let version = read_u16(record, 4)?;
    let (
        file_reference,
        parent_reference,
        usn_offset,
        timestamp_offset,
        reason_offset,
        attributes_offset,
        name_length_offset,
        name_offset_offset,
        minimum_length,
    ) = match version {
        2 => (
            FileReference::V2(read_array(record, 8)?),
            FileReference::V2(read_array(record, 16)?),
            24,
            32,
            40,
            52,
            56,
            58,
            60,
        ),
        3 => (
            FileReference::V3(read_array(record, 8)?),
            FileReference::V3(read_array(record, 24)?),
            40,
            48,
            56,
            68,
            72,
            74,
            76,
        ),
        _ => return Err(UsnParseError::VersionUnsupported),
    };
    if record.len() < minimum_length {
        return Err(UsnParseError::RecordMalformed);
    }
    let name_length = read_u16(record, name_length_offset)? as usize;
    let name_offset = read_u16(record, name_offset_offset)? as usize;
    if !name_length.is_multiple_of(2)
        || !name_offset.is_multiple_of(2)
        || name_offset < minimum_length
    {
        return Err(UsnParseError::RecordMalformed);
    }
    let name_end = name_offset
        .checked_add(name_length)
        .ok_or(UsnParseError::RecordMalformed)?;
    let name_bytes = record
        .get(name_offset..name_end)
        .ok_or(UsnParseError::RecordMalformed)?;
    let name = decode_name(name_bytes)?;
    let usn = read_i64(record, usn_offset)?;
    if usn < 0 {
        return Err(UsnParseError::RecordMalformed);
    }
    Ok(DecodedUsnRecord {
        file_reference,
        parent_reference,
        usn,
        timestamp_100ns: read_i64(record, timestamp_offset)?,
        reason: read_u32(record, reason_offset)?,
        file_attributes: read_u32(record, attributes_offset)?,
        name,
    })
}

fn decode_name(bytes: &[u8]) -> Result<Option<String>, UsnParseError> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let name_utf16: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let name = String::from_utf16(&name_utf16).map_err(|_| UsnParseError::NameInvalid)?;
    if matches!(name.as_str(), "." | "..") || name.contains(['\\', '/', '\0']) {
        return Err(UsnParseError::NameInvalid);
    }
    Ok(Some(name))
}

fn read_array<const N: usize>(buffer: &[u8], offset: usize) -> Result<[u8; N], UsnParseError> {
    buffer
        .get(offset..offset.saturating_add(N))
        .and_then(|value| value.try_into().ok())
        .ok_or(UsnParseError::RecordMalformed)
}

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16, UsnParseError> {
    Ok(u16::from_le_bytes(read_array(buffer, offset)?))
}

fn read_u32(buffer: &[u8], offset: usize) -> Result<u32, UsnParseError> {
    Ok(u32::from_le_bytes(read_array(buffer, offset)?))
}

fn read_i64(buffer: &[u8], offset: usize) -> Result<i64, UsnParseError> {
    Ok(i64::from_le_bytes(read_array(buffer, offset)?))
}

#[cfg(test)]
mod tests;
