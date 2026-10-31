use std::io::BufReader;
use std::path::Path;

use rusqlite::types::{ToSqlOutput, ValueRef};
use rusqlite::{Connection, OpenFlags, ToSql, params_from_iter};
use tempfile::NamedTempFile;

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveAssociations, OrganizationArchiveInspection, OrganizationArchivePhase,
    OrganizationArchiveProgress,
};

use super::{codec, database_error, error, io_error, records, report};

mod classification;
mod classification_schema;
mod input;
mod metadata_filter;
mod schema;
mod validation;

pub(super) struct StagedArchive {
    connection: Connection,
    _file: NamedTempFile,
    verified: codec::VerifiedArchive,
}

pub(super) fn read(
    archive: &Path,
    scratch_directory: &Path,
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
) -> Result<StagedArchive, ScanError> {
    report(progress, OrganizationArchivePhase::Verifying, 0)?;
    let input = input::open(archive)?;
    let file = tempfile::Builder::new()
        .prefix(".ame-restore-stage-")
        .tempfile_in(scratch_directory)
        .map_err(io_error)?;
    let connection = Connection::open_with_flags(file.path(), OpenFlags::SQLITE_OPEN_READ_WRITE)
        .map_err(database_error)?;
    // This database is private scratch. It is never usable after interruption, and its writes
    // must not hold a live-catalog transaction or require one durable flush per input record.
    connection
        .execute_batch(
            "PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;
        PRAGMA temp_store = FILE; PRAGMA cache_size = -2048; PRAGMA foreign_keys = OFF;",
        )
        .map_err(database_error)?;
    connection
        .execute_batch(schema::SQL)
        .map_err(database_error)?;
    connection
        .execute_batch(classification_schema::SQL)
        .map_err(database_error)?;
    let verified = codec::read_records(
        BufReader::new(input),
        &mut |index, values| insert(&connection, index, values),
        &mut |count| report(progress, OrganizationArchivePhase::Verifying, count),
    )?;
    if verified.header.version == 1 {
        connection
            .execute_batch(
                "WITH counts AS MATERIALIZED (
               SELECT file_identity_scheme, file_identity_value, COUNT(*) AS copies
               FROM archive_version_evidence WHERE file_identity_scheme = 'windows-file-id-128-v1'
               GROUP BY file_identity_scheme, file_identity_value
             )
             UPDATE archive_version_evidence AS version SET native_copy_count = COALESCE((
               SELECT copies FROM counts WHERE file_identity_scheme = version.file_identity_scheme
                 AND file_identity_value = version.file_identity_value), 1)
             WHERE native_copy_count = 0;",
            )
            .map_err(database_error)?;
    }
    validation::validate(
        &connection,
        verified.header.organization_revision,
        &mut || {
            report(
                progress,
                OrganizationArchivePhase::Verifying,
                verified.record_count,
            )
        },
    )?;
    classification::validate(&connection, &mut || {
        report(
            progress,
            OrganizationArchivePhase::Verifying,
            verified.record_count,
        )
    })?;
    Ok(StagedArchive {
        connection,
        _file: file,
        verified,
    })
}

fn insert(connection: &Connection, index: usize, values: &[codec::Cell]) -> Result<(), ScanError> {
    let kind = &records::KINDS[index];
    validation::record(kind.name, values)?;
    let placeholders = vec!["?"; kind.fields.len()].join(", ");
    let sql = format!("INSERT INTO archive_{} VALUES ({placeholders})", kind.name);
    connection
        .prepare_cached(&sql)
        .map_err(database_error)?
        .execute(params_from_iter(values))
        .map_err(|cause| write_error(kind.name, cause))?;
    Ok(())
}

fn write_error(kind: &str, cause: rusqlite::Error) -> ScanError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &cause
        && failure.code == rusqlite::ErrorCode::ConstraintViolation
    {
        return error("records_invalid", format!("Invalid {kind} record: {cause}"));
    }
    database_error(cause)
}

impl ToSql for codec::Cell {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(match self {
            Self::Null => ValueRef::Null,
            Self::Integer(value) => ValueRef::Integer(*value),
            Self::Text(value) => ValueRef::Text(value.as_bytes()),
        }))
    }
}

impl StagedArchive {
    pub(super) fn verified(&self) -> &codec::VerifiedArchive {
        &self.verified
    }

    pub(super) fn path(&self) -> &Path {
        self._file.path()
    }

    pub fn inspect(
        &self,
        associations: OrganizationArchiveAssociations,
    ) -> Result<OrganizationArchiveInspection, ScanError> {
        let counts: (i64, i64, i64, i64, i64, bool) = self.connection.query_row(
            "SELECT
               (SELECT COUNT(*) FROM archive_album AS album
                  WHERE NOT EXISTS(SELECT 1 FROM archive_retired_album AS retired WHERE retired.album_id = album.album_id)),
               (SELECT COUNT(*) FROM archive_group WHERE record_id = group_id),
               (SELECT COUNT(*) FROM archive_record AS record JOIN archive_group AS member USING(record_id)
                  WHERE member.record_id = member.group_id AND record.favorite = 1),
               (SELECT COUNT(DISTINCT record_id) FROM archive_conflict),
               (SELECT COUNT(*) FROM archive_duplicate_decision),
               EXISTS(SELECT 1 FROM archive_bookmark)", [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).map_err(database_error)?;
        Ok(OrganizationArchiveInspection {
            archive_id: self.verified.header.archive_id.clone(),
            checksum: self.verified.checksum.clone(),
            created_unix_ms: self.verified.header.created_unix_ms,
            record_count: self.verified.record_count,
            active_albums: counts.0 as u64,
            groups: counts.1 as u64,
            favorite_groups: counts.2 as u64,
            conflicting_groups: counts.3 as u64,
            duplicate_choices: counts.4 as u64,
            has_continuation: counts.5,
            associations,
        })
    }
}

#[cfg(test)]
mod tests;
