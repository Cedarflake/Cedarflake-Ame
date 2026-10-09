use rusqlite::{Connection, params};

use crate::domain::{FileIdentityEvidence, ScanError};

use super::database_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StoredIdentityGroupState {
    pub(super) generation: i64,
    pub(super) source_revision_token: Option<String>,
    pub(super) file_size: i64,
    pub(super) modified_unix_ms: i64,
}

pub(super) fn load_active_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_identity_group_state_query(
        connection,
        "SELECT MIN(locations.source_generation), MAX(locations.source_generation),
                COUNT(DISTINCT locations.source_revision_token),
                MAX(locations.source_revision_token),
                MIN(locations.file_size), MAX(locations.file_size),
                MIN(locations.modified_unix_ms), MAX(locations.modified_unix_ms)
         FROM asset_locations AS locations
         JOIN library_roots AS roots
           ON roots.id = locations.root_id
          AND roots.active_scan_id = locations.scan_id
         WHERE locations.file_identity_scheme = ?1
           AND locations.file_identity_value = ?2",
        params![identity.scheme, identity.value],
    )
}

pub(super) fn load_scan_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
    scan_id: &str,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_identity_group_state_query(
        connection,
        "SELECT MIN(source_generation), MAX(source_generation),
                COUNT(DISTINCT source_revision_token), MAX(source_revision_token),
                MIN(file_size), MAX(file_size),
                MIN(modified_unix_ms), MAX(modified_unix_ms)
         FROM asset_locations
         WHERE file_identity_scheme = ?1 AND file_identity_value = ?2
           AND scan_id = ?3",
        params![identity.scheme, identity.value, scan_id],
    )
}

pub(super) fn load_staging_identity_group_state(
    connection: &Connection,
    identity: &FileIdentityEvidence,
    scan_id: &str,
) -> Result<Option<StoredIdentityGroupState>, ScanError> {
    load_scan_identity_group_state(connection, identity, scan_id)?.map_or_else(
        || load_active_identity_group_state(connection, identity),
        |state| Ok(Some(state)),
    )
}

fn load_identity_group_state_query<P>(
    connection: &Connection,
    query: &str,
    parameters: P,
) -> Result<Option<StoredIdentityGroupState>, ScanError>
where
    P: rusqlite::Params,
{
    let (
        minimum_generation,
        maximum_generation,
        known_revision_count,
        source_revision_token,
        minimum_file_size,
        maximum_file_size,
        minimum_modified_unix_ms,
        maximum_modified_unix_ms,
    ) = connection
        .prepare_cached(query)
        .map_err(database_error)?
        .query_row(parameters, |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })
        .map_err(database_error)?;
    let Some(generation) = minimum_generation else {
        return Ok(None);
    };
    if maximum_generation != Some(generation)
        || known_revision_count > 1
        || minimum_file_size != maximum_file_size
        || minimum_modified_unix_ms != maximum_modified_unix_ms
    {
        return Err(ScanError::new(
            "catalog_source_identity_state_unverifiable",
            "The catalog contains conflicting source state for one physical file identity",
        ));
    }
    Ok(Some(StoredIdentityGroupState {
        generation,
        source_revision_token,
        file_size: minimum_file_size.expect("an identity group has a file size"),
        modified_unix_ms: minimum_modified_unix_ms
            .expect("an identity group has a modification time"),
    }))
}

#[cfg(test)]
mod tests;
