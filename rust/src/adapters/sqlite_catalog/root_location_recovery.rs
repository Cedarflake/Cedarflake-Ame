use std::path::{Component, Path};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{OptionalExtension, Transaction, params};

use crate::adapters::LocatedLibraryRoot;
use crate::domain::{
    IncrementalCatalogRoot, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLane,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    ScanError,
};

use super::change_queue::{activate_root_change_queue, enqueue_intents_in_transaction};
use super::scan_admission::root_binding::RootScanBinding;
use super::{SqliteCatalog, database_error, sqlite_integer, unix_time_ms};

mod retirement;
mod write_attempt;

use write_attempt::LocationWriteAttempt;

struct NamespaceOrigin {
    kind: String,
    catalog_revision: i64,
    established_unix_ms: i64,
    updated_unix_ms: i64,
}

impl SqliteCatalog {
    pub(crate) fn recover_located_root(
        &mut self,
        expected: &IncrementalCatalogRoot,
        located: &LocatedLibraryRoot,
        cancelled: &AtomicBool,
    ) -> Result<(), ScanError> {
        let attempt = LocationWriteAttempt::new();
        let _retirement = attempt.retirement_guard();
        let progress = attempt.clone();
        self.connection
            .progress_handler(1000, Some(move || progress.is_interrupted()))
            .map_err(database_error)?;
        let result = self.recover_root_in_attempt(expected, located, cancelled, &attempt);
        let _handler_cleanup = self.connection.progress_handler(0, None::<fn() -> bool>);
        result.map_err(|error| attempt.classify(error))
    }

    fn recover_root_in_attempt(
        &mut self,
        expected: &IncrementalCatalogRoot,
        located: &LocatedLibraryRoot,
        cancelled: &AtomicBool,
        attempt: &LocationWriteAttempt,
    ) -> Result<(), ScanError> {
        check_cancelled(cancelled)?;
        let identity = expected.publication_root_identity.as_ref().ok_or_else(|| {
            conflict("The missing directory has no retained publication identity")
        })?;
        located
            .guard()
            .require_metadata_inventory_root_identity(identity)?;
        let now = unix_time_ms();
        let interrupt = Arc::new(self.connection.get_interrupt_handle());
        let transaction = self.begin_preemptible_write_in_lane(
            LibraryChangeLane::Recovery,
            attempt.callback(move || interrupt.interrupt()),
        )?;
        // Retire callbacks before rollback, while the transaction still owns its write permit.
        let _transaction_retirement = attempt.retirement_guard();
        attempt.ensure_running()?;
        let is_published = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_roots AS root
                   WHERE root.id = ?1 AND root.active_scan_id IS NOT NULL
                     AND root.active_scan_id = ?2
                     AND NOT EXISTS(
                       SELECT 1 FROM scan_runs
                       WHERE root_id = root.id AND status IN ('running', 'paused')
                     )
                 )",
                params![expected.root_id, expected.active_scan_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !is_published || expected.root_path == located.path() {
            return Err(conflict(
                "The published directory binding changed before recovery",
            ));
        }
        let origin = transaction
            .query_row(
                "SELECT authority_kind, established_catalog_revision,
                        established_unix_ms, updated_unix_ms
                 FROM library_root_publication_namespaces
                 WHERE root_id = ?1 AND root_generation = ?2",
                params![
                    expected.root_id,
                    sqlite_integer(expected.root_generation.value(), "root generation")?
                ],
                |row| {
                    Ok(NamespaceOrigin {
                        kind: row.get(0)?,
                        catalog_revision: row.get(1)?,
                        established_unix_ms: row.get(2)?,
                        updated_unix_ms: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                ScanError::new(
                    "catalog_root_relocation_stale",
                    "The retained directory generation changed before recovery",
                )
            })?;
        let binding = RootScanBinding::IdentityRecovered { expected, identity };
        binding.validate(&transaction, &expected.root_id, located.path())?;
        retirement::retire_inventory(&transaction, expected, now, cancelled, attempt)?;
        RootScanBinding::IdentityRecovered { expected, identity }.validate_and_retire(
            &transaction,
            &expected.root_id,
            located.path(),
            now,
        )?;
        let generation = activate_root_change_queue(&transaction, &expected.root_id, now)?;
        rebase_locations(&transaction, expected, located.path(), cancelled, attempt)?;
        transaction
            .execute(
                "UPDATE library_roots SET path = ?2 WHERE id = ?1",
                params![expected.root_id, located.path()],
            )
            .map_err(database_error)?;
        transaction
            .execute(
                "INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value, authority_kind,
                   established_catalog_revision, established_unix_ms, updated_unix_ms
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    expected.root_id,
                    sqlite_integer(generation.value(), "root generation")?,
                    identity.scheme,
                    identity.value,
                    origin.kind,
                    origin.catalog_revision,
                    origin.established_unix_ms,
                    origin.updated_unix_ms.max(now)
                ],
            )
            .map_err(database_error)?;
        transaction
            .execute("UPDATE catalog_state SET revision = revision + 1", [])
            .map_err(database_error)?;
        let report = enqueue_intents_in_transaction(
            &transaction,
            &[recovery_gap(expected, generation, now)],
            None,
            now,
            LibraryChangeQueuePolicy::default(),
        )?;
        if report.stale_generation_count != 0 {
            return Err(conflict(
                "Directory recovery could not retain its evidence gap",
            ));
        }
        check_cancelled(cancelled)?;
        located
            .guard()
            .require_metadata_inventory_root_identity(identity)?;
        attempt.ensure_running()?;
        attempt.retire();
        transaction.commit().map_err(database_error)
    }
}

fn rebase_locations(
    transaction: &Transaction<'_>,
    root: &IncrementalCatalogRoot,
    new_path: &str,
    cancelled: &AtomicBool,
    attempt: &LocationWriteAttempt,
) -> Result<(), ScanError> {
    let mut read = transaction
        .prepare(
            "SELECT location_id, relative_path FROM asset_locations
         WHERE root_id = ?1 AND scan_id = ?2
           AND (relative_path, location_id) > (?3, ?4)
         ORDER BY relative_path, location_id LIMIT 128",
        )
        .map_err(database_error)?;
    let mut update = transaction
        .prepare(
            "UPDATE asset_locations SET absolute_path = ?3
         WHERE location_id = ?1 AND scan_id = ?2",
        )
        .map_err(database_error)?;
    let mut after_relative = String::new();
    let mut after_location = String::new();
    loop {
        check_cancelled(cancelled)?;
        attempt.ensure_running()?;
        let rows = read
            .query_map(
                params![
                    root.root_id,
                    root.active_scan_id,
                    after_relative,
                    after_location
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(database_error)?;
        let page = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        if page.is_empty() {
            return Ok(());
        }
        for (location_id, relative) in page {
            let relative_path = Path::new(&relative);
            if relative.is_empty()
                || relative.contains(['\0', ':'])
                || relative_path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(conflict(
                    "The published location has an unsafe relative path",
                ));
            }
            let absolute = Path::new(new_path).join(relative_path);
            update
                .execute(params![
                    location_id,
                    root.active_scan_id,
                    absolute.to_string_lossy()
                ])
                .map_err(database_error)?;
            after_relative = relative;
            after_location = location_id;
        }
        #[cfg(test)]
        tests::cost::after_recovery_write_page(&root.root_id);
    }
}

fn recovery_gap(
    root: &IncrementalCatalogRoot,
    generation: LibraryRootGeneration,
    now: i64,
) -> LibraryChangeIntent {
    LibraryChangeIntent {
        root_id: root.root_id.clone(),
        root_generation: generation,
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::LiveNotification,
        first_observed_unix_ms: now,
        most_recent_observed_unix_ms: now,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ScanError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ScanError::new(
            "root_location_recovery_cancelled",
            "Directory recovery was cancelled",
        ))
    } else {
        Ok(())
    }
}

fn conflict(message: &str) -> ScanError {
    ScanError::new("root_location_recovery_conflict", message)
}

#[cfg(test)]
mod tests;
