use std::collections::{BTreeMap, BTreeSet};

use windows_sys::Win32::System::Ioctl::{USN_REASON_FILE_DELETE, USN_REASON_RENAME_OLD_NAME};

mod parsing;

pub(crate) use parsing::{UsnParseError, decode_journal_buffer, parse_journal_buffer};

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
