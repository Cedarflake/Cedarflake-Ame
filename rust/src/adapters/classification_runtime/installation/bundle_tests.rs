use super::*;
use sha2::{Digest, Sha256};

#[test]
fn distribution_pins_match_runtime_files_and_compiled_notices() {
    let distribution: serde_json::Value =
        serde_json::from_str(include_str!("../model_distribution.json")).unwrap();
    assert_eq!(distribution["package"], PACKAGE_DIRECTORY);
    assert_eq!(
        distribution["bundleDirectory"],
        format!("data/classification-models/{PACKAGE_DIRECTORY}")
    );
    let files = distribution["files"].as_array().unwrap();
    assert_eq!(files.len(), MODEL_FILES.len());
    for artifact in MODEL_FILES {
        let file = files
            .iter()
            .find(|file| file["name"] == artifact.name)
            .unwrap();
        assert_eq!(file["length"], artifact.length);
        assert_eq!(file["sha256"], artifact.digest);
    }
    let notices = distribution["notices"].as_array().unwrap();
    assert_eq!(notices.len(), NOTICES.len());
    for (name, bytes) in NOTICES {
        let file = notices.iter().find(|file| file["name"] == name).unwrap();
        assert_eq!(file["length"], bytes.len() as u64);
        assert_eq!(file["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
}

#[test]
fn bundled_preparation_only_reads_and_pins_verified_files() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("bundled");
    std::fs::create_dir(&models).unwrap();
    let artifact = Artifact {
        name: "fixture.onnx",
        length: 3,
        digest: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    };
    std::fs::write(models.join(artifact.name), b"abc").unwrap();
    for (name, bytes) in NOTICES {
        std::fs::write(models.join(name), bytes).unwrap();
    }
    let mut updates = Vec::new();
    let installation = prepare_bundled_with(
        &models,
        directory.path(),
        &[&artifact],
        &[],
        &AtomicBool::new(false),
        &mut |progress| updates.push(progress),
    )
    .unwrap();
    assert_eq!(updates.last().unwrap().completed_bytes, 3);
    assert!(
        updates
            .iter()
            .all(|progress| progress.phase == ModelInstallationPhase::Verifying)
    );
    assert_eq!(std::fs::read_dir(&models).unwrap().count(), 3);
    assert!(std::fs::write(models.join(artifact.name), b"bad").is_err());
    drop(installation);
    std::fs::remove_file(models.join("NOTICE.txt")).unwrap();
    assert!(
        prepare_bundled_with(
            &models,
            directory.path(),
            &[&artifact],
            &[],
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    assert!(!models.join("NOTICE.txt").exists());
}
