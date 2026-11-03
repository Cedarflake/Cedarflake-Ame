use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};

use crate::adapters::SqliteCatalogSpaceMaintenance;
use crate::application::catalog_reclamation::{
    catalog_reclamation_snapshot, schedule_catalog_reclamation,
};
use crate::domain::CatalogReclamationPhase;
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

pub(super) fn finish_space_reclamation(catalog_path: &Path) {
    let before = std::fs::metadata(catalog_path).unwrap().len();
    let started = Instant::now();
    schedule_catalog_reclamation(catalog_path.to_path_buf());
    loop {
        let snapshot = catalog_reclamation_snapshot(catalog_path).expect("reclamation state");
        if snapshot.phase == CatalogReclamationPhase::Completed {
            let after = std::fs::metadata(catalog_path).unwrap().len();
            println!(
                "AME_SYNTHETIC_RECLAMATION before_bytes={before} after_bytes={after} \
                 reclaimed_bytes={} elapsed_ms={}",
                snapshot.reclaimed_bytes,
                started.elapsed().as_millis(),
            );
            return;
        }
        assert!(snapshot.is_active(), "reclamation failed: {snapshot:?}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "reclamation deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
