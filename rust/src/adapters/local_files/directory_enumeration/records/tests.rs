use super::*;
use crate::domain::SourceRevisionEvidence;

fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i64(buffer: &mut [u8], offset: usize, value: i64) {
    buffer[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_record(buffer: &mut [u8], next_offset: u32, name: &str, attributes: u32, file_id: u128) {
    let name = name.encode_utf16().collect::<Vec<_>>();
    write_u32(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
        next_offset,
    );
    write_i64(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, LastWriteTime),
        WINDOWS_TO_UNIX_EPOCH_100NS + 42 * HUNDRED_NS_PER_MILLISECOND,
    );
    write_i64(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, ChangeTime),
        0x0123_4567_89ab_cdef,
    );
    write_i64(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, EndOfFile),
        7,
    );
    write_u32(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileAttributes),
        attributes,
    );
    write_u32(
        buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileNameLength),
        u32::try_from(name.len() * size_of::<u16>()).expect("name length"),
    );
    let id_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileId);
    buffer[id_offset..id_offset + 16].copy_from_slice(&file_id.to_le_bytes());
    let name_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
    for (index, unit) in name.into_iter().enumerate() {
        let offset = name_offset + index * size_of::<u16>();
        buffer[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
    }
}

#[test]
fn handle_directory_buffer_parser_preserves_metadata_and_rejects_malformed_offsets() {
    let mut buffer = vec![0_u8; 512];
    write_record(&mut buffer[..256], 256, "first.png", 0, 11);
    write_record(
        &mut buffer[256..],
        0,
        "nested",
        FILE_ATTRIBUTE_DIRECTORY,
        12,
    );

    let entries = parse_handle_directory_buffer(&buffer, 9, "album").expect("valid records");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].relative_path, "album/first.png");
    assert_eq!(entries[0].kind, MetadataInventoryEntryKind::File);
    assert_eq!(entries[0].file_size, Some(7));
    assert_eq!(entries[0].modified_unix_ms, 42);
    assert_eq!(
        entries[0].source_revision,
        Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0123456789abcdef".to_owned(),
        })
    );
    assert_eq!(
        entries[0]
            .file_identity
            .as_ref()
            .map(|identity| identity.value.as_str()),
        Some("0000000000000009:0000000000000000000000000000000b")
    );
    assert_eq!(entries[1].relative_path, "album/nested");
    assert_eq!(entries[1].kind, MetadataInventoryEntryKind::Directory);
    assert_eq!(
        entries[1]
            .file_identity
            .as_ref()
            .map(|identity| identity.value.as_str()),
        Some("0000000000000009:0000000000000000000000000000000c")
    );

    write_u32(
        &mut buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, NextEntryOffset),
        3,
    );
    assert!(parse_handle_directory_buffer(&buffer, 9, "album").is_err());
}

#[test]
fn record_parser_rejects_truncated_headers_and_names() {
    let mut valid = vec![0_u8; 256];
    write_record(&mut valid, 0, "photo.jpg", 0, 11);
    let header = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
    for length in [0, header - 1, header, header + 3] {
        let error = parse_handle_directory_buffer(&valid[..length], 9, "album")
            .expect_err("incomplete records cannot publish metadata");
        assert_eq!(error.code, "directory_entry_unreadable");
        assert_eq!(error.path.as_deref(), Some("album"));
    }
}

#[test]
fn record_parser_requires_one_valid_utf16_name_component() {
    for name in ["a/b.jpg", "a\\b.jpg", "a\0b.jpg"] {
        let mut buffer = vec![0_u8; 256];
        write_record(&mut buffer, 0, name, 0, 11);
        assert!(parse_handle_directory_buffer(&buffer, 9, "album").is_err());
    }
    let mut buffer = vec![0_u8; 256];
    write_record(&mut buffer, 0, "a", 0, 11);
    let name_offset = std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName);
    buffer[name_offset..name_offset + 2].copy_from_slice(&0xd800_u16.to_le_bytes());
    assert!(parse_handle_directory_buffer(&buffer, 9, "album").is_err());
}

#[test]
fn dot_entries_are_skipped_and_non_ascii_names_are_preserved() {
    let mut buffer = vec![0_u8; 768];
    write_record(&mut buffer[..256], 256, ".", FILE_ATTRIBUTE_DIRECTORY, 1);
    write_record(
        &mut buffer[256..512],
        256,
        "..",
        FILE_ATTRIBUTE_DIRECTORY,
        2,
    );
    write_record(&mut buffer[512..], 0, "照片.jpg", 0, 11);
    let entries = parse_handle_directory_buffer(&buffer, 9, "相册").expect("valid records");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].relative_path, "相册/照片.jpg");
}

#[test]
fn offline_entries_never_grant_available_file_identity() {
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_OFFLINE;

    for attributes in [
        FILE_ATTRIBUTE_OFFLINE,
        FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_DIRECTORY,
    ] {
        let mut buffer = vec![0_u8; 256];
        write_record(&mut buffer, 0, "remote", attributes, 11);
        let entries = parse_handle_directory_buffer(&buffer, 9, "").expect("offline metadata");
        assert_eq!(entries.len(), 1);
        assert_ne!(
            entries[0].placeholder_state,
            MetadataInventoryPlaceholderState::Available
        );
        assert!(entries[0].file_identity.is_none());
        assert!(entries[0].source_revision.is_none());
        if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            assert_eq!(entries[0].kind, MetadataInventoryEntryKind::Other);
        }
    }
}

#[test]
fn negative_file_size_is_rejected_before_metadata_publication() {
    let mut buffer = vec![0_u8; 256];
    write_record(&mut buffer, 0, "photo.jpg", 0, 11);
    write_i64(
        &mut buffer,
        std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, EndOfFile),
        -1,
    );
    let error = parse_handle_directory_buffer(&buffer, 9, "album").expect_err("invalid file size");
    assert_eq!(error.code, "directory_entry_unreadable");
}
