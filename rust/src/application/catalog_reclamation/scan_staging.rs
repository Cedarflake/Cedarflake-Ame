use crate::domain::{CatalogReclamationPhase, ScanError};
use crate::ports::CatalogSpaceRepository;

use super::{CatalogReclamationOperation, retry_controlled_maintenance};

pub(super) fn discard_retired_staging(
    maintenance: &impl CatalogSpaceRepository,
    operation: &CatalogReclamationOperation,
) -> Result<Option<()>, ScanError> {
    loop {
        let Some(discarded) = retry_controlled_maintenance(
            operation,
            CatalogReclamationPhase::Reclaiming,
            |control| maintenance.try_discard_retired_scan_staging(control),
        )?
        else {
            return Ok(None);
        };
        if discarded == 0 {
            return Ok(Some(()));
        }
        std::thread::yield_now();
    }
}
