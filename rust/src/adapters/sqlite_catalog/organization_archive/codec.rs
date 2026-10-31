use std::io::{BufRead, Read, Write};

use serde::{Deserialize, Serialize};

use crate::domain::ScanError;

use super::{error, io_error, records};

pub(super) const FORMAT: &str = "ame-organization-archive";
pub(super) const VERSION: u32 = 5;
pub(super) const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    pub format: String,
    pub version: u32,
    pub archive_id: String,
    pub created_unix_ms: u64,
    pub source_schema: i64,
    pub organization_revision: i64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum Cell {
    Null,
    Integer(i64),
    Text(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Entry {
    Header { header: Header },
    Row { kind: String, values: Vec<Cell> },
    Footer { counts: Vec<u64>, checksum: String },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WrittenEntry<'a> {
    Header { header: &'a Header },
    Row { kind: &'a str, values: &'a [Cell] },
    Footer { counts: &'a [u64], checksum: String },
}

pub(super) struct ArchiveWriter<W> {
    writer: W,
    hash: blake3::Hasher,
    version: u32,
}

impl<W: Write> ArchiveWriter<W> {
    pub fn new(writer: W, header: &Header) -> Result<Self, ScanError> {
        validate_header(header)?;
        let mut archive = Self {
            writer,
            hash: blake3::Hasher::new(),
            version: header.version,
        };
        archive.append(&WrittenEntry::Header { header }, true)?;
        Ok(archive)
    }

    pub fn row(&mut self, kind_index: usize, values: &[Cell]) -> Result<(), ScanError> {
        if kind_index >= records::kind_count(self.version) {
            return Err(invalid());
        }
        let kind = records::KINDS.get(kind_index).ok_or_else(invalid)?;
        validate_values(kind, values, self.version)?;
        self.append(
            &WrittenEntry::Row {
                kind: kind.name,
                values,
            },
            true,
        )
    }

    pub fn finish(mut self, counts: &[u64]) -> Result<W, ScanError> {
        let count = records::kind_count(self.version);
        if counts.len() < count
            || counts.len() > records::KINDS.len()
            || counts[count..].iter().any(|value| *value != 0)
        {
            return Err(invalid());
        }
        self.append(
            &WrittenEntry::Footer {
                counts: &counts[..count],
                checksum: self.hash.finalize().to_hex().to_string(),
            },
            false,
        )?;
        Ok(self.writer)
    }

    fn append(&mut self, entry: &WrittenEntry<'_>, hash: bool) -> Result<(), ScanError> {
        let mut buffer = LineBuffer(Vec::new());
        serde_json::to_writer(&mut buffer, entry).map_err(|_| {
            error(
                "record_too_large",
                "An archive record cannot be encoded within its size limit",
            )
        })?;
        let mut bytes = buffer.0;
        bytes.push(b'\n');
        if bytes.len() > MAX_LINE_BYTES {
            return Err(error(
                "record_too_large",
                "An archive record exceeds the supported size",
            ));
        }
        self.writer.write_all(&bytes).map_err(io_error)?;
        if hash {
            self.hash.update(&bytes);
        }
        Ok(())
    }
}

struct LineBuffer(Vec<u8>);

impl Write for LineBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_LINE_BYTES - 1 - self.0.len() {
            return Err(std::io::Error::other("Archive line exceeds its byte limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) struct VerifiedArchive {
    pub header: Header,
    pub checksum: String,
    pub record_count: u64,
}

pub(super) fn validate(
    input: impl BufRead,
    progress: &mut dyn FnMut(u64) -> Result<(), ScanError>,
) -> Result<VerifiedArchive, ScanError> {
    read_records(input, &mut |_, _| Ok(()), progress)
}

pub(super) fn read_records(
    mut input: impl BufRead,
    accept: &mut impl FnMut(usize, &[Cell]) -> Result<(), ScanError>,
    progress: &mut dyn FnMut(u64) -> Result<(), ScanError>,
) -> Result<VerifiedArchive, ScanError> {
    let mut bytes = read_line(&mut input)?;
    let Entry::Header { header } = decode(&bytes)? else {
        return Err(invalid());
    };
    validate_header(&header)?;
    let mut hash = blake3::Hasher::new();
    hash.update(&bytes);
    let mut counts = vec![0_u64; records::kind_count(header.version)];
    let mut last_kind = 0;
    let mut total = 0_u64;
    loop {
        bytes = read_line(&mut input)?;
        match decode(&bytes)? {
            Entry::Header { .. } => return Err(invalid()),
            Entry::Row { kind, mut values } => {
                let index = records::KINDS
                    .iter()
                    .position(|spec| spec.name == kind)
                    .ok_or_else(invalid)?;
                if index < last_kind || index >= counts.len() {
                    return Err(invalid());
                }
                validate_values(&records::KINDS[index], &values, header.version)?;
                compatibility::upgrade(&kind, &mut values, header.version);
                accept(index, &values)?;
                counts[index] = counts[index].checked_add(1).ok_or_else(invalid)?;
                total = total.checked_add(1).ok_or_else(invalid)?;
                last_kind = index;
                hash.update(&bytes);
                if total.is_multiple_of(256) {
                    progress(total)?;
                }
            }
            Entry::Footer {
                counts: expected,
                checksum,
            } => {
                if expected != counts
                    || checksum != hash.finalize().to_hex().as_str()
                    || !input.fill_buf().map_err(io_error)?.is_empty()
                {
                    return Err(invalid());
                }
                return Ok(VerifiedArchive {
                    header,
                    checksum,
                    record_count: total,
                });
            }
        }
    }
}

fn read_line(input: &mut impl BufRead) -> Result<Vec<u8>, ScanError> {
    let mut bytes = Vec::new();
    input
        .take(MAX_LINE_BYTES as u64 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(io_error)?;
    if bytes.len() > MAX_LINE_BYTES || bytes.last() != Some(&b'\n') {
        return Err(invalid());
    }
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Result<Entry, ScanError> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}

fn validate_header(header: &Header) -> Result<(), ScanError> {
    if header.format != FORMAT || !(1..=VERSION).contains(&header.version) {
        return Err(error(
            "version_unsupported",
            "This organization archive format is not supported",
        ));
    }
    if header.archive_id.len() != 32
        || !header
            .archive_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || header.source_schema < 52
        || header.organization_revision < 0
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_values(
    kind: &records::RecordKind,
    values: &[Cell],
    version: u32,
) -> Result<(), ScanError> {
    let fields = compatibility::fields(kind, version);
    if fields.len() != values.len()
        || !fields
            .iter()
            .zip(values)
            .all(|(field, value)| field.accepts(value))
    {
        return Err(invalid());
    }
    if version >= 2
        && kind.name == "version_evidence"
        && !matches!(values.last(), Some(Cell::Integer(count)) if *count > 0)
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    error(
        "invalid",
        "The organization archive is incomplete or invalid",
    )
}

#[cfg(test)]
mod tests;

mod compatibility;
