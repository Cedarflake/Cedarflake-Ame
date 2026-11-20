use std::sync::LazyLock;

use serde::Deserialize;

use super::manifest::Artifact;

pub(super) struct RuntimeBinaries {
    pub cpu: Artifact,
    pub cpu_provider: Artifact,
    pub gpu: Artifact,
    pub gpu_provider: Artifact,
    pub directml: Artifact,
}

pub(super) static X64: LazyLock<RuntimeBinaries> = LazyLock::new(|| {
    RuntimeBinaries::from_manifests(
        include_str!("distribution.json"),
        include_str!("accelerator_distribution.json"),
    )
});
pub(super) static ARM64: LazyLock<RuntimeBinaries> = LazyLock::new(|| {
    RuntimeBinaries::from_manifests(
        include_str!("distribution.arm64.json"),
        include_str!("accelerator_distribution.arm64.json"),
    )
});

pub(super) fn native() -> &'static RuntimeBinaries {
    if cfg!(target_arch = "aarch64") {
        &ARM64
    } else {
        &X64
    }
}

#[derive(Deserialize)]
struct Distribution<'a> {
    #[serde(borrow)]
    files: Vec<FilePin<'a>>,
}

#[derive(Deserialize)]
struct FilePin<'a> {
    name: &'a str,
    length: u64,
    sha256: &'a str,
}

impl Distribution<'static> {
    fn binary(&self, name: &'static str) -> Artifact {
        let file = self
            .files
            .iter()
            .find(|file| file.name == name)
            .expect("the compiled distribution includes the required binary");
        Artifact {
            name,
            length: file.length,
            digest: file.sha256,
        }
    }
}

impl RuntimeBinaries {
    fn from_manifests(cpu: &'static str, gpu: &'static str) -> Self {
        let cpu: Distribution = serde_json::from_str(cpu).expect("valid bundled CPU distribution");
        let gpu: Distribution =
            serde_json::from_str(gpu).expect("valid bundled DirectML distribution");
        Self {
            cpu: cpu.binary("onnxruntime.dll"),
            cpu_provider: cpu.binary("onnxruntime_providers_shared.dll"),
            gpu: gpu.binary("onnxruntime.dll"),
            gpu_provider: gpu.binary("onnxruntime_providers_shared.dll"),
            directml: gpu.binary("DirectML.dll"),
        }
    }
}
