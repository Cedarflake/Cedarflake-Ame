use crate::application::classification_dispatch::{
    ClassificationDispatchSignals, ClassificationForegroundState, MonotonicDispatchClock,
};
use crate::application::classification_runtime::{
    ClassificationRuntimePaths, NativeClassificationRuntime, PreparedCategoryMaintenance,
    commit_category_maintenance,
};

use super::*;

#[test]
fn runtime_retirement_precedes_maintenance_and_reopening_uses_the_committed_setup() {
    let (directory, mut catalog) = super::super::super::fixture();
    sparse(&mut catalog);
    let signals = ClassificationDispatchSignals::new(MonotonicDispatchClock::default());
    signals
        .foreground_changed(ClassificationForegroundState {
            suspended: true,
            ..Default::default()
        })
        .unwrap();
    let paths = || ClassificationRuntimePaths {
        database: directory.path().join("catalog.sqlite3"),
        feature_cache_directory: directory.path().join("features"),
        worker: directory.path().join("absent-worker.exe"),
        models: directory.path().join("absent-models"),
        runtime: directory.path().join("absent-runtime"),
    };
    let mut runtime =
        NativeClassificationRuntime::open(paths(), id("before-maintenance"), signals).unwrap();
    let pending = crate::application::classification_discovery::prepare_category_reduction(
        id("merge"),
        &catalog,
        None,
        &Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
    .unwrap();
    let before = catalog.load_classification_configuration().unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    assert_eq!(
        commit_category_maintenance(
            &paths().database,
            PreparedCategoryMaintenance::from(&pending),
            &cancelled
        )
        .unwrap_err()
        .code,
        "classification_runtime_active"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    runtime.control().signals().close().unwrap();
    runtime.run().unwrap();
    drop(runtime);
    let updated = commit_category_maintenance(
        &paths().database,
        PreparedCategoryMaintenance::from(&pending),
        &cancelled,
    )
    .unwrap();
    assert_eq!(updated.dispatch.run_id, Some(id("merge")));
    assert_eq!(updated.dispatch.mode, ClassificationDispatchMode::Automatic);
    catalog
        .change_classification_dispatch(
            updated.dispatch.revision,
            &ClassificationDispatchCommand::Pause,
        )
        .unwrap();
    let paused = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        commit_category_maintenance(
            &paths().database,
            PreparedCategoryMaintenance::from(&pending),
            &cancelled
        )
        .unwrap(),
        paused
    );
    let signals = ClassificationDispatchSignals::new(MonotonicDispatchClock::default());
    let mut reopened =
        NativeClassificationRuntime::open(paths(), id("after-maintenance"), signals).unwrap();
    assert!(matches!(
        reopened.step().unwrap(),
        crate::application::classification_dispatch::ClassificationDispatchStep::Waiting { .. }
    ));
    let current = catalog.load_classification_configuration().unwrap();
    assert_eq!(current.dispatch.run_id, Some(id("merge")));
    assert_eq!(current.dispatch.mode, ClassificationDispatchMode::Paused);
    assert_eq!(current.dispatch.session_id, Some(id("after-maintenance")));
    assert!(!paths().feature_cache_directory.exists());
    reopened.control().signals().close().unwrap();
    reopened.run().unwrap();
    drop(reopened);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_maintenance_evidence"
        ),
        2
    );
}

#[test]
fn cancelled_and_stale_maintenance_leave_the_prior_setup_available() {
    let (directory, mut catalog) = super::super::super::fixture();
    sparse(&mut catalog);
    let pending = proposal(&catalog);
    let database = directory.path().join("catalog.sqlite3");
    let cancelled = Arc::new(AtomicBool::new(true));
    let before = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        commit_category_maintenance(
            &database,
            PreparedCategoryMaintenance::Merge(&pending),
            &cancelled
        )
        .unwrap_err()
        .code,
        "classification_cancelled"
    );
    assert_eq!(catalog.load_classification_configuration().unwrap(), before);
    cancelled.store(false, Ordering::Release);
    catalog
        .change_classification_dispatch(
            before.dispatch.revision,
            &ClassificationDispatchCommand::Stop,
        )
        .unwrap();
    let stopped = catalog.load_classification_configuration().unwrap();
    assert_eq!(
        commit_category_maintenance(
            &database,
            PreparedCategoryMaintenance::Merge(&pending),
            &cancelled
        )
        .unwrap_err()
        .code,
        "classification_changed"
    );
    assert_eq!(
        catalog.load_classification_configuration().unwrap(),
        stopped
    );
    assert!(
        catalog
            .load_classification_setup(&id("merge"))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_maintenance_evidence"
        ),
        1
    );
}
