use crate::domain::analysis_capacity::AnalysisCapacity;

pub(crate) trait AnalysisCapacityProvider: Send + Sync {
    fn capacity(&self) -> Option<AnalysisCapacity>;
    fn memory_pressure(&self) -> bool;
}
