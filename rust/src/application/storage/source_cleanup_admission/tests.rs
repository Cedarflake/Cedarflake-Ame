use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};

use tempfile::tempdir;

use super::*;

#[test]
fn cleanup_excludes_source_ancestors_descendants_and_resolved_aliases() {
    let directory = tempdir().expect("isolated root");
    let cache = directory.path().join("cache");
    fs::create_dir(&cache).expect("cache");
    let _cleanup = reserve_preview_cleanup(&cache).expect("cleanup admission");
    for source in [
        cache.clone(),
        directory.path().to_path_buf(),
        cache.join("future-source"),
        fs::canonicalize(&cache).expect("physical spelling"),
        cache
            .join(".")
            .join("child")
            .join("..")
            .join("future-source"),
    ] {
        let error = reserve_source_registration(&source)
            .err()
            .expect("overlap rejected");
        assert_eq!(error.code, "source_root_cleanup_active");
    }
    let _neighbor = reserve_source_registration(&directory.path().join("cache-neighbor"))
        .expect("component boundary does not exclude neighbor");
}

#[test]
fn pending_registration_excludes_cleanup_but_not_other_registrations() {
    let directory = tempdir().expect("isolated root");
    let source = directory.path().join("source");
    fs::create_dir(&source).expect("source");
    let first = reserve_source_registration(&source).expect("first registration");
    let second = reserve_source_registration(&source).expect("second registration");
    drop(first);
    for cache in [
        source.clone(),
        source.join("previews"),
        directory.path().to_path_buf(),
    ] {
        let error = reserve_preview_cleanup(&cache)
            .err()
            .expect("pending source wins");
        assert_eq!(error.code, "preview_cleanup_source_registration_active");
    }
    let _other =
        reserve_preview_cleanup(&directory.path().join("other-cache")).expect("unrelated cleanup");
    drop(second);
    let _released = reserve_preview_cleanup(&source).expect("last source permit releases scope");
}

#[test]
fn scope_unwinds_without_poisoning_or_retaining_admission() {
    let directory = tempdir().expect("isolated root");
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _cleanup = reserve_preview_cleanup(directory.path()).expect("cleanup");
        panic!("operation failed outside registry");
    }));
    assert!(outcome.is_err());
    let _registration = reserve_source_registration(directory.path()).expect("released on unwind");
}
