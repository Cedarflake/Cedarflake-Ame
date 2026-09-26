use std::io::{self, Cursor, Read};
use std::path::Path;

use super::{has_image_extension, has_supported_magic_from_reader};

#[test]
fn admitted_signatures_include_icons_without_trusting_the_extension() {
    for signature in [
        &b"\x89PNG\r\n\x1a\n"[..],
        &b"\xff\xd8\xff"[..],
        &b"GIF87a"[..],
        &b"GIF89a"[..],
        &b"BM"[..],
        &b"II*\0"[..],
        &b"MM\0*"[..],
        &b"\0\0\x01\0"[..],
        &b"RIFF\x0c\0\0\0WEBP"[..],
    ] {
        assert!(has_supported_magic_from_reader(&mut Cursor::new(signature)).expect("signature"));
    }
    assert!(!has_image_extension(Path::new("valid.data")));
    for extension in [
        "bmp", "gif", "ico", "jpeg", "JPG", "PNG", "tif", "tiff", "webp",
    ] {
        assert!(has_image_extension(Path::new(&format!(
            "image.{extension}"
        ))));
    }
}

#[test]
fn nonmedia_and_unadmitted_formats_do_not_become_candidates() {
    for signature in [
        &b""[..],
        &b"\0\0\0\0"[..],
        &b"\0\0\0\x18ftypmp42"[..],
        &b"\0\0\0\x18ftypavif"[..],
        &b"DDS "[..],
        &b"RIFF\x0c\0\0\0WAVE"[..],
        &b"not a picture"[..],
    ] {
        assert!(!has_supported_magic_from_reader(&mut Cursor::new(signature)).expect("signature"));
    }
}

#[test]
fn signature_reads_are_bounded_and_handle_short_reads_and_interruption() {
    let bytes = b"\x89PNG\r\n\x1a\n0123456789abcdef";
    let mut reader = ShortReader {
        bytes: Cursor::new(bytes),
        interrupt: true,
    };
    assert!(has_supported_magic_from_reader(&mut reader).expect("short reads"));
    assert_eq!(reader.bytes.position(), 16);
}

#[test]
fn signature_read_failures_remain_io_failures() {
    let mut reader = FailingReader;
    assert_eq!(
        has_supported_magic_from_reader(&mut reader)
            .expect_err("read failure")
            .kind(),
        io::ErrorKind::PermissionDenied,
    );
}

struct ShortReader<'a> {
    bytes: Cursor<&'a [u8]>,
    interrupt: bool,
}

impl Read for ShortReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if std::mem::take(&mut self.interrupt) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let limit = output.len().min(1);
        self.bytes.read(&mut output[..limit])
    }
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _output: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
