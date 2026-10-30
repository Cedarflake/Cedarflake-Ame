use std::collections::BTreeSet;

use rusqlite::{Connection, params, params_from_iter, types::Value};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::organization::{
    OrganizationGroupKey, OrganizationGroupTarget, OrganizationPreparedSelection,
    OrganizationSelection, stale_selection,
};

use super::super::exact_duplicate_review::DECISION_JOIN;
use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, sqlite_integer};
use super::authority::field;

pub(in crate::adapters::sqlite_catalog) fn prepare(
    connection: &Connection,
    selections: &[OrganizationSelection],
) -> Result<OrganizationPreparedSelection, ScanError> {
    let selections = selections
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let catalog_revision = connection
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            read_unsigned(row, 0)
        })
        .map_err(database_error)?;
    let organization_revision = connection
        .query_row(
            "SELECT revision FROM organization_state WHERE singleton = 1",
            [],
            |row| read_unsigned(row, 0),
        )
        .map_err(database_error)?;
    let keys = selected_keys(connection, &selections)?;
    let groups = keys
        .iter()
        .map(|key| target(connection, key))
        .collect::<Result<_, _>>()?;
    Ok(OrganizationPreparedSelection {
        catalog_revision,
        organization_revision,
        selections,
        groups,
    })
}

pub(in crate::adapters::sqlite_catalog) fn selected_keys(
    connection: &Connection,
    selections: &[OrganizationSelection],
) -> Result<BTreeSet<OrganizationGroupKey>, ScanError> {
    let mut values = vec![Value::Text(EXACT_FINGERPRINT_SCHEME.to_owned())];
    let mut placeholders = Vec::with_capacity(selections.len());
    for selection in selections {
        let start = values.len() + 1;
        placeholders.push(format!("(?{start}, ?{}, ?{})", start + 1, start + 2));
        values.extend([
            Value::Text(selection.asset_id.clone()),
            Value::Text(selection.location_id.clone()),
            Value::Integer(sqlite_integer(
                selection.source_generation,
                "source generation",
            )?),
        ]);
    }
    let sql = format!(
        "WITH selected(asset_id, location_id, source_generation) AS (VALUES {})
         SELECT l.asset_id, l.source_generation, f.digest, f.byte_length,
           EXISTS(SELECT 1 FROM gallery_retained_fingerprints AS retained
             WHERE retained.source_generation = l.source_generation AND retained.scheme = ?1)
         FROM {CURRENT_LOCATIONS}
         JOIN selected ON selected.asset_id = l.asset_id AND selected.location_id = l.location_id
           AND selected.source_generation = l.source_generation
         LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}",
        placeholders.join(", ")
    );
    let mut statement = connection.prepare(&sql).map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(values))
        .map_err(database_error)?;
    let mut count = 0;
    let mut keys = BTreeSet::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let digest: Option<String> = row.get(2).map_err(database_error)?;
        let key = if let Some(digest) = digest {
            OrganizationGroupKey::Confirmed {
                digest,
                byte_length: read_unsigned(row, 3).map_err(database_error)?,
            }
        } else {
            if row.get::<_, bool>(4).map_err(database_error)? {
                return Err(pending_evidence());
            }
            OrganizationGroupKey::Provisional {
                asset_id: row.get(0).map_err(database_error)?,
                source_generation: read_unsigned(row, 1).map_err(database_error)?,
            }
        };
        keys.insert(key);
        count += 1;
    }
    if count != selections.len() {
        return Err(stale_selection());
    }
    Ok(keys)
}

pub(super) struct MemberQuery {
    pub from: String,
    pub predicate: &'static str,
    pub values: Vec<Value>,
}

pub(super) fn member_query(key: &OrganizationGroupKey) -> Result<MemberQuery, ScanError> {
    let mut values = vec![Value::Text(EXACT_FINGERPRINT_SCHEME.to_owned())];
    let predicate = match key {
        OrganizationGroupKey::Confirmed {
            digest,
            byte_length,
        } => {
            values.push(Value::Text(digest.clone()));
            values.push(Value::Integer(sqlite_integer(
                *byte_length,
                "content length",
            )?));
            "f.digest = ?2 AND f.byte_length = ?3"
        }
        OrganizationGroupKey::Provisional {
            asset_id,
            source_generation,
        } => {
            values.push(Value::Text(asset_id.clone()));
            values.push(Value::Integer(sqlite_integer(
                *source_generation,
                "source generation",
            )?));
            "l.asset_id = ?2 AND l.source_generation = ?3 AND f.source_generation IS NULL"
        }
    };
    Ok(MemberQuery {
        from: format!(
            "{CURRENT_LOCATIONS} LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}"
        ),
        predicate,
        values,
    })
}

pub(super) fn target(
    connection: &Connection,
    key: &OrganizationGroupKey,
) -> Result<OrganizationGroupTarget, ScanError> {
    if let OrganizationGroupKey::Confirmed {
        digest,
        byte_length,
    } = key
    {
        let pending_sql = format!(
            "SELECT EXISTS(SELECT 1 FROM {CURRENT_LOCATIONS}
             JOIN gallery_retained_fingerprints AS retained ON retained.source_generation = l.source_generation AND retained.scheme = ?1
             JOIN content_fingerprints AS old ON old.source_generation = retained.evidence_source_generation AND old.scheme = ?1
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             WHERE old.digest = ?2 AND old.byte_length = ?3 AND f.source_generation IS NULL)"
        );
        let pending: bool = connection
            .prepare_cached(&pending_sql)
            .map_err(database_error)?
            .query_row(
                params![
                    EXACT_FINGERPRINT_SCHEME,
                    digest,
                    sqlite_integer(*byte_length, "content length")?
                ],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if pending {
            return Err(pending_evidence());
        }
    }
    let query = member_query(key)?;
    let sql = format!(
        "SELECT l.asset_id, l.location_id, l.source_generation, l.root_id, l.scan_id,
          state.generation, l.absolute_path, l.file_size, l.file_identity_scheme,
          l.file_identity_value, l.source_revision_token,
          COALESCE(l.location_id = decision.preferred_location_id AND l.asset_id = decision.preferred_asset_id, 0)
         FROM {} {DECISION_JOIN} WHERE {} ORDER BY l.location_id",
        query.from, query.predicate
    );
    let mut statement = connection.prepare_cached(&sql).map_err(database_error)?;
    let mut rows = statement
        .query(params_from_iter(query.values))
        .map_err(database_error)?;
    let mut hasher = blake3::Hasher::new();
    field(&mut hasher, b"ame-organization-members-v1");
    let mut representative = None;
    let mut location_count = 0;
    while let Some(row) = rows.next().map_err(database_error)? {
        let selection = OrganizationSelection {
            asset_id: row.get(0).map_err(database_error)?,
            location_id: row.get(1).map_err(database_error)?,
            source_generation: read_unsigned(row, 2).map_err(database_error)?,
        };
        for column in [0, 1, 3, 4, 6] {
            field(
                &mut hasher,
                row.get::<_, String>(column)
                    .map_err(database_error)?
                    .as_bytes(),
            );
        }
        for column in [2, 5, 7] {
            field(
                &mut hasher,
                &row.get::<_, i64>(column)
                    .map_err(database_error)?
                    .to_le_bytes(),
            );
        }
        for column in [8, 9, 10] {
            let value: Option<String> = row.get(column).map_err(database_error)?;
            field(&mut hasher, &[u8::from(value.is_some())]);
            if let Some(value) = value {
                field(&mut hasher, value.as_bytes());
            }
        }
        if representative.is_none() || row.get::<_, bool>(11).map_err(database_error)? {
            representative = Some(selection);
        }
        location_count += 1;
    }
    Ok(OrganizationGroupTarget {
        key: key.clone(),
        representative: representative.ok_or_else(stale_selection)?,
        location_count,
        membership_authority: hasher.finalize().to_hex().to_string(),
    })
}

fn pending_evidence() -> ScanError {
    ScanError::new(
        "organization_evidence_pending",
        "The displayed group is still being verified; try again when its content is confirmed",
    )
}
