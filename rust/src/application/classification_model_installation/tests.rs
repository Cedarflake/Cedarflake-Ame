use super::*;

#[test]
fn prepared_publication_preserves_prior_selection_on_cancel_stale_or_active_runtime() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("classification.sqlite3");
    let original = ClassificationInstallation {
        models: directory.path().join("old-models"),
        runtime: directory.path().join("old-runtime"),
    };
    let replacement = ClassificationInstallation {
        models: directory.path().join("new-models"),
        runtime: directory.path().join("new-runtime"),
    };
    ClassificationInstallationStore::save(&settings, &original).unwrap();
    let cancelled = AtomicBool::new(true);
    assert!(publish_installation(&settings, Some(&original), &replacement, &cancelled).is_err());
    cancelled.store(false, Ordering::Release);
    let stale = publish_installation(&settings, None, &replacement, &cancelled).unwrap_err();
    assert_eq!(stale.code, "classification_installation_changed");
    while_retired(|| {
        let busy =
            publish_installation(&settings, Some(&original), &replacement, &cancelled).unwrap_err();
        assert_eq!(busy.code, "classification_runtime_active");
        Ok(())
    })
    .unwrap();
    assert_eq!(
        ClassificationInstallationStore::load(&settings).unwrap(),
        Some(original.clone())
    );
    publish_installation(&settings, Some(&original), &replacement, &cancelled).unwrap();
    assert_eq!(
        ClassificationInstallationStore::load(&settings).unwrap(),
        Some(replacement)
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn only_one_installer_owns_download_and_prepared_lifetime() {
    let owner = InstallationOwnership::acquire().unwrap();
    assert!(InstallationOwnership::acquire().is_err());
    drop(owner);
    drop(InstallationOwnership::acquire().unwrap());
}
