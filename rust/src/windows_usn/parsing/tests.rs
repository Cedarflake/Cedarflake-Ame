use super::{DecodedUsnRecord, UsnParseError, decode_journal_buffer, parse_journal_buffer};
use crate::windows_usn::FileReference;

const END_USN: i64 = 8192;

// V2 redaction retains the 60-byte header and eight-byte record alignment on native NTFS.
const REDACTED_V2: [u8; 64] = [
    64, 0, 0, 0, 2, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 128, 8, 9, 10, 11, 12, 13, 14, 129, 0, 16, 0, 0,
    0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 2, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0,
    60, 0, 0, 0, 0, 0,
];

const REDACTED_V3: [u8; 80] = [
    80, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 128, 9, 10, 11, 12, 13, 14, 15, 130, 8, 9, 10,
    11, 12, 13, 14, 129, 16, 17, 18, 19, 20, 21, 22, 131, 0, 16, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0,
    0, 0, 0, 0, 2, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 76, 0, 0, 0, 0, 0,
];

fn page(records: &[&[u8]]) -> Vec<u8> {
    let mut bytes = END_USN.to_le_bytes().to_vec();
    for record in records {
        bytes.extend_from_slice(record);
    }
    bytes
}

fn decode(bytes: &[u8]) -> Result<(i64, Vec<DecodedUsnRecord>), UsnParseError> {
    decode_journal_buffer(bytes, Ok)
}

fn with_name(record: &[u8], name: &[u16]) -> Vec<u8> {
    let (header_size, length_offset) = match record[4] {
        2 => (60, 56),
        3 => (76, 72),
        _ => panic!("The fixture only defines V2 and V3 records"),
    };
    let mut output = record[..header_size].to_vec();
    for unit in name {
        output.extend_from_slice(&unit.to_le_bytes());
    }
    let record_size = output.len().div_ceil(8) * 8;
    output.resize(record_size, 0);
    output[..4].copy_from_slice(&u32::try_from(record_size).unwrap().to_le_bytes());
    output[length_offset..length_offset + 2]
        .copy_from_slice(&u16::try_from(name.len() * 2).unwrap().to_le_bytes());
    output
}

#[test]
fn redacted_v2_preserves_identity_and_metadata_without_inventing_a_name() {
    let (cursor, records) = decode(&page(&[&REDACTED_V2])).unwrap();
    assert_eq!(cursor, END_USN);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(
        record.file_reference,
        FileReference::V2([1, 2, 3, 4, 5, 6, 7, 128])
    );
    assert_eq!(
        record.parent_reference,
        FileReference::V2([8, 9, 10, 11, 12, 13, 14, 129])
    );
    assert_eq!(record.usn, 4096);
    assert_eq!(record.timestamp_100ns, 8192);
    assert_eq!(record.reason, 0x8000_0200);
    assert_eq!(record.file_attributes, 16);
    assert_eq!(record.name, None);
}

#[test]
fn redacted_v3_preserves_all_128_identity_bits() {
    let (_, records) = decode(&page(&[&REDACTED_V3])).unwrap();
    let record = &records[0];
    assert_eq!(
        record.file_reference,
        FileReference::V3([1, 2, 3, 4, 5, 6, 7, 128, 9, 10, 11, 12, 13, 14, 15, 130])
    );
    assert_eq!(
        record.parent_reference,
        FileReference::V3([
            8, 9, 10, 11, 12, 13, 14, 129, 16, 17, 18, 19, 20, 21, 22, 131
        ])
    );
    assert_eq!(record.usn, 4096);
    assert_eq!(record.reason, 0x8000_0200);
    assert_eq!(record.file_attributes, 16);
    assert_eq!(record.name, None);
}

#[test]
fn named_path_parser_still_rejects_every_redacted_version() {
    for record in [REDACTED_V2.as_slice(), REDACTED_V3.as_slice()] {
        assert_eq!(
            parse_journal_buffer(&page(&[record])).unwrap_err(),
            UsnParseError::NameInvalid
        );
    }
}

#[test]
fn named_v2_and_v3_keep_unicode_names_and_metadata() {
    let expected = "十月🌸.png";
    for record in [REDACTED_V2.as_slice(), REDACTED_V3.as_slice()] {
        let named = with_name(record, &expected.encode_utf16().collect::<Vec<_>>());
        let (_, decoded) = decode(&page(&[&named])).unwrap();
        let (cursor, parsed) = parse_journal_buffer(&page(&[&named])).unwrap();
        assert_eq!(cursor, END_USN);
        assert_eq!(decoded[0].name.as_deref(), Some(expected));
        assert_eq!(parsed[0].name, expected);
        assert_eq!(parsed[0].file_reference, decoded[0].file_reference);
        assert_eq!(parsed[0].parent_reference, decoded[0].parent_reference);
        assert_eq!(parsed[0].timestamp_100ns, 8192);
        assert_eq!(parsed[0].reason, 0x8000_0200);
        assert_eq!(parsed[0].file_attributes, 16);
    }
}

#[test]
fn mixed_page_never_publishes_partial_named_records() {
    let named = with_name(
        &REDACTED_V2,
        &"image.png".encode_utf16().collect::<Vec<_>>(),
    );
    for records in [
        [named.as_slice(), REDACTED_V3.as_slice()],
        [REDACTED_V3.as_slice(), named.as_slice()],
    ] {
        let bytes = page(&records);
        assert_eq!(decode(&bytes).unwrap().1.len(), 2);
        assert_eq!(
            parse_journal_buffer(&bytes).unwrap_err(),
            UsnParseError::NameInvalid
        );
    }
}

#[test]
fn malformed_name_boundaries_are_not_redaction() {
    for (source, length_offset, header) in
        [(&REDACTED_V2[..], 56, 60_u16), (&REDACTED_V3[..], 72, 76)]
    {
        for (length, offset) in [
            (0_u16, 0_u16),
            (0, header - 2),
            (0, header + 1),
            (0, 65534),
            (1, header),
            (65534, header),
        ] {
            let mut malformed = source.to_vec();
            malformed[length_offset..length_offset + 2].copy_from_slice(&length.to_le_bytes());
            malformed[length_offset + 2..length_offset + 4].copy_from_slice(&offset.to_le_bytes());
            assert_eq!(
                decode(&page(&[&malformed])).unwrap_err(),
                UsnParseError::RecordMalformed
            );
        }
    }
}

#[test]
fn invalid_utf16_and_path_components_remain_rejected() {
    let invalid_names = [".", "..", "a/b", "a\\b", "a\0b"];
    for source in [REDACTED_V2.as_slice(), REDACTED_V3.as_slice()] {
        let mut names = invalid_names
            .iter()
            .map(|name| name.encode_utf16().collect::<Vec<_>>())
            .collect::<Vec<_>>();
        names.push(vec![0xd800]);
        for name in names {
            let bytes = page(&[&with_name(source, &name)]);
            assert_eq!(decode(&bytes).unwrap_err(), UsnParseError::NameInvalid);
            assert_eq!(
                parse_journal_buffer(&bytes).unwrap_err(),
                UsnParseError::NameInvalid
            );
        }
    }
}

#[test]
fn record_lengths_and_v3_header_truncation_remain_rejected() {
    for length in [0_u32, 56, 62, 72, u32::MAX] {
        let mut malformed = REDACTED_V2;
        malformed[..4].copy_from_slice(&length.to_le_bytes());
        assert_eq!(
            decode(&page(&[&malformed])).unwrap_err(),
            UsnParseError::RecordMalformed
        );
    }
    let mut malformed = REDACTED_V3[..72].to_vec();
    malformed[..4].copy_from_slice(&72_u32.to_le_bytes());
    assert_eq!(
        decode(&page(&[&malformed])).unwrap_err(),
        UsnParseError::RecordMalformed
    );
}

#[test]
fn truncated_cursor_and_trailing_bytes_cannot_return_a_page() {
    for length in 0..8 {
        assert_eq!(
            decode(&[0; 8][..length]).unwrap_err(),
            UsnParseError::BufferMalformed
        );
    }
    for length in 1..8 {
        let mut bytes = page(&[&REDACTED_V2]);
        bytes.extend_from_slice(&[0; 8][..length]);
        assert_eq!(decode(&bytes).unwrap_err(), UsnParseError::BufferMalformed);
    }
}

#[test]
fn unsupported_record_versions_and_negative_usns_remain_rejected() {
    let mut unsupported = REDACTED_V2;
    unsupported[4] = 4;
    assert_eq!(
        decode(&page(&[&unsupported])).unwrap_err(),
        UsnParseError::VersionUnsupported
    );
    for (source, offset) in [(&REDACTED_V2[..], 24), (&REDACTED_V3[..], 40)] {
        let mut negative = source.to_vec();
        negative[offset..offset + 8].copy_from_slice(&(-1_i64).to_le_bytes());
        assert_eq!(
            decode(&page(&[&negative])).unwrap_err(),
            UsnParseError::RecordMalformed
        );
    }
}

#[test]
fn empty_page_retains_its_exact_cursor() {
    let (cursor, records) = decode(&page(&[])).unwrap();
    assert_eq!(cursor, END_USN);
    assert!(records.is_empty());
    assert!(parse_journal_buffer(&page(&[])).unwrap().1.is_empty());
}

#[test]
fn rejected_record_admission_cannot_return_a_successful_cursor() {
    let result = decode_journal_buffer(&page(&[&REDACTED_V2]), |_| {
        Err::<(), _>(UsnParseError::NameInvalid)
    });
    assert_eq!(result.unwrap_err(), UsnParseError::NameInvalid);
}
