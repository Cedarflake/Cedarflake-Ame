use std::io::{Cursor, Error};

use super::*;

#[test]
fn full_content_differs_when_only_the_last_chunk_changes() {
    let mut bytes = (0..700_003)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    let cancelled = AtomicBool::new(false);
    let first = hash_reader(Cursor::new(&bytes), bytes.len() as u64, &cancelled)
        .unwrap()
        .unwrap();
    assert_eq!(first.byte_length, 700_003);
    assert_eq!(
        first,
        hash_reader(Cursor::new(&bytes), bytes.len() as u64, &cancelled)
            .unwrap()
            .unwrap()
    );
    *bytes.last_mut().unwrap() ^= 1;
    let changed = hash_reader(Cursor::new(&bytes), bytes.len() as u64, &cancelled)
        .unwrap()
        .unwrap();
    assert_ne!(first.digest, changed.digest);
}

#[test]
fn empty_input_matches_the_published_blake3_vector() {
    let fingerprint = hash_reader(Cursor::new([]), 0, &AtomicBool::new(false))
        .unwrap()
        .unwrap();
    assert_eq!(
        fingerprint.digest,
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
}

#[test]
fn shorter_and_longer_sources_never_publish_partial_evidence() {
    for expected in [3, 5] {
        let error =
            hash_reader(Cursor::new([1, 2, 3, 4]), expected, &AtomicBool::new(false)).unwrap_err();
        assert_eq!(error.code, "exact_source_changed");
    }
}

struct CancellingReader<'a> {
    cancellation: &'a AtomicBool,
    reads: usize,
}

impl Read for CancellingReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        self.reads += 1;
        assert_eq!(
            self.reads, 1,
            "cancellation must stop before another content read"
        );
        self.cancellation.store(true, Ordering::Release);
        buffer[0] = 1;
        Ok(1)
    }
}

#[test]
fn cancellation_drops_partial_content_between_chunks() {
    let cancelled = AtomicBool::new(false);
    let reader = CancellingReader {
        cancellation: &cancelled,
        reads: 0,
    };
    assert!(hash_reader(reader, 600_000, &cancelled).unwrap().is_none());
}

struct FailingReader;
impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
        Err(Error::other("fixture read failure"))
    }
}

#[test]
fn read_failures_produce_no_digest() {
    assert_eq!(
        hash_reader(FailingReader, 4, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "exact_source_read_failed"
    );
}
