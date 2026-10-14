use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

use crate::adapters::SqliteCatalogSpaceMaintenance;
use crate::ports::{CatalogMaintenanceAttempt, CatalogMaintenanceControl, CatalogSpaceRepository};

pub(super) fn finish_retired_staging(catalog_path: &Path, maximum_rows: u64) {
    let maintenance = SqliteCatalogSpaceMaintenance::new(catalog_path.to_path_buf());
    let control = CatalogMaintenanceControl::new(Arc::new(AtomicBool::new(false)));
    let mut discarded_rows = 0;
    // The standalone scan fixture has no application maintenance worker. Drive its same
    // bounded cleanup after measuring foreground cancellation, then assert final storage.
    for _ in 0..=maximum_rows {
        match maintenance
            .try_discard_retired_scan_staging(&control)
            .expect("retired staging cleanup")
        {
            CatalogMaintenanceAttempt::Completed(0) => return,
            CatalogMaintenanceAttempt::Completed(count) => {
                discarded_rows += count as u64;
                assert!(
                    discarded_rows <= maximum_rows,
                    "cleanup exceeded cancelled work"
                );
            }
            CatalogMaintenanceAttempt::Busy | CatalogMaintenanceAttempt::Interrupted => {
                panic!("isolated cleanup unexpectedly lost writer admission");
            }
        }
    }
    panic!("retired staging cleanup did not reach its empty boundary");
}
