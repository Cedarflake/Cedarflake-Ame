use rusqlite::{Connection, Transaction, params};

use crate::domain::ScanError;
use crate::domain::organization_cleanup::{
    OrganizationCleanupInspection, OrganizationCleanupPhase, OrganizationCleanupProgress,
};

use super::super::super::{database_error, load_catalog_revision, unix_time_ms};
use super::super::{association_lifetime, authority};

pub(super) fn stage(
    connection: &Transaction<'_>,
    cutoff: Option<i64>,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
) -> Result<OrganizationCleanupInspection, ScanError> {
    let now = unix_time_ms();
    let cutoff = cutoff.unwrap_or(now);
    if cutoff < 0 || cutoff > now {
        return Err(ScanError::new(
            "organization_cleanup_request_invalid",
            "Invalid cleanup date",
        ));
    }
    super::reset_selection(connection)?;
    let mut after = String::new();
    let mut examined = 0;
    loop {
        super::report(progress, OrganizationCleanupPhase::Inspecting, examined)?;
        let groups = connection
            .prepare(
                "SELECT record_id FROM organization_record_groups
             WHERE record_id = group_id AND record_id > ?1 ORDER BY record_id LIMIT 256",
            )
            .map_err(database_error)?
            .query_map([&after], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let Some(last) = groups.last() else {
            break;
        };
        after = last.clone();
        examined += groups.len() as u64;
        association_lifetime::refresh_groups(connection, &groups, now)?;
        connection
            .execute(
                "INSERT INTO temp.organization_cleanup_groups
             SELECT selected.group_id FROM temp.organization_association_window AS selected
             JOIN organization_records AS record ON record.record_id = selected.group_id
             WHERE selected.associated = 0 AND record.unassociated_since_unix_ms <= ?1
               AND NOT EXISTS(SELECT 1 FROM organization_record_groups AS member
                 JOIN organization_sources AS source ON source.record_id = member.record_id
                 JOIN library_roots AS root ON root.id = source.root_id
                 WHERE member.group_id = selected.group_id AND root.active_scan_id IS NULL)",
                [cutoff],
            )
            .map_err(database_error)?;
    }
    select_duplicate_choices(connection, cutoff, examined, progress)?;
    let mut inspection = OrganizationCleanupInspection {
        cutoff_unix_ms: cutoff,
        inspected_unix_ms: now,
        catalog_revision: load_catalog_revision(connection)?,
        organization_revision: connection
            .query_row(
                "SELECT revision FROM organization_state WHERE singleton = 1",
                [],
                |row| super::super::super::exact_duplicates::read_unsigned(row, 0),
            )
            .map_err(database_error)?,
        selection_digest: String::new(),
        groups: 0,
        album_groups: 0,
        favorite_groups: 0,
        progress_groups: 0,
        duplicate_choices: 0,
        oldest_unassociated_unix_ms: None,
        newest_unassociated_unix_ms: None,
    };
    let mut digest = blake3::Hasher::new();
    authority::field(&mut digest, b"ame-organization-cleanup-v1");
    let mut statement = connection.prepare(
        "SELECT record.record_id, record.album_id, record.disposition, record.favorite,
           record.organization_revision, record.favorite_revision, record.unassociated_since_unix_ms,
           EXISTS(SELECT 1 FROM organization_conflicts AS conflict WHERE conflict.record_id = record.record_id),
           EXISTS(SELECT 1 FROM organization_record_groups AS member
             JOIN organization_review_bookmark AS bookmark ON bookmark.record_id = member.record_id
             WHERE member.group_id = record.record_id)
         FROM temp.organization_cleanup_groups AS selected
         CROSS JOIN organization_records AS record ON record.record_id = selected.group_id
         ORDER BY selected.group_id",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        inspection.groups += 1;
        let album: Option<String> = row.get(1).map_err(database_error)?;
        let disposition: String = row.get(2).map_err(database_error)?;
        let favorite: bool = row.get(3).map_err(database_error)?;
        let conflict: bool = row.get(7).map_err(database_error)?;
        let bookmark: bool = row.get(8).map_err(database_error)?;
        inspection.album_groups += u64::from(album.is_some() || conflict);
        inspection.favorite_groups += u64::from(favorite);
        inspection.progress_groups += u64::from(disposition != "pending" || bookmark);
        include_age(&mut inspection, row.get(6).map_err(database_error)?);
        hash_row(&mut digest, row, 9)?;
        if inspection.groups.is_multiple_of(256) {
            super::report(progress, OrganizationCleanupPhase::Inspecting, examined)?;
        }
    }
    let mut statement = connection.prepare(
        "SELECT choice.scheme, choice.digest, choice.byte_length, choice.preferred_location_id,
           choice.preferred_asset_id, choice.ignored, choice.updated_unix_ms, choice.unassociated_since_unix_ms
         FROM temp.organization_cleanup_duplicates AS selected
         CROSS JOIN organization_recovery_duplicate_decisions AS choice USING(scheme, digest, byte_length)
         ORDER BY selected.scheme, selected.digest, selected.byte_length",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        inspection.duplicate_choices += 1;
        include_age(&mut inspection, row.get(7).map_err(database_error)?);
        hash_row(&mut digest, row, 8)?;
        if inspection.duplicate_choices.is_multiple_of(256) {
            super::report(
                progress,
                OrganizationCleanupPhase::Inspecting,
                examined + inspection.duplicate_choices,
            )?;
        }
    }
    inspection.selection_digest = digest.finalize().to_hex().to_string();
    Ok(inspection)
}

fn select_duplicate_choices(
    connection: &Connection,
    cutoff: i64,
    mut examined: u64,
    progress: &mut dyn FnMut(OrganizationCleanupProgress) -> bool,
) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS organization_cleanup_duplicate_window (
           scheme TEXT NOT NULL, digest TEXT NOT NULL, byte_length INTEGER NOT NULL,
           PRIMARY KEY(scheme, digest, byte_length)
         ) WITHOUT ROWID;",
        )
        .map_err(database_error)?;
    let mut after = (String::new(), String::new(), -1_i64);
    loop {
        super::report(progress, OrganizationCleanupPhase::Inspecting, examined)?;
        connection
            .execute_batch("DELETE FROM temp.organization_cleanup_duplicate_window;")
            .map_err(database_error)?;
        let selected = connection
            .execute(
                "INSERT INTO temp.organization_cleanup_duplicate_window
             SELECT scheme, digest, byte_length FROM organization_recovery_duplicate_decisions
             WHERE (scheme, digest, byte_length) > (?1, ?2, ?3)
             ORDER BY scheme, digest, byte_length LIMIT 256",
                params![after.0, after.1, after.2],
            )
            .map_err(database_error)?;
        if selected == 0 {
            return Ok(());
        }
        after = connection
            .query_row(
                "SELECT scheme, digest, byte_length FROM temp.organization_cleanup_duplicate_window
             ORDER BY scheme DESC, digest DESC, byte_length DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(database_error)?;
        examined += selected as u64;
        connection.execute(
        "INSERT INTO temp.organization_cleanup_duplicates
         SELECT choice.scheme, choice.digest, choice.byte_length
         FROM temp.organization_cleanup_duplicate_window AS pending
         CROSS JOIN organization_recovery_duplicate_decisions AS choice USING(scheme, digest, byte_length)
         WHERE choice.unassociated_since_unix_ms <= ?1
           AND NOT EXISTS(SELECT 1 FROM exact_duplicate_decisions AS current
             WHERE current.scheme = choice.scheme AND current.digest = choice.digest AND current.byte_length = choice.byte_length)
           AND NOT EXISTS(SELECT 1 FROM organization_recovery_duplicate_sources AS source
             JOIN library_roots AS root ON root.id = source.root_id
             LEFT JOIN library_change_root_state AS state ON state.root_id = root.id
             WHERE source.scheme = choice.scheme AND source.digest = choice.digest
               AND source.byte_length = choice.byte_length
               AND (root.active_scan_id IS NULL OR COALESCE(state.is_active, 0) = 0))",
        [cutoff],
        ).map_err(database_error)?;
    }
}

fn include_age(inspection: &mut OrganizationCleanupInspection, since: i64) {
    inspection.oldest_unassociated_unix_ms = Some(
        inspection
            .oldest_unassociated_unix_ms
            .map_or(since, |old| old.min(since)),
    );
    inspection.newest_unassociated_unix_ms = Some(
        inspection
            .newest_unassociated_unix_ms
            .map_or(since, |old| old.max(since)),
    );
}

fn hash_row(
    digest: &mut blake3::Hasher,
    row: &rusqlite::Row<'_>,
    columns: usize,
) -> Result<(), ScanError> {
    use rusqlite::types::ValueRef;
    for index in 0..columns {
        match row.get_ref(index).map_err(database_error)? {
            ValueRef::Null => authority::field(digest, &[0]),
            ValueRef::Integer(value) => {
                authority::field(digest, &[1]);
                authority::field(digest, &value.to_le_bytes());
            }
            ValueRef::Text(value) => {
                authority::field(digest, &[2]);
                authority::field(digest, value);
            }
            _ => return Err(super::changed()),
        }
    }
    Ok(())
}
