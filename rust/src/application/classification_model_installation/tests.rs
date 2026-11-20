use super::*;

#[test]
fn relocated_msi_uses_its_bundled_models_and_portable_keeps_the_saved_directory() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("settings.sqlite3");
    let old_data = directory.path().join("old-x64/data");
    let new_data = directory.path().join("new-arm64/data");
    let package = Path::new("classification-models").join(model_package().package_directory());
    let saved = ClassificationInstallation {
        models: old_data.join(&package),
        runtime: old_data.join("classification-runtime"),
    };
    std::fs::create_dir_all(&saved.models).unwrap();
    installation_repository().save(&settings, &saved).unwrap();

    let resolve = || {
        production::installation_for_execution(&settings, &new_data)
            .unwrap()
            .unwrap()
    };
    assert_eq!(resolve().models, saved.models);
    std::fs::remove_dir(&saved.models).unwrap();
    let installed_models = new_data.join(package);
    std::fs::create_dir_all(&installed_models).unwrap();
    let installed = resolve();
    assert_eq!(installed.models, installed_models);
    assert_eq!(installed.runtime, new_data.join("classification-runtime"));
    assert_eq!(
        installation_repository().load(&settings).unwrap(),
        Some(saved)
    );
}

#[test]
fn moving_a_portable_build_uses_its_own_runtime_and_keeps_the_saved_model_selection() {
    let directory = tempfile::tempdir().unwrap();
    let settings = directory.path().join("settings.sqlite3");
    let application_data = directory.path().join("arm64/data");
    let bundled = directory.path().join("x64/data/classification-runtime");
    let custom = directory.path().join("custom/classification-runtime");
    std::fs::create_dir_all(
        application_data
            .join("classification-models")
            .join(model_package().package_directory()),
    )
    .unwrap();
    for (saved, effective) in [
        (bundled, application_data.join("classification-runtime")),
        (custom.clone(), custom),
    ] {
        let selected = ClassificationInstallation {
            models: directory.path().join("models"),
            runtime: saved,
        };
        installation_repository()
            .save(&settings, &selected)
            .unwrap();
        let loaded = production::installation_for_execution(&settings, &application_data)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.models, selected.models);
        assert_eq!(loaded.runtime, effective);
        assert_eq!(
            installation_repository().load(&settings).unwrap(),
            Some(selected)
        );
    }
}

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
