use std::sync::atomic::{AtomicBool, Ordering};

use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};

use super::*;

#[test]
fn idle_cache_namespace_replacement_rebuilds_accounting_without_retaining_handles() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = preview_fixture("idle-cache-namespace");
    let source_bytes = fs::read(&fixture.source_path).expect("source fixture bytes");
    let initial = materialize_active_preview(
        fixture.request.clone(),
        fixture.storage.clone(),
        &mut PreviewStageTimings::default(),
    )
    .expect("initial production preview");
    let old = active_preview_store(&fixture.storage).expect("initial accounting owner");
    assert!(old.used_bytes() > 0);
    let retired = fixture._directory.path().join("idle-retired-cache");
    fs::rename(&fixture.storage.preview_root, &retired)
        .expect("an idle cached store holds no namespace handles");
    fs::create_dir(&fixture.storage.preview_root).expect("replacement cache directory");
    let orphan = fixture.storage.preview_root.join(format!(
        "{}-{}.jpg",
        crate::adapters::PREVIEW_CACHE_VERSION,
        "b".repeat(64)
    ));
    let orphan_bytes = [7_u8; 4096];
    fs::write(&orphan, orphan_bytes).expect("replacement directory accounting fixture");
    let fresh = active_preview_store(&fixture.storage).expect("replacement accounting owner");
    assert!(!Arc::ptr_eq(&old, &fresh));
    assert_ne!(old.namespace_identity(), fresh.namespace_identity());
    assert_eq!(fresh.used_bytes(), orphan_bytes.len() as u64);
    let current_request = preview_request_for(&initial, fixture.request.preview_edge, false);
    let location = materialize_active_preview(
        current_request,
        fixture.storage.clone(),
        &mut PreviewStageTimings::default(),
    )
    .expect("preview in the newly admitted namespace");
    assert!(matches!(location.preview_status, PreviewStatus::Ready));
    let artifact = std::path::Path::new(&location.preview_path);
    let actual_bytes = artifact.metadata().expect("generated artifact").len();
    let current = active_preview_store(&fixture.storage).expect("current accounting owner");
    assert_eq!(
        current.used_bytes(),
        orphan_bytes.len() as u64 + actual_bytes
    );
    assert_preview_matches_source(&fixture.source_path, artifact);
    assert_eq!(
        fs::read(&orphan).expect("unrelated cache fixture remains"),
        orphan_bytes
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("source unchanged"),
        source_bytes
    );
    fs::rename(
        &fixture.storage.preview_root,
        fixture._directory.path().join("released-new-cache"),
    )
    .expect("completed preview releases its namespace while accounting Arcs survive");
}

#[test]
fn superseded_preview_never_discards_a_source_replacing_its_staging_namespace() {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let fixture = preview_fixture("staging-namespace");
    let foreign_root = fixture._directory.path().join("other-source");
    let displaced = fixture._directory.path().join("displaced-cache");
    fs::create_dir(&foreign_root).expect("generated external source directory");
    let foreign_bytes = encode_rgb_quadrants(MediaFixtureFormat::Jpeg, 16, 16)
        .expect("generated external source image");
    let remaining_source = Arc::new(Mutex::new(None));
    let replaced = Arc::new(AtomicBool::new(false));
    let hook_remaining = Arc::clone(&remaining_source);
    let hook_replaced = Arc::clone(&replaced);
    let hook_foreign_bytes = foreign_bytes.clone();
    let hook_cache = fixture.storage.preview_root.clone();
    let hook_changed_source = fixture.source_path.clone();
    const CHANGED_SOURCE: &[u8] = b"owned source fixture changed after staging";
    install_preview_test_hook(
        &BEFORE_PREVIEW_COMMIT_HOOKS,
        &fixture.request.location_id,
        move || {
            let staging = fs::read_dir(&hook_cache)
                .expect("actual generated staging directory")
                .map(|entry| entry.expect("staged entry").path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "tmp"))
                .collect::<Vec<_>>();
            assert_eq!(
                staging.len(),
                1,
                "real preview generation must own one staged file"
            );
            let leaf = staging[0].file_name().expect("staged file name");
            let foreign_source = foreign_root.join(leaf);
            fs::write(&foreign_source, hook_foreign_bytes)
                .expect("generated source with wrong suffix");
            let remaining = match fs::rename(&hook_cache, &displaced) {
                Ok(()) => {
                    fs::rename(&foreign_root, &hook_cache)
                        .expect("replace cache using only disposable source fixtures");
                    hook_replaced.store(true, Ordering::Release);
                    hook_cache.join(leaf)
                }
                Err(error) => {
                    assert_eq!(
                        error.raw_os_error(),
                        Some(32),
                        "only a held Windows sharing guard may exclude the interleaving",
                    );
                    foreign_source
                }
            };
            *hook_remaining.lock().expect("observed source location") = Some(remaining);
            fs::write(&hook_changed_source, CHANGED_SOURCE)
                .expect("supersede owned source fixture");
        },
    );
    let result = materialize_active_preview(
        fixture.request.clone(),
        fixture.storage.clone(),
        &mut PreviewStageTimings::default(),
    );
    let source = remaining_source
        .lock()
        .expect("observed source location")
        .clone()
        .expect("the actual post-generation hook must execute");
    eprintln!(
        "staging namespace: replacement_admitted={}, source_survives={}",
        replaced.load(Ordering::Acquire),
        source.exists(),
    );
    assert_eq!(
        result
            .expect_err("changed source supersedes publication")
            .code,
        "preview_request_superseded"
    );
    assert_eq!(
        fs::read(&source).expect("staging discard cannot delete an externally moved source"),
        foreign_bytes,
    );
    assert_eq!(
        fs::read_dir(source.parent().expect("source directory"))
            .expect("source entries")
            .count(),
        1,
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("owned changed source"),
        CHANGED_SOURCE
    );
    assert_active_preview_is_pending(&fixture.storage.catalog_path, &fixture.request.location_id);
}
