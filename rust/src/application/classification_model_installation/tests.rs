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
    installation_repository()
        .save(&settings, &original)
        .unwrap();
    let cancelled = AtomicBool::new(true);
    assert!(
        publish_installation(&settings, Some(&original), &replacement, None, &cancelled).is_err()
    );
    cancelled.store(false, Ordering::Release);
    let stale = publish_installation(&settings, None, &replacement, None, &cancelled).unwrap_err();
    assert_eq!(stale.code, "classification_installation_changed");
    while_retired(|| {
        let busy = publish_installation(&settings, Some(&original), &replacement, None, &cancelled)
            .unwrap_err();
        assert_eq!(busy.code, "classification_runtime_active");
        Ok(())
    })
    .unwrap();
    assert_eq!(
        installation_repository().load(&settings).unwrap(),
        Some(original.clone())
    );
    publish_installation(&settings, Some(&original), &replacement, None, &cancelled).unwrap();
    assert_eq!(
        installation_repository().load(&settings).unwrap(),
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

#[test]
fn prepared_resources_outlive_publication_and_retire_after_every_terminal_outcome() {
    use std::sync::{Arc, Mutex};

    struct PreparedFixture {
        settings: PathBuf,
        installation: ClassificationInstallation,
        selection_at_release: Arc<Mutex<Option<ClassificationInstallation>>>,
    }

    impl PreparedClassificationModel for PreparedFixture {
        fn installation(&self) -> ClassificationInstallation {
            self.installation.clone()
        }
    }

    impl Drop for PreparedFixture {
        fn drop(&mut self) {
            assert!(InstallationOwnership::acquire().is_err());
            *self.selection_at_release.lock().unwrap() =
                installation_repository().load(&self.settings).unwrap();
        }
    }

    for outcome in ["commit", "cancel", "stale", "active-runtime", "dispose"] {
        let directory = tempfile::tempdir().unwrap();
        let settings = directory.path().join("classification.sqlite3");
        let original = ClassificationInstallation {
            models: directory.path().join("original-models"),
            runtime: directory.path().join("original-runtime"),
        };
        let replacement = ClassificationInstallation {
            models: directory.path().join("replacement-models"),
            runtime: directory.path().join("replacement-runtime"),
        };
        installation_repository()
            .save(&settings, &original)
            .unwrap();
        let selection_at_release = Arc::new(Mutex::new(None));
        let prepared = PreparedModelInstallation {
            settings: settings.clone(),
            expected: (outcome != "stale").then_some(original.clone()),
            verified: Box::new(PreparedFixture {
                settings: settings.clone(),
                installation: replacement.clone(),
                selection_at_release: selection_at_release.clone(),
            }),
            pending: None,
            _ownership: InstallationOwnership::acquire().unwrap(),
        };
        let cancelled = AtomicBool::new(outcome == "cancel");
        match outcome {
            "commit" => prepared.commit(&cancelled).unwrap(),
            "cancel" => assert_eq!(
                prepared.commit(&cancelled).unwrap_err().code,
                "classification_cancelled",
            ),
            "stale" => assert_eq!(
                prepared.commit(&cancelled).unwrap_err().code,
                "classification_installation_changed",
            ),
            "active-runtime" => while_retired(|| {
                assert_eq!(
                    prepared.commit(&cancelled).unwrap_err().code,
                    "classification_runtime_active",
                );
                Ok(())
            })
            .unwrap(),
            "dispose" => drop(prepared),
            _ => unreachable!(),
        }
        let expected = if outcome == "commit" {
            replacement
        } else {
            original
        };
        assert_eq!(*selection_at_release.lock().unwrap(), Some(expected));
        drop(InstallationOwnership::acquire().unwrap());
    }
}
