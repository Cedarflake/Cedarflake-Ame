use std::io::Cursor;

use exif::{Field, In, Tag, Value, experimental::Writer};

use crate::adapters::exif_metadata::KamadakExifExtractor;
use crate::domain::{CameraMetadata, MetadataInspection};
use crate::ports::MetadataExtractor;

fn inspect(make: Value, model: Value) -> MetadataInspection {
    let fields = [
        Field {
            tag: Tag::Make,
            ifd_num: In::PRIMARY,
            value: make,
        },
        Field {
            tag: Tag::Model,
            ifd_num: In::PRIMARY,
            value: model,
        },
        Field {
            tag: Tag::DateTimeOriginal,
            ifd_num: In::PRIMARY,
            value: text(b"2026:10:30 12:34:56"),
        },
    ];
    let mut writer = Writer::new();
    for field in &fields {
        writer.push_field(field);
    }
    let mut bytes = Cursor::new(Vec::new());
    writer.write(&mut bytes, false).unwrap();
    KamadakExifExtractor.extract(Some(bytes.get_ref()), "C:/Fixture/camera.jpg")
}

fn text(value: &[u8]) -> Value {
    Value::Ascii(vec![value.to_vec()])
}

#[test]
fn camera_metadata_preserves_unicode_and_trims_padding_without_changing_capture_time() {
    let result = inspect(text(" 制造商 ".as_bytes()), text(b" Camera 2 "));
    assert_eq!(
        result.camera,
        CameraMetadata {
            make: Some("制造商".into()),
            model: Some("Camera 2".into())
        }
    );
    assert_eq!(
        result.capture_time.unwrap().local_time,
        "2026-10-30T12:34:56.000000000"
    );
    assert!(result.issues.is_empty());
}

#[test]
fn camera_metadata_rejects_bad_fields_independently_of_other_evidence() {
    for invalid in [
        text(&[0xff]),
        text(b"Camera\nInjected"),
        text(&[b'x'; 129]),
        Value::Ascii(vec![b"one".to_vec(), b"two".to_vec()]),
        Value::Short(vec![42]),
    ] {
        for invalid_make in [true, false] {
            let (make, model) = if invalid_make {
                (invalid.clone(), text(b"Valid"))
            } else {
                (text(b"Valid"), invalid.clone())
            };
            let result = inspect(make, model);
            assert_eq!(result.camera.make.is_none(), invalid_make);
            assert_eq!(result.camera.model.is_none(), !invalid_make);
            assert!(result.capture_time.is_some());
            assert_eq!(result.issues.len(), 1);
            assert_eq!(result.issues[0].code, "camera_metadata_invalid");
        }
    }
}

#[test]
fn camera_metadata_accepts_limit_and_leaves_absent_or_blank_fields_unknown() {
    let result = inspect(text(&[b'x'; 128]), text(b"   "));
    assert_eq!(result.camera.make.as_ref().unwrap().len(), 128);
    assert!(result.camera.model.is_none());
    assert!(result.issues.is_empty());
    let absent = KamadakExifExtractor.extract(None, "C:/Fixture/plain.png");
    assert_eq!(absent.camera, CameraMetadata::default());
    assert!(absent.issues.is_empty());
}
