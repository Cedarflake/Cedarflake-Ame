use std::fs::{self, File};
use std::io;
use std::os::windows::fs::OpenOptionsExt;

use tempfile::tempdir;
use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

use super::tests::{NEW, OLD, staged_fixture};
use super::{ReplacementFault, claim_backup, replace_file_once, retry_sharing_conflict};
use crate::adapters::preview_cache::cache_inventory;

#[test]
fn replacement_reuses_staged_bytes_after_a_native_reader_retires() {
    for lock_staging in [false, true] {
        let directory = tempdir().expect("cache");
        let target = directory.path().join("target.jpg");
        let staged = directory.path().join("staged.tmp");
        fs::write(&target, OLD).expect("old preview");
        fs::write(&staged, NEW).expect("staged preview");
        let backup = claim_backup(&target).expect("exclusive backup");
        let locked = if lock_staging { &staged } else { &target };
        let mut reader = Some(
            File::options()
                .read(true)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .open(locked)
                .expect("native preview reader"),
        );
        let mut attempts = 0;

        retry_sharing_conflict(|| {
            attempts += 1;
            let result = replace_file_once(&staged, &target, Some(&backup));
            if attempts == 1 {
                assert_eq!(
                    result
                        .as_ref()
                        .expect_err("reader blocks replacement")
                        .raw_os_error(),
                    Some(32)
                );
                assert_eq!(fs::read(&target).expect("unchanged target"), OLD);
                assert_eq!(fs::read(&staged).expect("retained staging"), NEW);
                drop(reader.take());
            }
            result
        })
        .expect("replacement after reader retirement");

        assert_eq!(attempts, 2);
        assert_eq!(fs::read(&target).expect("installed preview"), NEW);
        assert_eq!(fs::read(&backup).expect("old preview backup"), OLD);
        assert!(!staged.exists());
    }
}

#[test]
fn persistent_native_reader_preserves_old_preview_and_exact_accounting() {
    let directory = tempdir().expect("cache");
    let (store, staged, target) = staged_fixture(directory.path(), ReplacementFault::default());
    let reader = File::options()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&target)
        .expect("native preview reader");

    let issue = store
        .commit(staged)
        .expect_err("persistent sharing conflict");

    assert_eq!(issue.code, "preview_publish_failed");
    assert_eq!(fs::read(&target).expect("retained preview"), OLD);
    assert_eq!(store.used_bytes(), OLD.len() as u64);
    assert_eq!(
        cache_inventory(directory.path()).expect("inventory").0,
        OLD.len() as u64
    );
    assert_eq!(
        fs::read_dir(directory.path())
            .expect("cache entries")
            .count(),
        1
    );
    drop(reader);
}

#[test]
fn sharing_conflict_retry_is_bounded_and_preserves_the_last_error() {
    let mut attempts = 0;
    let error = retry_sharing_conflict(|| {
        attempts += 1;
        Err(io::Error::from_raw_os_error(32))
    })
    .expect_err("persistent conflict");

    assert_eq!(attempts, 4);
    assert_eq!(error.raw_os_error(), Some(32));
}

#[test]
fn unrelated_and_partial_replacement_errors_return_without_another_attempt() {
    for code in [5, 112, 1175, 1176, 1177] {
        let mut attempts = 0;
        let error = retry_sharing_conflict(|| {
            attempts += 1;
            Err(io::Error::from_raw_os_error(code))
        })
        .expect_err("terminal installation error");

        assert_eq!(attempts, 1);
        assert_eq!(error.raw_os_error(), Some(code));
    }
}

#[test]
fn terminal_error_after_a_sharing_conflict_stops_retrying() {
    let mut attempts = 0;
    let error = retry_sharing_conflict(|| {
        attempts += 1;
        let code = if attempts == 1 { 32 } else { 1177 };
        Err(io::Error::from_raw_os_error(code))
    })
    .expect_err("partial replacement needs restoration");

    assert_eq!(attempts, 2);
    assert_eq!(error.raw_os_error(), Some(1177));
}
