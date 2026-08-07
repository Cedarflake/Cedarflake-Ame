use exif::{DateTime, Exif, In, Reader, Tag, Value};

use crate::domain::{CaptureTimeEvidence, CaptureTimeSource, MetadataInspection, ScanIssue};
use crate::ports::MetadataExtractor;

pub(crate) const METADATA_ENGINE_ID: &str = "kamadak-exif";
pub(crate) const METADATA_ENGINE_VERSION: &str = "0.6.1";
const MAX_EXIF_BYTES: usize = 4 * 1024 * 1024;
const MAX_CAPTURE_FIELD_BYTES: usize = 64;

#[flutter_rust_bridge::frb(opaque)]
pub struct KamadakExifExtractor;

impl MetadataExtractor for KamadakExifExtractor {
    #[flutter_rust_bridge::frb(ignore)]
    fn engine_id(&self) -> &'static str {
        METADATA_ENGINE_ID
    }

    #[flutter_rust_bridge::frb(ignore)]
    fn engine_version(&self) -> &'static str {
        METADATA_ENGINE_VERSION
    }

    #[flutter_rust_bridge::frb(ignore)]
    fn extract(&self, raw_exif: Option<&[u8]>, source_path: &str) -> MetadataInspection {
        let mut inspection = MetadataInspection {
            engine_id: self.engine_id().to_owned(),
            engine_version: self.engine_version().to_owned(),
            capture_time: None,
            issues: Vec::new(),
        };
        let Some(raw_exif) = raw_exif else {
            return inspection;
        };
        if raw_exif.len() > MAX_EXIF_BYTES {
            inspection.issues.push(metadata_issue(
                source_path,
                "metadata_size_exceeded",
                format!("EXIF metadata exceeds the {MAX_EXIF_BYTES}-byte parsing limit"),
            ));
            return inspection;
        }
        let exif = match Reader::new().read_raw(raw_exif.to_vec()) {
            Ok(exif) => exif,
            Err(error) => {
                inspection
                    .issues
                    .push(metadata_issue(source_path, "metadata_parse_failed", error));
                return inspection;
            }
        };

        let candidates = [
            CaptureCandidate {
                date_tag: Tag::DateTimeOriginal,
                subsecond_tag: Tag::SubSecTimeOriginal,
                offset_tag: Tag::OffsetTimeOriginal,
                source: CaptureTimeSource::Original,
            },
            CaptureCandidate {
                date_tag: Tag::DateTimeDigitized,
                subsecond_tag: Tag::SubSecTimeDigitized,
                offset_tag: Tag::OffsetTimeDigitized,
                source: CaptureTimeSource::Digitized,
            },
            CaptureCandidate {
                date_tag: Tag::DateTime,
                subsecond_tag: Tag::SubSecTime,
                offset_tag: Tag::OffsetTime,
                source: CaptureTimeSource::Image,
            },
        ];
        for candidate in candidates {
            match capture_time(&exif, &candidate) {
                Ok(Some(capture_time)) => {
                    inspection.capture_time = Some(capture_time);
                    break;
                }
                Ok(None) => {}
                Err(message) => inspection.issues.push(metadata_issue(
                    source_path,
                    "capture_time_invalid",
                    message,
                )),
            }
        }
        inspection
    }
}

struct CaptureCandidate {
    date_tag: Tag,
    subsecond_tag: Tag,
    offset_tag: Tag,
    source: CaptureTimeSource,
}

fn capture_time(
    exif: &Exif,
    candidate: &CaptureCandidate,
) -> Result<Option<CaptureTimeEvidence>, String> {
    let Some(date_raw) = ascii_field(exif, candidate.date_tag)? else {
        return Ok(None);
    };
    let mut date_time = DateTime::from_ascii(&date_raw)
        .map_err(|error| format!("{} is invalid: {error}", candidate.date_tag))?;
    validate_date_time(&date_time)?;

    let subsecond_raw = ascii_field(exif, candidate.subsecond_tag)?;
    if let Some(raw) = &subsecond_raw {
        if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) {
            return Err(format!(
                "{} must contain only decimal digits",
                candidate.subsecond_tag
            ));
        }
        date_time
            .parse_subsec(raw)
            .map_err(|error| format!("{} is invalid: {error}", candidate.subsecond_tag))?;
    }
    let offset_raw = ascii_field(exif, candidate.offset_tag)?;
    if let Some(raw) = &offset_raw {
        validate_offset_ascii(raw, candidate.offset_tag)?;
        date_time
            .parse_offset(raw)
            .map_err(|error| format!("{} is invalid: {error}", candidate.offset_tag))?;
    }
    validate_date_time(&date_time)?;

    let nanosecond = date_time.nanosecond.unwrap_or(0);
    let local_time = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{nanosecond:09}",
        date_time.year,
        date_time.month,
        date_time.day,
        date_time.hour,
        date_time.minute,
        date_time.second,
    );
    let raw_value = [
        String::from_utf8_lossy(&date_raw).into_owned(),
        subsecond_raw
            .as_deref()
            .map(String::from_utf8_lossy)
            .map(|value| value.into_owned())
            .unwrap_or_default(),
        offset_raw
            .as_deref()
            .map(String::from_utf8_lossy)
            .map(|value| value.into_owned())
            .unwrap_or_default(),
    ]
    .join("|");

    Ok(Some(CaptureTimeEvidence {
        local_time,
        offset_minutes: date_time.offset,
        source: candidate.source.clone(),
        raw_value,
    }))
}

fn ascii_field(exif: &Exif, tag: Tag) -> Result<Option<Vec<u8>>, String> {
    let Some(field) = exif.get_field(tag, In::PRIMARY) else {
        return Ok(None);
    };
    let Value::Ascii(values) = &field.value else {
        return Err(format!("{tag} is not an ASCII field"));
    };
    let Some(value) = values.first() else {
        return Err(format!("{tag} is empty"));
    };
    if value.len() > MAX_CAPTURE_FIELD_BYTES {
        return Err(format!(
            "{tag} exceeds the {MAX_CAPTURE_FIELD_BYTES}-byte evidence limit"
        ));
    }
    Ok(Some(value.clone()))
}

fn validate_date_time(value: &DateTime) -> Result<(), String> {
    if value.year == 0 || value.month == 0 || value.month > 12 {
        return Err("capture date has an invalid year or month".to_owned());
    }
    let days = days_in_month(value.year, value.month);
    if value.day == 0 || value.day > days {
        return Err("capture date has an invalid day".to_owned());
    }
    if value.hour > 23 || value.minute > 59 || value.second > 59 {
        return Err("capture time has an invalid clock value".to_owned());
    }
    if value
        .nanosecond
        .is_some_and(|nanosecond| nanosecond >= 1_000_000_000)
    {
        return Err("capture time has an invalid subsecond value".to_owned());
    }
    if value
        .offset
        .is_some_and(|offset| offset.unsigned_abs() > 14 * 60)
    {
        return Err("capture time has an invalid timezone offset".to_owned());
    }
    Ok(())
}

fn validate_offset_ascii(value: &[u8], tag: Tag) -> Result<(), String> {
    if value.len() != 6
        || !matches!(value[0], b'+' | b'-')
        || value[3] != b':'
        || !value[1..3].iter().all(u8::is_ascii_digit)
        || !value[4..6].iter().all(u8::is_ascii_digit)
    {
        return Err(format!("{tag} must use the EXIF +HH:MM or -HH:MM form"));
    }
    let hour = u16::from(value[1] - b'0') * 10 + u16::from(value[2] - b'0');
    let minute = u16::from(value[4] - b'0') * 10 + u16::from(value[5] - b'0');
    if minute > 59 || hour > 14 || (hour == 14 && minute != 0) {
        return Err(format!("{tag} is outside the supported timezone range"));
    }
    Ok(())
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn metadata_issue(source_path: &str, code: &str, message: impl std::fmt::Display) -> ScanIssue {
    ScanIssue {
        path: Some(source_path.to_owned()),
        code: code.to_owned(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use exif::experimental::Writer;
    use exif::{Field, In};

    use super::*;

    #[test]
    fn extracts_original_time_with_subseconds_and_offset() {
        let fields = [
            ascii_field_fixture(Tag::DateTimeOriginal, b"2024:02:29 23:59:58"),
            ascii_field_fixture(Tag::SubSecTimeOriginal, b"1234"),
            ascii_field_fixture(Tag::OffsetTimeOriginal, b"+08:00"),
        ];
        let raw = encode_exif(&fields);

        let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\photo.jpg");
        let capture = inspection.capture_time.expect("capture time");

        assert_eq!(capture.local_time, "2024-02-29T23:59:58.123400000");
        assert_eq!(capture.offset_minutes, Some(480));
        assert!(matches!(capture.source, CaptureTimeSource::Original));
        assert_eq!(capture.raw_value, "2024:02:29 23:59:58|1234|+08:00");
        assert!(inspection.issues.is_empty());
    }

    #[test]
    fn falls_back_after_an_invalid_higher_priority_timestamp() {
        let fields = [
            ascii_field_fixture(Tag::DateTimeOriginal, b"2023:02:29 12:00:00"),
            ascii_field_fixture(Tag::DateTimeDigitized, b"2023:03:01 12:00:00"),
        ];
        let raw = encode_exif(&fields);

        let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\photo.jpg");
        let capture = inspection.capture_time.expect("fallback capture time");

        assert_eq!(capture.local_time, "2023-03-01T12:00:00.000000000");
        assert!(matches!(capture.source, CaptureTimeSource::Digitized));
        assert_eq!(inspection.issues.len(), 1);
        assert_eq!(inspection.issues[0].code, "capture_time_invalid");
    }

    #[test]
    fn uses_generic_image_time_when_capture_specific_tags_are_absent() {
        let raw = encode_exif(&[ascii_field_fixture(Tag::DateTime, b"2020:12:31 23:59:59")]);

        let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\photo.jpg");
        let capture = inspection.capture_time.expect("generic capture time");

        assert_eq!(capture.local_time, "2020-12-31T23:59:59.000000000");
        assert!(matches!(capture.source, CaptureTimeSource::Image));
        assert!(inspection.issues.is_empty());
    }

    #[test]
    fn rejects_out_of_range_calendar_and_clock_values() {
        for value in [
            b"2024:13:01 00:00:00".as_slice(),
            b"2024:04:31 00:00:00".as_slice(),
            b"2023:02:29 00:00:00".as_slice(),
            b"2024:01:01 24:00:00".as_slice(),
            b"2024:01:01 23:60:00".as_slice(),
            b"2024:01:01 23:59:60".as_slice(),
        ] {
            let raw = encode_exif(&[ascii_field_fixture(Tag::DateTimeOriginal, value)]);
            let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\photo.jpg");

            assert!(inspection.capture_time.is_none(), "accepted {value:?}");
            assert_eq!(inspection.issues.len(), 1);
            assert_eq!(inspection.issues[0].code, "capture_time_invalid");
        }
    }

    #[test]
    fn rejects_invalid_subseconds_and_offsets_without_inventing_evidence() {
        let invalid_subsecond = encode_exif(&[
            ascii_field_fixture(Tag::DateTimeOriginal, b"2024:01:01 00:00:00"),
            ascii_field_fixture(Tag::SubSecTimeOriginal, b"12x"),
        ]);
        let invalid_offset = encode_exif(&[
            ascii_field_fixture(Tag::DateTimeOriginal, b"2024:01:01 00:00:00"),
            ascii_field_fixture(Tag::OffsetTimeOriginal, b"+12:99"),
        ]);

        for raw in [invalid_subsecond, invalid_offset] {
            let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\photo.jpg");
            assert!(inspection.capture_time.is_none());
            assert_eq!(inspection.issues.len(), 1);
            assert_eq!(inspection.issues[0].code, "capture_time_invalid");
        }
    }

    #[test]
    fn treats_absent_metadata_as_normal_and_malformed_metadata_as_an_issue() {
        let absent = KamadakExifExtractor.extract(None, "C:\\plain.png");
        assert!(absent.capture_time.is_none());
        assert!(absent.issues.is_empty());

        let malformed = KamadakExifExtractor.extract(Some(b"not exif"), "C:\\broken.jpg");
        assert!(malformed.capture_time.is_none());
        assert_eq!(malformed.issues.len(), 1);
        assert_eq!(malformed.issues[0].code, "metadata_parse_failed");
    }

    #[test]
    fn rejects_oversized_metadata_before_parsing() {
        let raw = vec![0; MAX_EXIF_BYTES + 1];

        let inspection = KamadakExifExtractor.extract(Some(&raw), "C:\\oversized.jpg");

        assert!(inspection.capture_time.is_none());
        assert_eq!(inspection.issues.len(), 1);
        assert_eq!(inspection.issues[0].code, "metadata_size_exceeded");
    }

    fn ascii_field_fixture(tag: Tag, value: &[u8]) -> Field {
        Field {
            tag,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![value.to_vec()]),
        }
    }

    fn encode_exif(fields: &[Field]) -> Vec<u8> {
        let mut writer = Writer::new();
        for field in fields {
            writer.push_field(field);
        }
        let mut output = Cursor::new(Vec::new());
        writer.write(&mut output, false).expect("encode EXIF");
        output.into_inner()
    }
}
