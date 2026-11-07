#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassificationComputeDevice {
    Cpu,
    Gpu { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationDeviceStatus {
    pub device: ClassificationComputeDevice,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationAccelerationSettings {
    pub gpu_enabled: bool,
    pub status: Option<ClassificationDeviceStatus>,
}
