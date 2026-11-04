use std::path::Path;
use std::thread;

use crate::domain::{CatalogAutoVacuumMode, CatalogReclamationPhase, ScanError};
use crate::ports::CatalogSpaceRepository;
use crate::ports::catalog_capacity::CatalogCapacity;

use super::{
    CatalogReclamationOperation, ReclamationCompletion, apply_usage, checkpoint_with_retry,
    convert_with_retry, inspect_with_retry, reclaim_batch_with_retry, reclamation_target_reached,
    required_vacuum_capacity, scan_staging, should_start_reclamation,
};

pub(super) fn run(
    path: &Path,
    operation: &CatalogReclamationOperation,
    maintenance: &impl CatalogSpaceRepository,
    capacity: &impl CatalogCapacity,
) -> Result<ReclamationCompletion, ScanError> {
    if scan_staging::discard_retired_staging(maintenance, operation)?.is_none() {
        return Ok(ReclamationCompletion::Cancelled);
    }
    let Some(mut usage) = inspect_with_retry(maintenance, operation)? else {
        return Ok(ReclamationCompletion::Cancelled);
    };
    operation.update(|snapshot| apply_usage(snapshot, usage));

    if usage.auto_vacuum == CatalogAutoVacuumMode::Full {
        let Some(converted) = convert_with_retry(path, maintenance, operation)? else {
            return Ok(ReclamationCompletion::Cancelled);
        };
        usage = converted;
        operation.update(|snapshot| apply_usage(snapshot, usage));
    }

    if should_start_reclamation(usage) && usage.auto_vacuum == CatalogAutoVacuumMode::None {
        operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::CheckingCapacity);
        let required = required_vacuum_capacity(usage.catalog_file_bytes())?;
        let available = capacity.available_for(path)?;
        operation.update(|snapshot| {
            snapshot.required_temporary_bytes = Some(required);
            snapshot.available_temporary_bytes = Some(available);
        });
        if available < required {
            return Err(ScanError::new(
                "catalog_reclamation_space_insufficient",
                format!(
                    "Catalog compaction requires {required} bytes of free space, but only {available} bytes are available"
                ),
            ));
        }
        let Some(converted) = convert_with_retry(path, maintenance, operation)? else {
            return Ok(ReclamationCompletion::Cancelled);
        };
        usage = converted;
        operation.update(|snapshot| apply_usage(snapshot, usage));
    }

    if should_start_reclamation(usage) && usage.auto_vacuum == CatalogAutoVacuumMode::Incremental {
        while !reclamation_target_reached(usage) {
            if operation.is_cancelled() {
                return Ok(ReclamationCompletion::Cancelled);
            }
            operation.update(|snapshot| snapshot.phase = CatalogReclamationPhase::Reclaiming);
            let Some(next) = reclaim_batch_with_retry(path, maintenance, operation)? else {
                return Ok(ReclamationCompletion::Cancelled);
            };
            if next.freelist_count >= usage.freelist_count && next.page_count >= usage.page_count {
                return Err(ScanError::new(
                    "catalog_reclamation_no_progress",
                    "Incremental catalog reclamation could not release another page",
                ));
            }
            usage = next;
            operation.update(|snapshot| apply_usage(snapshot, usage));
            thread::yield_now();
        }
    }

    let Some(()) = checkpoint_with_retry(maintenance, operation)? else {
        return Ok(ReclamationCompletion::Cancelled);
    };

    Ok(ReclamationCompletion::Completed)
}

#[cfg(test)]
mod tests;
