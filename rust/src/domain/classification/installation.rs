#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelInstallationPhase {
    Verifying,
    Downloading,
    Prepared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelInstallationProgress {
    pub phase: ModelInstallationPhase,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}
