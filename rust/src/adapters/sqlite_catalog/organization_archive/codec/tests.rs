use std::io::Cursor;

use super::*;

fn header() -> Header {
    Header {
        format: FORMAT.into(),
        version: VERSION,
        archive_id: "a".repeat(32),
        created_unix_ms: 1,
        source_schema: 52,
        organization_revision: 3,
    }
}

fn archive() -> Vec<u8> {
    let mut writer = ArchiveWriter::new(Vec::new(), &header()).unwrap();
    writer
        .row(
            0,
            &[
                Cell::Text("album-a".into()),
                Cell::Text("风景🌲\n雪".into()),
                Cell::Integer(3),
                Cell::Integer(1),
                Cell::Integer(2),
            ],
        )
        .unwrap();
    let mut counts = vec![0; records::KINDS.len()];
    counts[0] = 1;
    writer.finish(&counts).unwrap()
}

#[test]
fn archive_codec_preserves_unicode_and_accepts_empty_completion() {
    assert_eq!(
        validate(Cursor::new(archive()), &mut |_| Ok(()))
            .unwrap()
            .record_count,
        1
    );
    let empty = ArchiveWriter::new(Vec::new(), &header())
        .unwrap()
        .finish(&vec![0; records::KINDS.len()])
        .unwrap();
    assert_eq!(
        validate(Cursor::new(empty), &mut |_| Ok(()))
            .unwrap()
            .record_count,
        0
    );
}

#[test]
fn archive_codec_rejects_every_truncated_prefix() {
    let bytes = archive();
    for end in 0..bytes.len() {
        assert!(
            validate(Cursor::new(&bytes[..end]), &mut |_| Ok(())).is_err(),
            "prefix {end}"
        );
    }
}

#[test]
fn archive_codec_rejects_tampering_trailing_data_and_wrong_counts() {
    let text = String::from_utf8(archive()).unwrap();
    for altered in [
        text.replace("风景", "湖面"),
        format!("{text}{{}}\n"),
        text.replace("\"counts\":[1,", "\"counts\":[0,"),
    ] {
        assert!(validate(Cursor::new(altered), &mut |_| Ok(())).is_err());
    }
}

#[test]
fn archive_codec_rejects_future_versions_unknown_fields_and_kinds() {
    let text = String::from_utf8(archive()).unwrap();
    for altered in [
        text.replace("\"version\":3", "\"version\":4"),
        text.replace(
            "\"source_schema\":52",
            "\"source_schema\":52,\"script\":\"ignored\"",
        ),
        text.replace("\"kind\":\"album\"", "\"kind\":\"future_kind\""),
        text.replace("\"values\":[", "\"values\":[null,"),
    ] {
        assert!(validate(Cursor::new(altered), &mut |_| Ok(())).is_err());
    }
}

#[test]
fn archive_codec_bounds_unterminated_lines_and_field_types() {
    assert!(validate(Cursor::new(vec![b' '; MAX_LINE_BYTES + 1]), &mut |_| Ok(())).is_err());
    let mut writer = ArchiveWriter::new(Vec::new(), &header()).unwrap();
    assert!(writer.row(0, &[Cell::Text("a".into())]).is_err());
    assert!(
        writer
            .row(
                0,
                &[
                    Cell::Null,
                    Cell::Text("a".into()),
                    Cell::Integer(1),
                    Cell::Integer(1),
                    Cell::Integer(1)
                ]
            )
            .is_err()
    );
    assert!(
        writer
            .row(
                0,
                &[
                    Cell::Text("a".into()),
                    Cell::Text("x".repeat(MAX_LINE_BYTES)),
                    Cell::Integer(1),
                    Cell::Integer(1),
                    Cell::Integer(1)
                ]
            )
            .is_err()
    );
}

#[test]
fn archive_codec_propagates_read_write_and_cancellation_failures() {
    struct FailedReader;
    impl Read for FailedReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("Unavailable input"))
        }
    }
    impl BufRead for FailedReader {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            Err(std::io::Error::other("Unavailable input"))
        }
        fn consume(&mut self, _: usize) {}
    }
    assert_eq!(
        validate(FailedReader, &mut |_| Ok(())).err().unwrap().code,
        "organization_archive_io"
    );
    let mut capacity = [0_u8; 1];
    let failing_writer = std::io::Cursor::new(&mut capacity[..]);
    assert!(ArchiveWriter::new(failing_writer, &header()).is_err());
    let mut writer = ArchiveWriter::new(Vec::new(), &header()).unwrap();
    for index in 0..256 {
        writer
            .row(
                0,
                &[
                    Cell::Text(index.to_string()),
                    Cell::Text("a".into()),
                    Cell::Integer(1),
                    Cell::Integer(1),
                    Cell::Integer(1),
                ],
            )
            .unwrap();
    }
    let mut counts = vec![0; records::KINDS.len()];
    counts[0] = 256;
    let bytes = writer.finish(&counts).unwrap();
    let failure = validate(Cursor::new(bytes), &mut |_| Err(error("cancelled", "stop")));
    assert_eq!(
        failure.err().unwrap().code,
        "organization_archive_cancelled"
    );
}
