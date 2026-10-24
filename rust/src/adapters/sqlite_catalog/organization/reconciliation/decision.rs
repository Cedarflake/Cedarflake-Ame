use rusqlite::{Connection, params};

use crate::domain::ScanError;

use super::super::super::{database_error, unix_time_ms};
use super::restored_authority;

struct OrganizationDecision {
    album_id: Option<String>,
    disposition: String,
    revision: i64,
    conflict: bool,
    restored: bool,
}

pub(super) fn merge(connection: &Connection, canonical: &str) -> Result<(), ScanError> {
    restored_authority::prepare(connection)?;
    let organization = organization_decision(connection)?;
    let (favorite, favorite_revision, favorite_restored): (bool, i64, bool) = connection
        .query_row(
            "SELECT record.favorite, record.favorite_revision, selected.restored
         FROM temp.organization_merge_authority AS selected
         CROSS JOIN organization_records AS record ON selected.record_id = record.record_id
         WHERE selected.field = 'favorite'
         ORDER BY record.favorite_revision DESC, record.favorite DESC, record.record_id LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(database_error)?;
    connection.execute(
        "UPDATE organization_records SET album_id = ?1, disposition = ?2, organization_revision = ?3,
           favorite = ?4, favorite_revision = ?5, updated_unix_ms = ?6 WHERE record_id = ?7",
        params![organization.album_id, organization.disposition, organization.revision,
            favorite, favorite_revision, unix_time_ms(), canonical],
    ).map_err(database_error)?;
    connection.execute(
        "DELETE FROM organization_conflicts WHERE record_id IN (SELECT group_id FROM temp.organization_merge_groups)", [],
    ).map_err(database_error)?;
    if organization.conflict {
        connection
            .execute(
                "INSERT INTO organization_conflicts(record_id, album_id, decision_revision)
             SELECT ?1, album_id, decision_revision FROM temp.organization_merge_albums",
                [canonical],
            )
            .map_err(database_error)?;
    }
    restored_authority::retain(
        connection,
        canonical,
        organization.restored.then_some(organization.revision),
        favorite_restored.then_some(favorite_revision),
    )?;
    Ok(())
}

fn organization_decision(connection: &Connection) -> Result<OrganizationDecision, ScanError> {
    connection.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS organization_merge_albums (
           album_id TEXT PRIMARY KEY NOT NULL, decision_revision INTEGER NOT NULL
         ) WITHOUT ROWID;
         DELETE FROM temp.organization_merge_albums;
         INSERT INTO temp.organization_merge_albums
         SELECT record.album_id, MAX(record.organization_revision) FROM temp.organization_merge_authority AS selected
         CROSS JOIN organization_records AS record ON selected.record_id = record.record_id
         WHERE selected.field = 'organization' AND record.album_id IS NOT NULL GROUP BY record.album_id;
         INSERT INTO temp.organization_merge_albums
         SELECT conflict.album_id, MAX(conflict.decision_revision) FROM temp.organization_merge_authority AS selected
         CROSS JOIN organization_conflicts AS conflict ON selected.record_id = conflict.record_id
         WHERE selected.field = 'organization' GROUP BY conflict.album_id
         ON CONFLICT(album_id) DO UPDATE SET decision_revision = MAX(decision_revision, excluded.decision_revision);"
    ).map_err(database_error)?;
    let (album_count, retained_conflict): (i64, bool) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM temp.organization_merge_albums),
           EXISTS(SELECT 1 FROM temp.organization_merge_authority AS selected
             CROSS JOIN organization_conflicts AS conflict ON selected.record_id = conflict.record_id
             WHERE selected.field = 'organization')",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(database_error)?;
    let (latest_disposition, revision, restored): (String, i64, bool) = connection
        .query_row(
            "SELECT record.disposition, record.organization_revision, selected.restored
         FROM temp.organization_merge_authority AS selected
         CROSS JOIN organization_records AS record ON selected.record_id = record.record_id
         WHERE selected.field = 'organization'
         ORDER BY record.organization_revision DESC, record.record_id LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(database_error)?;
    let conflict = retained_conflict || album_count > 1;
    if conflict {
        return Ok(OrganizationDecision {
            album_id: None,
            disposition: "deferred".into(),
            revision,
            conflict,
            restored,
        });
    }
    if album_count == 1 {
        let album_id = connection
            .query_row(
                "SELECT album_id FROM temp.organization_merge_albums",
                [],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        return Ok(OrganizationDecision {
            album_id: Some(album_id),
            disposition: "organized".into(),
            revision,
            conflict,
            restored,
        });
    }
    Ok(OrganizationDecision {
        album_id: None,
        disposition: latest_disposition,
        revision,
        conflict,
        restored,
    })
}
