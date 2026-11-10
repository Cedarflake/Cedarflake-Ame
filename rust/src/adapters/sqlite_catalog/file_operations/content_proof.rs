use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::EXACT_FINGERPRINT_SCHEME;
use crate::domain::file_operation_journal::AuthorizedFileOperation;
use crate::domain::file_operations::FileOperationContentProof;

use super::super::exact_duplicates::read_unsigned;
use super::super::{database_error, source_revision_token, sqlite_integer};

pub(super) fn load(
    connection: &Connection,
    operation: &AuthorizedFileOperation,
) -> Result<Option<FileOperationContentProof>, ScanError> {
    let source = &operation.request.source.state;
    let (Some(identity), Some(revision)) = (&source.file_identity, &source.source_revision) else {
        return Ok(None);
    };
    // Both sources must describe the authorized version, never the current destination version.
    let mut statement = connection
        .prepare(
            "SELECT scheme, digest, byte_length FROM content_fingerprints
         WHERE source_generation = ?1 AND byte_length = ?2
           AND file_identity_scheme = ?3 AND file_identity_value = ?4
           AND source_revision_token = ?5 AND scheme = ?6
         UNION
         SELECT fingerprint_scheme, digest, byte_length FROM organization_bindings
         WHERE source_generation = ?1 AND byte_length = ?2
           AND file_identity_scheme = ?3 AND file_identity_value = ?4
           AND source_revision_token = ?5 AND fingerprint_scheme = ?6
           AND asset_id = ?7 AND digest IS NOT NULL
         LIMIT 2",
        )
        .map_err(database_error)?;
    let proofs = statement
        .query_map(
            params![
                sqlite_integer(operation.catalog.source_generation, "original generation")?,
                sqlite_integer(source.file_size, "original size")?,
                identity.scheme,
                identity.value,
                source_revision_token(revision)?,
                EXACT_FINGERPRINT_SCHEME,
                operation.catalog.asset_id,
            ],
            |row| {
                Ok(FileOperationContentProof {
                    scheme: row.get(0)?,
                    digest: row.get(1)?,
                    byte_length: read_unsigned(row, 2)?,
                })
            },
        )
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    // Conflicting persisted evidence cannot establish content continuity.
    Ok(if proofs.len() == 1 {
        proofs.into_iter().next()
    } else {
        None
    })
}
