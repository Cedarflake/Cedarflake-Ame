use std::mem::size_of;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ID_EXTD_DIR_INFO,
};

use crate::domain::{
    FileIdentityEvidence, MetadataInventoryEntry, MetadataInventoryEntryKind,
    MetadataInventoryPlaceholderState, ScanIssue,
};

use super::super::{
    ReparseKind, metadata_inventory_entry_kind, metadata_placeholder_state_from_attributes,
    reparse_evidence_from_attribute_tag, windows_source_revision,
};

const WINDOWS_TO_UNIX_EPOCH_100NS: i64 = 116_444_736_000_000_000;

const HUNDRED_NS_PER_MILLISECOND: i64 = 10_000;

pub(super) fn parse_handle_directory_buffer(
    buffer: &[u8],
    volume_serial: u64,
    relative_directory: &str,
) -> Result<Vec<MetadataInventoryEntry>, ScanIssue> {
    let header_size = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
    let mut offset = 0_usize;
    let mut entries = Vec::new();
    loop {
        let record = buffer.get(offset..).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid record offset",
            )
        })?;
        if record.len() < header_size {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a truncated record header",
            ));
        }
        let next_offset = read_u32_field(
            record,
            std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
        )? as usize;
        let name_length = read_u32_field(
            record,
            std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileNameLength),
        )? as usize;
        if name_length == 0 || !name_length.is_multiple_of(size_of::<u16>()) {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid UTF-16 name length",
            ));
        }
        let record_extent = if next_offset == 0 {
            record.len()
        } else {
            if next_offset < header_size
                || next_offset > record.len()
                || !next_offset.is_multiple_of(std::mem::align_of::<FILE_ID_EXTD_DIR_INFO>())
            {
                return Err(directory_enumeration_issue(
                    relative_directory,
                    "The directory handle returned an invalid continuation offset",
                ));
            }
            next_offset
        };
        let name_end = header_size.checked_add(name_length).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle name length overflowed",
            )
        })?;
        if name_end > record_extent {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a truncated UTF-16 name",
            ));
        }
        let name_bytes = &record[header_size..name_end];
        let name_units = name_bytes
            .chunks_exact(size_of::<u16>())
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .collect::<Vec<_>>();
        let name = String::from_utf16(&name_units).map_err(|_| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned an invalid UTF-16 name",
            )
        })?;
        if name.contains(['\0', '/', '\\']) {
            return Err(directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a name that is not one path component",
            ));
        }
        if name != "." && name != ".." {
            let attributes = read_u32_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileAttributes),
            )?;
            let reparse_tag = read_u32_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ReparsePointTag),
            )?;
            let end_of_file = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, EndOfFile),
            )?;
            let modified_100ns = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, LastWriteTime),
            )?;
            let change_time_100ns = read_i64_field(
                record,
                std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ChangeTime),
            )?;
            let file_id_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileId);
            let file_id_bytes: [u8; 16] = record
                .get(file_id_offset..file_id_offset + 16)
                .and_then(|value| value.try_into().ok())
                .ok_or_else(|| {
                    directory_enumeration_issue(
                        relative_directory,
                        "The directory handle returned a truncated file identity",
                    )
                })?;
            entries.push(metadata_inventory_entry_from_handle_record(
                relative_directory,
                &name,
                attributes,
                reparse_tag,
                end_of_file,
                modified_100ns,
                change_time_100ns,
                volume_serial,
                file_id_bytes,
            )?);
        }
        if next_offset == 0 {
            break;
        }
        offset = offset.checked_add(next_offset).ok_or_else(|| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle continuation offset overflowed",
            )
        })?;
    }
    Ok(entries)
}

#[allow(clippy::too_many_arguments)]
fn metadata_inventory_entry_from_handle_record(
    relative_directory: &str,
    name: &str,
    attributes: u32,
    reparse_tag: u32,
    end_of_file: i64,
    modified_100ns: i64,
    change_time_100ns: i64,
    volume_serial: u64,
    file_id_bytes: [u8; 16],
) -> Result<MetadataInventoryEntry, ScanIssue> {
    let relative_path = if relative_directory.is_empty() {
        name.to_owned()
    } else {
        format!("{relative_directory}/{name}")
    };
    let (reparse_kind, placeholder_state) = if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        (
            ReparseKind::None,
            metadata_placeholder_state_from_attributes(attributes),
        )
    } else {
        reparse_evidence_from_attribute_tag(attributes, reparse_tag).map_err(|error| ScanIssue {
            path: Some(relative_path.clone()),
            code: "file_reparse_evidence_unreadable".to_owned(),
            message: error.to_string(),
        })?
    };
    let is_directory = attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
    let is_reparse_point = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
    let is_opaque_directory = is_directory
        && (is_reparse_point || placeholder_state != MetadataInventoryPlaceholderState::Available);
    let kind = if reparse_kind == ReparseKind::Other || is_opaque_directory {
        MetadataInventoryEntryKind::Other
    } else {
        metadata_inventory_entry_kind(is_directory, !is_directory, reparse_kind)
    };
    let file_size = if kind == MetadataInventoryEntryKind::File {
        Some(u64::try_from(end_of_file).map_err(|_| {
            directory_enumeration_issue(
                relative_directory,
                "The directory handle returned a negative file size",
            )
        })?)
    } else {
        None
    };
    let file_identity = (matches!(
        kind,
        MetadataInventoryEntryKind::File | MetadataInventoryEntryKind::Directory
    ) && placeholder_state == MetadataInventoryPlaceholderState::Available)
        .then(|| FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".to_owned(),
            value: format!(
                "{volume_serial:016x}:{:032x}",
                u128::from_le_bytes(file_id_bytes)
            ),
        });
    Ok(MetadataInventoryEntry {
        relative_path,
        kind,
        file_size,
        modified_unix_ms: windows_file_time_to_unix_ms(modified_100ns),
        file_identity,
        source_revision: (kind == MetadataInventoryEntryKind::File
            && placeholder_state == MetadataInventoryPlaceholderState::Available)
            .then(|| windows_source_revision(change_time_100ns)),
        placeholder_state,
        is_reparse_point,
    })
}

fn read_u32_field(record: &[u8], offset: usize) -> Result<u32, ScanIssue> {
    let value: [u8; 4] = record
        .get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| {
            directory_enumeration_issue("", "The directory handle record is truncated")
        })?;
    Ok(u32::from_le_bytes(value))
}

fn read_i64_field(record: &[u8], offset: usize) -> Result<i64, ScanIssue> {
    let value: [u8; 8] = record
        .get(offset..offset + 8)
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| {
            directory_enumeration_issue("", "The directory handle record is truncated")
        })?;
    Ok(i64::from_le_bytes(value))
}

fn windows_file_time_to_unix_ms(value: i64) -> i64 {
    value
        .saturating_sub(WINDOWS_TO_UNIX_EPOCH_100NS)
        .checked_div(HUNDRED_NS_PER_MILLISECOND)
        .unwrap_or(0)
        .max(0)
}

pub(super) fn directory_enumeration_issue(relative_directory: &str, message: &str) -> ScanIssue {
    ScanIssue {
        path: (!relative_directory.is_empty()).then(|| relative_directory.to_owned()),
        code: "directory_entry_unreadable".to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests;
