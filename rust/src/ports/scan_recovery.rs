use crate::domain::{RecoverableScan, ScanError};

/// Converges obsolete foreground scans before returning the retained import.
/// Call only while the catalog's foreground scan exclusion is held.
pub(crate) trait ForegroundScanRecovery {
    fn load_single_recoverable_foreground_scan(
        &mut self,
    ) -> Result<Option<RecoverableScan>, ScanError>;
    fn load_single_paused_foreground_scan(&mut self) -> Result<Option<RecoverableScan>, ScanError>;
}
