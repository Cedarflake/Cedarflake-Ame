use std::collections::{BTreeMap, BTreeSet};

use windows_sys::Win32::System::Ioctl::{USN_REASON_FILE_DELETE, USN_REASON_RENAME_OLD_NAME};

#[cfg_attr(not(test), allow(dead_code))]
const WINDOWS_TO_UNIX_EPOCH_100NS: i64 = 116_444_736_000_000_000;
#[cfg_attr(not(test), allow(dead_code))]
const HUNDRED_NS_PER_MILLISECOND: i64 = 10_000;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FileReference {
    V2([u8; 8]),
    V3([u8; 16]),
}

impl FileReference {
    pub(crate) fn bytes(self) -> Vec<u8> {
        match self {
            Self::V2(value) => value.to_vec(),
            Self::V3(value) => value.to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedUsnRecord {
    pub(crate) file_reference: FileReference,
    pub(crate) parent_reference: FileReference,
    pub(crate) usn: i64,
    // The legacy catch-up adapter remains test-only while the broker replaces it. Keep its
    // timestamp semantics in the shared parser so both consumers cannot drift during removal.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) timestamp_100ns: i64,
    pub(crate) reason: u32,
    pub(crate) file_attributes: u32,
    pub(crate) name: String,
}

impl ParsedUsnRecord {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn observed_unix_ms(&self) -> i64 {
        self.timestamp_100ns
            .saturating_sub(WINDOWS_TO_UNIX_EPOCH_100NS)
            .checked_div(HUNDRED_NS_PER_MILLISECOND)
            .unwrap_or(0)
            .max(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UsnParseError {
    BufferMalformed,
    RecordMalformed,
    VersionUnsupported,
    NameInvalid,
}

pub(crate) fn parse_journal_buffer(
    buffer: &[u8],
) -> Result<(i64, Vec<ParsedUsnRecord>), UsnParseError> {
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
        records.push(parse_usn_record(
            remaining
                .get(..record_length)
                .ok_or(UsnParseError::RecordMalformed)?,
        )?);
        offset = offset
            .checked_add(record_length)
            .ok_or(UsnParseError::RecordMalformed)?;
    }
    Ok((next_usn, records))
}

fn parse_usn_record(record: &[u8]) -> Result<ParsedUsnRecord, UsnParseError> {
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
    let name_utf16: Vec<u16> = name_bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let name = String::from_utf16(&name_utf16).map_err(|_| UsnParseError::NameInvalid)?;
    if name.is_empty() || matches!(name.as_str(), "." | "..") || name.contains(['\\', '/', '\0']) {
        return Err(UsnParseError::NameInvalid);
    }
    let usn = read_i64(record, usn_offset)?;
    if usn < 0 {
        return Err(UsnParseError::RecordMalformed);
    }
    Ok(ParsedUsnRecord {
        file_reference,
        parent_reference,
        usn,
        timestamp_100ns: read_i64(record, timestamp_offset)?,
        reason: read_u32(record, reason_offset)?,
        file_attributes: read_u32(record, attributes_offset)?,
        name,
    })
}

pub(crate) type ReferenceHistories = BTreeMap<FileReference, Vec<usize>>;

pub(crate) fn reference_histories(records: &[ParsedUsnRecord]) -> ReferenceHistories {
    let mut histories = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        histories
            .entry(record.file_reference)
            .or_insert_with(Vec::new)
            .push(index);
    }
    histories
}

#[derive(Debug)]
pub(crate) enum ReferenceResolutionError<CallbackError> {
    CycleOrDepth,
    Callback(CallbackError),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_reference_path_with<Path, CallbackError, OpenReference, AppendName>(
    reference: FileReference,
    before_usn: i64,
    records: &[ParsedUsnRecord],
    histories: &ReferenceHistories,
    visiting: &mut BTreeSet<FileReference>,
    open_reference: &OpenReference,
    append_name: &AppendName,
) -> Result<Path, ReferenceResolutionError<CallbackError>>
where
    OpenReference: Fn(FileReference) -> Result<Path, CallbackError>,
    AppendName: Fn(Path, &str) -> Result<Path, CallbackError>,
{
    if !visiting.insert(reference) || visiting.len() > 256 {
        return Err(ReferenceResolutionError::CycleOrDepth);
    }
    let historical = histories
        .get(&reference)
        .and_then(|indices| historical_parent_record(indices, records, before_usn));
    let result = if let Some(parent_record) = historical {
        let parent = resolve_reference_path_with(
            parent_record.parent_reference,
            parent_record.usn,
            records,
            histories,
            visiting,
            open_reference,
            append_name,
        )?;
        append_name(parent, &parent_record.name).map_err(ReferenceResolutionError::Callback)
    } else {
        open_reference(reference).map_err(ReferenceResolutionError::Callback)
    };
    visiting.remove(&reference);
    result
}

fn historical_parent_record<'a>(
    indices: &[usize],
    records: &'a [ParsedUsnRecord],
    before_usn: i64,
) -> Option<&'a ParsedUsnRecord> {
    indices
        .iter()
        .rev()
        .filter_map(|index| records.get(*index))
        .find(|record| record.usn < before_usn)
        .or_else(|| {
            indices
                .iter()
                .filter_map(|index| records.get(*index))
                .find(|record| {
                    record.usn >= before_usn
                        && record.reason & (USN_REASON_FILE_DELETE | USN_REASON_RENAME_OLD_NAME)
                            != 0
                })
        })
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
