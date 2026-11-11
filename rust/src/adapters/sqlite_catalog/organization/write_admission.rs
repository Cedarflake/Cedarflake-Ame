use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::organization::{
    OrganizationGroupTarget, OrganizationPreparedSelection, stale_selection,
};

use super::super::{database_error, exact_duplicates::read_unsigned};
use super::{inspection, operations, selection};

#[derive(Clone, Copy)]
pub(super) enum DecisionField {
    Organization,
    Favorite,
}

pub(super) fn admit(
    connection: &Connection,
    operation_id: &str,
    digest: &str,
    field: DecisionField,
    expected_revision: u64,
) -> Result<operations::Admission, ScanError> {
    let revision = connection
        .query_row("SELECT revision FROM organization_state", [], |row| {
            read_unsigned(row, 0)
        })
        .map_err(database_error)?;
    if revision < expected_revision {
        return Err(stale_selection());
    }
    let kind = match field {
        DecisionField::Organization => "organization",
        DecisionField::Favorite => "favorite",
    };
    // The caller validates every selected group inside this same write transaction.
    operations::admit(connection, operation_id, digest, kind, revision)
}

pub(super) fn validate_selection(
    connection: &Connection,
    prepared: &OrganizationPreparedSelection,
    field: DecisionField,
) -> Result<(), ScanError> {
    let keys = selection::selected_keys(connection, &prepared.selections)?;
    if !keys
        .iter()
        .eq(prepared.groups.iter().map(|group| &group.key))
    {
        return Err(stale_selection());
    }
    for group in &prepared.groups {
        validate_group(connection, group, prepared.organization_revision, field)?;
    }
    Ok(())
}

pub(super) fn validate_group(
    connection: &Connection,
    target: &OrganizationGroupTarget,
    expected_revision: u64,
    field: DecisionField,
) -> Result<(), ScanError> {
    if target != &selection::target(connection, &target.key)? {
        return Err(stale_selection());
    }
    if let Some(version) = inspection::inspect_group(connection, &target.key)?.version {
        let revision = match field {
            DecisionField::Organization => version.organization_revision,
            DecisionField::Favorite => version.favorite_revision,
        };
        if revision > expected_revision {
            return Err(stale_selection());
        }
    }
    Ok(())
}
