use std::cell::RefCell;

use super::*;

type Hook = Option<(String, Box<dyn FnOnce()>)>;
thread_local! {
    static BEFORE_RECLAMATION: RefCell<Hook> = RefCell::new(None);
    static AFTER_RECLAMATION: RefCell<Hook> = RefCell::new(None);
}

pub(in crate::application::preview) fn before_reclamation(id: &str) {
    run(&BEFORE_RECLAMATION, id);
}

pub(in crate::application::preview) fn after_reclamation(id: &str) {
    run(&AFTER_RECLAMATION, id);
}

fn run(hooks: &'static std::thread::LocalKey<RefCell<Hook>>, id: &str) {
    let hook = hooks.with(|hooks| {
        let mut slot = hooks.borrow_mut();
        if slot.as_ref().is_some_and(|(expected, _)| expected == id) {
            slot.take().map(|(_, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook();
    }
}

#[test]
fn recovery_before_capacity_reclamation_uses_current_budget_owner() {
    exercise_recovery_gap(true);
}

#[test]
fn recovery_after_capacity_reclamation_accounts_the_retried_artifact() {
    exercise_recovery_gap(false);
}

#[test]
fn cache_namespace_remains_pinned_before_capacity_reclamation() {
    exercise_namespace_gap(true);
}

#[test]
fn cache_namespace_remains_pinned_after_capacity_reclamation() {
    exercise_namespace_gap(false);
}

fn exercise_namespace_gap(before: bool) {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let mut fixture = preview_fixture(if before {
        "namespace-before-reclaim"
    } else {
        "namespace-after-reclaim"
    });
    fixture.storage.preview_budget_bytes = 8192;
    fs::create_dir(&fixture.storage.preview_root).expect("cache directory");
    let orphan = fixture.storage.preview_root.join(format!(
        "{}-{}.jpg",
        crate::adapters::PREVIEW_CACHE_VERSION,
        "c".repeat(64)
    ));
    fs::write(&orphan, [0_u8; 8192]).expect("capacity fixture");
    let source_bytes = fs::read(&fixture.source_path).expect("source fixture");
    let hook_storage = fixture.storage.clone();
    let displaced = fixture._directory.path().join("displaced-in-gap");
    let hook_displaced = displaced.clone();
    let invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hook_invoked = Arc::clone(&invoked);
    let hook: Box<dyn FnOnce()> = Box::new(move || {
        let error = fs::rename(&hook_storage.preview_root, &hook_displaced)
            .expect_err("namespace identity survives the runtime-access gap");
        assert_eq!(error.raw_os_error(), Some(32));
        crate::application::preview_recovery::run_preview_recovery_for_test(&hook_storage)
            .expect("independent recovery remains admitted inside the gap");
        hook_invoked.store(true, std::sync::atomic::Ordering::Release);
    });
    let slot = if before {
        &BEFORE_RECLAMATION
    } else {
        &AFTER_RECLAMATION
    };
    slot.with(|slot| *slot.borrow_mut() = Some((fixture.request.location_id.clone(), hook)));
    let result = materialize_active_preview(
        fixture.request.clone(),
        fixture.storage.clone(),
        &mut PreviewStageTimings::default(),
    );
    BEFORE_RECLAMATION.with(|slot| slot.borrow_mut().take());
    AFTER_RECLAMATION.with(|slot| slot.borrow_mut().take());
    assert!(
        invoked.load(std::sync::atomic::Ordering::Acquire),
        "actual capacity gap was reached"
    );
    let location = result.expect("capacity retry completes under its original namespace");
    assert!(matches!(location.preview_status, PreviewStatus::Ready));
    assert_preview_matches_source(
        &fixture.source_path,
        std::path::Path::new(&location.preview_path),
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("source remains intact"),
        source_bytes
    );
    fs::rename(&fixture.storage.preview_root, displaced)
        .expect("request completion releases namespace guards");
}

fn exercise_recovery_gap(before: bool) {
    let _serial = crate::application::PREVIEW_LIFECYCLE_TEST_LOCK
        .lock()
        .expect("preview lifecycle lock");
    let mut fixture = preview_fixture(if before {
        "before-reclaim"
    } else {
        "after-reclaim"
    });
    fixture.storage.preview_budget_bytes = 8192;
    fs::create_dir(&fixture.storage.preview_root).expect("preview root");
    let orphan = fixture.storage.preview_root.join(format!(
        "{}-{}.jpg",
        crate::adapters::PREVIEW_CACHE_VERSION,
        "a".repeat(64),
    ));
    fs::write(&orphan, vec![0; 8192]).expect("bounded unreferenced cache fixture");
    let source_bytes = fs::read(&fixture.source_path).expect("source bytes");
    invalidate_active_preview_store().expect("reset isolated active owner");
    let old = active_preview_store(&fixture.storage).expect("initial actual owner");
    assert_eq!(old.used_bytes(), 8192);
    let hook_storage = fixture.storage.clone();
    let old_for_hook = Arc::clone(&old);
    let invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hook_invoked = Arc::clone(&invoked);
    let hook: Box<dyn FnOnce()> = Box::new(move || {
        if !before {
            assert!(
                !orphan.exists(),
                "production reclamation removed the original orphan"
            );
            assert_eq!(old_for_hook.used_bytes(), 0);
            fs::write(&orphan, [0]).expect("interrupted derived artifact for real recovery");
        }
        crate::application::preview_recovery::run_preview_recovery_for_test(&hook_storage)
            .expect("real recovery and invalidation in the access gap");
        assert!(!orphan.exists());
        let current = active_preview_store(&hook_storage).expect("replacement actual owner");
        assert!(!Arc::ptr_eq(&old_for_hook, &current));
        assert_eq!(current.used_bytes(), 0);
        hook_invoked.store(true, std::sync::atomic::Ordering::Release);
    });
    let slot = if before {
        &BEFORE_RECLAMATION
    } else {
        &AFTER_RECLAMATION
    };
    slot.with(|slot| *slot.borrow_mut() = Some((fixture.request.location_id.clone(), hook)));
    let result = materialize_active_preview(
        fixture.request.clone(),
        fixture.storage.clone(),
        &mut PreviewStageTimings::default(),
    );
    let _ = BEFORE_RECLAMATION.with(|slot| slot.borrow_mut().take());
    let _ = AFTER_RECLAMATION.with(|slot| slot.borrow_mut().take());
    assert!(
        invoked.load(std::sync::atomic::Ordering::Acquire),
        "the production capacity branch ran"
    );
    let location = result.expect("active preview request");
    assert!(
        matches!(location.preview_status, PreviewStatus::Ready),
        "a replaced budget owner cannot reject available capacity: {:?}",
        location.preview_status
    );
    let current = active_preview_store(&fixture.storage).expect("current owner after retry");
    assert!(!Arc::ptr_eq(&old, &current));
    let artifact = only_current_preview_artifact(&fixture.storage.preview_root);
    let actual_bytes = artifact.metadata().expect("installed artifact").len();
    assert!(actual_bytes > 0 && actual_bytes <= fixture.storage.preview_budget_bytes);
    assert_eq!(
        current.used_bytes(),
        actual_bytes,
        "the active owner accounts for the retry installation"
    );
    assert_preview_matches_source(&fixture.source_path, &artifact);
    assert_unique_ready_preview_owner(
        &fixture.storage.catalog_path,
        &location.location_id,
        &location.preview_path,
    );
    assert_eq!(
        fs::read(&fixture.source_path).expect("unchanged source"),
        source_bytes
    );
}
