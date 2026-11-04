use crate::domain::{RecoverableScan, ScanError};
use crate::ports::scan_recovery::ForegroundScanRecovery;

use super::super::{ScanOwner, SqliteCatalog, database_error, load_scan_with_status, unix_time_ms};
use super::abandon_scan_transaction;

impl ForegroundScanRecovery for SqliteCatalog {
    fn load_single_recoverable_foreground_scan(
        &mut self,
    ) -> Result<Option<RecoverableScan>, ScanError> {
        self.converge_single_recoverable_foreground_scan()?;
        load_scan_with_status(&self.connection, "running", ScanOwner::Foreground)
    }

    fn load_single_paused_foreground_scan(&mut self) -> Result<Option<RecoverableScan>, ScanError> {
        self.converge_single_recoverable_foreground_scan()?;
        load_scan_with_status(&self.connection, "paused", ScanOwner::Foreground)
    }
}

impl SqliteCatalog {
    fn converge_single_recoverable_foreground_scan(&mut self) -> Result<(), ScanError> {
        let retired_scan_ids = {
            let completed_unix_ms = unix_time_ms();
            let transaction = self.begin_write()?;
            let unfinished_scans = {
                let mut statement = transaction
                    .prepare(
                        "SELECT scans.id, scans.issue_count,
                                roots.active_scan_id IS NOT NULL AS has_published_root
                         FROM scan_runs AS scans
                         JOIN library_roots AS roots ON roots.id = scans.root_id
                         WHERE scans.status IN ('running', 'paused')
                           AND scans.scan_owner = 'foreground'
                         ORDER BY CASE scans.status WHEN 'paused' THEN 0 ELSE 1 END,
                                  scans.started_unix_ms DESC, scans.id DESC",
                    )
                    .map_err(database_error)?;
                let rows = statement
                    .query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, bool>(2)?,
                        ))
                    })
                    .map_err(database_error)?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(database_error)?
            };
            let mut retired_scan_ids = Vec::new();
            let mut retained_initial_import = false;
            for (scan_id, issue_count, has_published_root) in unfinished_scans {
                if !has_published_root && !retained_initial_import {
                    retained_initial_import = true;
                    continue;
                }
                abandon_scan_transaction(
                    &transaction,
                    &scan_id,
                    "failed",
                    issue_count,
                    completed_unix_ms,
                )?;
                retired_scan_ids.push(scan_id);
            }
            transaction.commit().map_err(database_error)?;
            retired_scan_ids
        };
        if !retired_scan_ids.is_empty() {
            self.pending_locations.retain(|pending| {
                !retired_scan_ids
                    .iter()
                    .any(|scan_id| scan_id == &pending.scan_id)
            });
        }
        Ok(())
    }
}
