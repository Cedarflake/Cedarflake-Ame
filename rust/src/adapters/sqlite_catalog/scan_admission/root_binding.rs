use rusqlite::Connection;

use super::{
    FileIdentityEvidence, IncrementalCatalogRoot, ScanError, Transaction, database_error, params,
    retire_root_change_queue, sqlite_integer,
};

pub(in crate::adapters::sqlite_catalog) enum RootScanBinding<'a> {
    Registered,
    Relocated(&'a IncrementalCatalogRoot),
    IdentityRecovered {
        expected: &'a IncrementalCatalogRoot,
        identity: &'a FileIdentityEvidence,
    },
}

impl RootScanBinding<'_> {
    pub(in crate::adapters::sqlite_catalog) fn validate_and_retire(
        &self,
        transaction: &Transaction<'_>,
        root_id: &str,
        root_path: &str,
        now: i64,
    ) -> Result<(), ScanError> {
        self.validate(transaction, root_id, root_path)?;
        let expected = match self {
            Self::Registered => return Ok(()),
            Self::Relocated(expected) | Self::IdentityRecovered { expected, .. } => expected,
        };
        if expected.root_path != root_path {
            // Workers bound to the old namespace cannot enter the replacement generation.
            retire_root_change_queue(transaction, root_id, now)?;
        }
        Ok(())
    }

    pub(in crate::adapters::sqlite_catalog) fn validate(
        &self,
        transaction: &Transaction<'_>,
        root_id: &str,
        root_path: &str,
    ) -> Result<(), ScanError> {
        let expected = match self {
            Self::Registered => return require_registered_path(transaction, root_id, root_path),
            Self::Relocated(expected) => expected,
            Self::IdentityRecovered { expected, identity } => {
                if expected.publication_root_identity.as_ref() != Some(identity)
                    || unique_published_root_id(transaction, identity)?.as_deref() != Some(root_id)
                {
                    return Err(ScanError::new(
                        "catalog_root_identity_changed",
                        "The directory no longer has one matching published library root",
                    ));
                }
                expected
            }
        };
        let is_current = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM library_roots AS root
                   JOIN library_change_root_state AS state ON state.root_id = root.id
                   WHERE root.id = ?1 AND root.path = ?2
                     AND state.generation = ?3 AND state.is_active = 1
                 )",
                params![
                    root_id,
                    expected.root_path,
                    sqlite_integer(expected.root_generation.value(), "root generation")?
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if !is_current {
            return Err(ScanError::new(
                "catalog_root_relocation_stale",
                "The selected library root changed before relocation started",
            ));
        }
        let path_is_registered = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM library_roots WHERE path = ?1 AND id <> ?2)",
                params![root_path, root_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(database_error)?;
        if path_is_registered {
            return Err(ScanError::new(
                "catalog_root_relocation_conflict",
                "The selected directory already belongs to another library root",
            ));
        }
        Ok(())
    }
}

fn require_registered_path(
    transaction: &Transaction<'_>,
    root_id: &str,
    root_path: &str,
) -> Result<(), ScanError> {
    let path_changed = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_roots WHERE id = ?1 AND path <> ?2)",
            params![root_id, root_path],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if path_changed {
        return Err(ScanError::new(
            "catalog_root_path_changed",
            "A registered root path requires explicit replacement or proven identity recovery",
        ));
    }
    Ok(())
}

pub(super) fn unique_published_root_id(
    connection: &Connection,
    identity: &FileIdentityEvidence,
) -> Result<Option<String>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT roots.id
             FROM library_roots AS roots
             JOIN library_change_root_state AS state ON state.root_id = roots.id
             JOIN library_root_publication_namespaces AS namespace
               ON namespace.root_id = roots.id AND namespace.root_generation = state.generation
             WHERE state.is_active = 1 AND roots.active_scan_id IS NOT NULL
               AND namespace.identity_scheme = ?1 AND namespace.identity_value = ?2
             LIMIT 2",
        )
        .map_err(database_error)?;
    let mut matches = statement
        .query_map(params![identity.scheme, identity.value], |row| {
            row.get::<_, String>(0)
        })
        .map_err(database_error)?;
    let first = matches.next().transpose().map_err(database_error)?;
    if matches
        .next()
        .transpose()
        .map_err(database_error)?
        .is_some()
    {
        return Err(ScanError::new(
            "catalog_root_identity_ambiguous",
            "Multiple published library roots claim this directory identity",
        ));
    }
    Ok(first)
}
