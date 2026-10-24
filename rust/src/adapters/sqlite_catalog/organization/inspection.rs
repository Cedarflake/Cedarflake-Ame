use rusqlite::{Connection, params_from_iter, types::Value};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::{
    OrganizationAlbum, OrganizationDisposition, OrganizationGroupDecision, OrganizationGroupKey,
    OrganizationInspection, OrganizationSelection,
};

use super::super::{database_error, exact_duplicates::read_unsigned, sqlite_integer};
use super::selection;

pub(super) fn load(
    connection: &Connection,
    selections: &[OrganizationSelection],
) -> Result<OrganizationInspection, ScanError> {
    let selections = selections
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let (catalog_revision, organization_revision) = connection
        .query_row(
            "SELECT catalog.revision, organization.revision
             FROM catalog_state AS catalog, organization_state AS organization
             WHERE organization.singleton = 1",
            [],
            |row| Ok((read_unsigned(row, 0)?, read_unsigned(row, 1)?)),
        )
        .map_err(database_error)?;
    let decisions = selection::selected_keys(connection, &selections)?
        .iter()
        .map(|key| decision(connection, key))
        .collect::<Result<_, _>>()?;
    Ok(OrganizationInspection {
        catalog_revision,
        organization_revision,
        decisions,
    })
}

fn decision(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<OrganizationGroupDecision, ScanError> {
    let Some(record_id) = record(connection, key)? else {
        return Ok(OrganizationGroupDecision {
            key: key.clone(),
            favorite: false,
            disposition: OrganizationDisposition::Pending,
            album: None,
            has_conflict: false,
        });
    };
    connection
        .query_row(
            "SELECT record.favorite, record.disposition,
                    album.album_id, album.name, album.revision,
                    EXISTS(
                        SELECT 1 FROM organization_conflicts AS conflict
                        WHERE conflict.record_id = record.record_id
                    )
             FROM organization_records AS record
             LEFT JOIN organization_albums AS album ON album.album_id = record.album_id
             WHERE record.record_id = ?1",
            [record_id],
            |row| {
                let disposition = match row.get::<_, String>(1)?.as_str() {
                    "pending" => OrganizationDisposition::Pending,
                    "deferred" => OrganizationDisposition::Deferred,
                    "organized" => OrganizationDisposition::Organized,
                    "ignored" => OrganizationDisposition::Ignored,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                let album = match row.get::<_, Option<String>>(2)? {
                    Some(album_id) => Some(OrganizationAlbum {
                        album_id,
                        name: row.get(3)?,
                        revision: read_unsigned(row, 4)?,
                    }),
                    None => None,
                };
                Ok(OrganizationGroupDecision {
                    key: key.clone(),
                    favorite: row.get(0)?,
                    disposition,
                    album,
                    has_conflict: row.get(5)?,
                })
            },
        )
        .map_err(database_error)
}

fn record(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<Option<String>, ScanError> {
    let members = selection::member_query(key)?;
    let mut values = members.values;
    let mut sql = format!(
        "SELECT mapping.group_id FROM {} JOIN organization_bindings AS binding
           ON binding.asset_id = l.asset_id AND binding.source_generation = l.source_generation
         JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
         WHERE {}",
        members.from, members.predicate,
    );
    if let OrganizationGroupKey::Confirmed {
        digest,
        byte_length,
    } = key
    {
        let start = values.len() + 1;
        sql.push_str(&format!(
            " UNION SELECT mapping.group_id FROM organization_bindings AS binding
              JOIN organization_record_groups AS mapping ON mapping.record_id = binding.record_id
              WHERE binding.fingerprint_scheme = ?{start} AND binding.digest = ?{}
                AND binding.byte_length = ?{}",
            start + 1,
            start + 2,
        ));
        values.extend([
            Value::Text(EXACT_FINGERPRINT_SCHEME.into()),
            Value::Text(digest.clone()),
            Value::Integer(sqlite_integer(*byte_length, "content byte length")?),
        ]);
    }
    let mut statement = connection
        .prepare(&format!("SELECT DISTINCT group_id FROM ({sql}) LIMIT 2"))
        .map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(values))
        .map_err(database_error)?;
    let first = rows
        .next()
        .map_err(database_error)?
        .map(|row| row.get(0))
        .transpose()
        .map_err(database_error)?;
    if rows.next().map_err(database_error)?.is_some() {
        return Err(ScanError::new(
            "organization_evidence_pending",
            "The selected group is still being reconciled; retry after publication",
        ));
    }
    Ok(first)
}
