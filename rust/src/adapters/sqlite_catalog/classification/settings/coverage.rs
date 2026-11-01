use std::sync::{Arc, atomic::AtomicBool};

use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::maintenance::{CategoryCoveragePage, CategoryCoverageSnapshot};
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification::{ClassificationContent, ModelCompatibility};
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;
use crate::ports::classification_configuration::CategoryCoverageRepository;

use super::super::super::exact_duplicates::read_unsigned;
use super::{SqliteCatalog, changed, database_error};

mod query;
mod transaction;
pub(super) use transaction::interruptible;

impl CategoryCoverageRepository for SqliteCatalog {
    fn begin_category_coverage(
        &self,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryCoverageSnapshot, ScanError> {
        transaction::read(&self.connection, cancelled, |connection| {
            let (catalog_revision, configuration, model) = current(connection)?;
            Ok(CategoryCoverageSnapshot {
                catalog_revision,
                configuration,
                model,
                unverified_sources: query::unverified_sources(connection)?,
            })
        })
    }

    fn read_category_coverage(
        &self,
        snapshot: &CategoryCoverageSnapshot,
        after: Option<&ClassificationContent>,
        limit: usize,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<CategoryCoveragePage, ScanError> {
        if !(1..=64).contains(&limit) {
            return Err(changed());
        }
        transaction::read(&self.connection, cancelled, |connection| {
            validate(connection, snapshot)?;
            query::page(connection, snapshot, after, limit)
        })
    }

    fn validate_category_coverage(
        &self,
        snapshot: &CategoryCoverageSnapshot,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<(), ScanError> {
        transaction::read(&self.connection, cancelled, |connection| {
            validate_snapshot(connection, snapshot)
        })
    }
}

pub(super) fn validate_snapshot(
    connection: &Connection,
    snapshot: &CategoryCoverageSnapshot,
) -> Result<(), ScanError> {
    validate(connection, snapshot)?;
    if query::unverified_sources(connection)? != snapshot.unverified_sources {
        return Err(changed());
    }
    Ok(())
}

fn current(
    connection: &Connection,
) -> Result<(u64, ClassificationConfigurationSnapshot, ModelCompatibility), ScanError> {
    let configuration = super::load(connection)?;
    if configuration.dispatch.mode != ClassificationDispatchMode::Automatic {
        return Err(changed());
    }
    let model = super::authority::active_model(connection, &configuration)?.ok_or_else(changed)?;
    let revision = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            read_unsigned(row, 0)
        })
        .map_err(database_error)?;
    Ok((revision, configuration, model))
}

fn validate(connection: &Connection, expected: &CategoryCoverageSnapshot) -> Result<(), ScanError> {
    let (revision, configuration, model) = current(connection)?;
    if revision != expected.catalog_revision
        || configuration != expected.configuration
        || model != expected.model
    {
        return Err(changed());
    }
    Ok(())
}
