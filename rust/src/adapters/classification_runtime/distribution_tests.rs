use super::manifest::{PROVIDER, RUNTIME};

#[test]
fn bundled_runtime_matches_the_worker_artifact_contract() {
    let distribution: serde_json::Value =
        serde_json::from_str(include_str!("distribution.json")).unwrap();
    assert_eq!(distribution["schema"], 1);
    assert_eq!(distribution["platform"], "windows-x64");
    assert_eq!(
        distribution["bundleDirectory"],
        "data/classification-runtime"
    );
    let files = distribution["files"].as_array().unwrap();
    assert_eq!(files.len(), 7);
    for artifact in [RUNTIME, PROVIDER] {
        let matching: Vec<_> = files
            .iter()
            .filter(|file| file["name"] == artifact.name)
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0]["length"], artifact.length);
        assert_eq!(matching[0]["sha256"], artifact.digest);
    }
    for notice in [
        "LICENSE",
        "Privacy.md",
        "ThirdPartyNotices.txt",
        "VERSION_NUMBER",
        "GIT_COMMIT_ID",
    ] {
        assert_eq!(
            files.iter().filter(|file| file["name"] == notice).count(),
            1
        );
    }
}

#[test]
fn accelerator_bundle_contains_pinned_binaries_and_distribution_notices() {
    use super::backend::{DIRECTML, GPU_PROVIDER, GPU_RUNTIME};
    let distribution: serde_json::Value =
        serde_json::from_str(include_str!("accelerator_distribution.json")).unwrap();
    let files = distribution["files"].as_array().unwrap();
    assert_eq!(files.len(), 9);
    for artifact in [GPU_RUNTIME, GPU_PROVIDER, DIRECTML] {
        let file = files
            .iter()
            .find(|file| file["name"] == artifact.name)
            .unwrap();
        assert_eq!(file["length"], artifact.length);
        assert_eq!(file["sha256"], artifact.digest);
    }
    for name in [
        "LICENSE-ONNXRuntime.txt",
        "Privacy.md",
        "ThirdPartyNotices-ONNXRuntime.txt",
        "LICENSE-DirectML.txt",
        "LICENSE-CODE-DirectML.txt",
        "ThirdPartyNotices-DirectML.txt",
    ] {
        assert_eq!(files.iter().filter(|file| file["name"] == name).count(), 1);
    }
}
