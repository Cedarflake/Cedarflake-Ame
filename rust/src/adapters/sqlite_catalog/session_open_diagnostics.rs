use std::time::{Duration, Instant};

use crate::domain::{LibraryChangeLane, ScanError};

use super::{SqliteCatalog, SqliteCatalogReadStage, SqliteCatalogSession};

impl SqliteCatalogSession {
    pub(super) fn open_in_lane_with_diagnostics(
        &self,
        lane: LibraryChangeLane,
    ) -> Result<SqliteCatalog, ScanError> {
        let started = Instant::now();
        let mut previous = started;
        let mut identity_ms = None;
        let mut open_config_ms = None;
        let result = self.open_in_lane_with_read_stage(lane, |stage| {
            let now = Instant::now();
            let elapsed = now.duration_since(previous).as_millis();
            match stage {
                SqliteCatalogReadStage::Open => identity_ms = Some(elapsed),
                SqliteCatalogReadStage::Validation => open_config_ms = Some(elapsed),
                SqliteCatalogReadStage::Query => unreachable!("opening does not run a user query"),
            }
            previous = now;
            Ok(())
        });
        let elapsed = started.elapsed();
        if elapsed >= Duration::from_millis(100) {
            eprintln!(
                "[Ame sync catalog open] lane={lane:?} total_ms={} identity_ms={identity_ms:?} \
                 open_config_ms={open_config_ms:?} tail_ms={} outcome={}",
                elapsed.as_millis(),
                previous.elapsed().as_millis(),
                if result.is_ok() { "ok" } else { "error" },
            );
        }
        result
    }
}
