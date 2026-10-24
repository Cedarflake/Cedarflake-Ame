use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::types::ValueRef;

use crate::domain::ScanError;
use crate::domain::organization_archive::{
    OrganizationArchiveInspection, OrganizationArchivePhase, OrganizationArchiveProgress,
    OrganizationArchiveReceipt,
};
use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::{SqliteCatalog, database_error};

mod association;
mod codec;
mod records;
mod snapshot;
mod staging;

impl OrganizationArchiveRepository for SqliteCatalog {
    fn inspect_organization_archive(
        &self,
        archive: &Path,
        progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    ) -> Result<OrganizationArchiveInspection, ScanError> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| error("staging", "Missing catalog directory"))?;
        let stage = staging::read(archive, parent, progress)?;
        let associations = association::inspect(&self.path, &stage, progress)?;
        stage.inspect(associations)
    }

    fn export_organization_archive(
        &self,
        destination: &Path,
        progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    ) -> Result<OrganizationArchiveReceipt, ScanError> {
        if destination.exists() {
            return Err(error(
                "destination_exists",
                "The backup destination already exists",
            ));
        }
        let parent = destination
            .parent()
            .filter(|parent| parent.is_dir())
            .ok_or_else(|| {
                error(
                    "destination_invalid",
                    "The backup destination directory is unavailable",
                )
            })?;
        report(progress, OrganizationArchivePhase::Snapshot, 0)?;
        let snapshot = snapshot::capture(&self.connection, &self.path, progress)?;
        let (archive_id, schema, revision): (String, i64, i64) = snapshot.connection.query_row(
            "SELECT lower(hex(randomblob(16))), schema_info.version, organization_state.revision
             FROM schema_info CROSS JOIN organization_state",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(database_error)?;
        let header = codec::Header {
            format: codec::FORMAT.into(),
            version: codec::VERSION,
            archive_id: archive_id.clone(),
            created_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|cause| error("clock", cause.to_string()))?
                .as_millis()
                .try_into()
                .map_err(|_| error("clock", "The export timestamp is out of range"))?,
            source_schema: schema,
            organization_revision: revision,
        };
        let mut output = tempfile::Builder::new()
            .prefix(".ame-organization-")
            .tempfile_in(parent)
            .map_err(io_error)?;
        let mut counts = vec![0_u64; records::KINDS.len()];
        {
            let mut writer =
                codec::ArchiveWriter::new(BufWriter::new(output.as_file_mut()), &header)?;
            let mut total = 0_u64;
            for (kind_index, kind) in records::KINDS.iter().enumerate() {
                report(progress, OrganizationArchivePhase::Writing, total)?;
                let mut statement = snapshot
                    .connection
                    .prepare(kind.query)
                    .map_err(database_error)?;
                let mut rows = statement.query([]).map_err(database_error)?;
                while let Some(row) = rows.next().map_err(database_error)? {
                    let mut values = Vec::with_capacity(kind.fields.len());
                    for column in 0..kind.fields.len() {
                        values.push(match row.get_ref(column).map_err(database_error)? {
                            ValueRef::Null => codec::Cell::Null,
                            ValueRef::Integer(value) => codec::Cell::Integer(value),
                            ValueRef::Text(value) => {
                                if value.len() >= codec::MAX_LINE_BYTES {
                                    return Err(error(
                                        "record_too_large",
                                        "An archive field exceeds the supported size",
                                    ));
                                }
                                codec::Cell::Text(
                                    std::str::from_utf8(value)
                                        .map_err(|cause| error("encoding", cause.to_string()))?
                                        .to_owned(),
                                )
                            }
                            _ => {
                                return Err(error(
                                    "record_invalid",
                                    "Unsupported archive field type",
                                ));
                            }
                        });
                    }
                    writer.row(kind_index, &values)?;
                    counts[kind_index] += 1;
                    total += 1;
                    if total.is_multiple_of(256) {
                        report(progress, OrganizationArchivePhase::Writing, total)?;
                    }
                }
            }
            writer.finish(&counts)?.flush().map_err(io_error)?;
        }
        drop(snapshot);
        output.as_file().sync_all().map_err(io_error)?;
        report(progress, OrganizationArchivePhase::Verifying, 0)?;
        let verified = codec::validate(
            BufReader::new(File::open(output.path()).map_err(io_error)?),
            &mut |count| report(progress, OrganizationArchivePhase::Verifying, count),
        )?;
        let byte_length = output.as_file().metadata().map_err(io_error)?.len();
        report(
            progress,
            OrganizationArchivePhase::Verifying,
            verified.record_count,
        )?;
        output
            .persist_noclobber(destination)
            .map_err(|cause| io_error(cause.error))?;
        Ok(OrganizationArchiveReceipt {
            archive_id,
            record_count: verified.record_count,
            byte_length,
        })
    }
}

fn report(
    progress: &mut dyn FnMut(OrganizationArchiveProgress) -> bool,
    phase: OrganizationArchivePhase,
    completed_units: u64,
) -> Result<(), ScanError> {
    if progress(OrganizationArchiveProgress {
        phase,
        completed_units,
    }) {
        Ok(())
    } else {
        Err(error(
            "cancelled",
            "The organization archive operation was cancelled",
        ))
    }
}

fn io_error(cause: std::io::Error) -> ScanError {
    error(
        "io",
        format!("Could not read or write the organization archive: {cause}"),
    )
}

fn error(code: &str, message: impl Into<String>) -> ScanError {
    ScanError::new(format!("organization_archive_{code}"), message)
}

#[cfg(test)]
mod tests;
