use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EncoderBackend {
    #[default]
    Cpu,
    DirectMl {
        device_id: i32,
    },
}

pub(super) const GPU_RUNTIME: super::manifest::Artifact = super::manifest::Artifact {
    name: "onnxruntime.dll",
    length: 17_328_152,
    digest: "e7eedec6a6f26dc39dc948276a75ef6d2bee3fff944d874ceed0bbd3b97bff40",
};
#[cfg(any(windows, test))]
pub(super) const GPU_PROVIDER: super::manifest::Artifact = super::manifest::Artifact {
    name: "onnxruntime_providers_shared.dll",
    length: 22_040,
    digest: "265c8daf29637cb259cac8be9f08f2cd45f3883f0f0e4949cbfddd5b4cbec3b6",
};
pub(super) const DIRECTML: super::manifest::Artifact = super::manifest::Artifact {
    name: "DirectML.dll",
    length: 18_527_776,
    digest: "9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1",
};
