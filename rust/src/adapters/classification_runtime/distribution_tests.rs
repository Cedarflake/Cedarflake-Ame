use std::collections::BTreeSet;

use super::{EncoderBackend, binaries, manifest};

#[test]
fn runtime_distributions_include_platform_binaries_and_redistribution_notices() {
    for (platform, cpu, gpu, provenance) in [
        (
            "windows-x64",
            include_str!("distribution.json"),
            include_str!("accelerator_distribution.json"),
            &["VERSION_NUMBER", "GIT_COMMIT_ID"][..],
        ),
        (
            "windows-arm64",
            include_str!("distribution.arm64.json"),
            include_str!("accelerator_distribution.arm64.json"),
            &["Microsoft.ML.OnnxRuntime.nuspec"][..],
        ),
    ] {
        let cpu_files = [
            "onnxruntime.dll",
            "onnxruntime_providers_shared.dll",
            "LICENSE",
            "Privacy.md",
            "ThirdPartyNotices.txt",
        ];
        let gpu_files = [
            "onnxruntime.dll",
            "onnxruntime_providers_shared.dll",
            "DirectML.dll",
            "LICENSE-ONNXRuntime.txt",
            "Privacy.md",
            "ThirdPartyNotices-ONNXRuntime.txt",
            "LICENSE-DirectML.txt",
            "LICENSE-CODE-DirectML.txt",
            "ThirdPartyNotices-DirectML.txt",
        ];
        for (json, directory, expected) in [
            (
                cpu,
                "data/classification-runtime",
                [cpu_files.as_slice(), provenance].concat(),
            ),
            (gpu, "data/classification-accelerator", gpu_files.to_vec()),
        ] {
            let distribution: serde_json::Value = serde_json::from_str(json).unwrap();
            assert_eq!(distribution["schema"], 1);
            assert_eq!(distribution["platform"], platform);
            assert_eq!(distribution["bundleDirectory"], directory);
            let files = distribution["files"].as_array().unwrap();
            let names: BTreeSet<_> = files.iter().map(|f| f["name"].as_str().unwrap()).collect();
            assert_eq!(files.len(), names.len());
            assert_eq!(names, expected.into_iter().collect());
        }
    }
}

#[cfg(windows)]
#[test]
fn switching_architectures_recognizes_prior_runs_and_separates_native_features() {
    let factory = super::LocalClassificationEncoderFactory::default();
    for backend in [
        EncoderBackend::Cpu,
        EncoderBackend::DirectMl { device_id: 0 },
    ] {
        let x64 = manifest::pipeline(&binaries::X64, backend);
        let arm64 = manifest::pipeline(&binaries::ARM64, backend);
        assert!(factory.recognizes_model(x64));
        assert!(factory.recognizes_model(arm64));
        assert_eq!(
            x64.image_identity().embedding_space,
            arm64.image_identity().embedding_space
        );
        assert_ne!(
            x64.image_identity().pipeline,
            arm64.image_identity().pipeline
        );
        assert_ne!(x64.text_identity().pipeline, arm64.text_identity().pipeline);
    }
}
