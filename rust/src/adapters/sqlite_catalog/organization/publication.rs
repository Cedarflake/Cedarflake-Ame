use rusqlite::{Connection, params};

use crate::domain::ScanError;
use crate::domain::exact_duplicates::{EXACT_FINGERPRINT_SCHEME, EXACT_PUBLICATION_BATCH_SIZE};

use super::super::exact_duplicates::queries::{COMPATIBLE_FINGERPRINT, CURRENT_LOCATIONS};
use super::super::{database_error, sqlite_integer};
use super::records;

pub(in crate::adapters::sqlite_catalog) fn batch(
    connection: &Connection,
    run_id: &str,
    generations: &[u64],
) -> Result<(), ScanError> {
    let mut published = Vec::with_capacity(generations.len());
    for generation in generations {
        let generation = sqlite_integer(*generation, "source generation")?;
        let accepted: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM content_fingerprints
             WHERE source_generation = ?1 AND scheme = ?2 AND run_id = ?3)",
                params![generation, EXACT_FINGERPRINT_SCHEME, run_id],
                |row| row.get(0),
            )
            .map_err(database_error)?;
        if accepted {
            published.push(generation);
        }
    }
    published_generations(connection, &published)
}

pub(in crate::adapters::sqlite_catalog) fn published_generations(
    connection: &Connection,
    generations: &[i64],
) -> Result<(), ScanError> {
    super::recovery::batch(connection, generations)?;
    if !has_records(connection)? {
        return Ok(());
    }
    for generation in generations {
        records::publish_generation(connection, *generation)?;
    }
    Ok(())
}

pub(in crate::adapters::sqlite_catalog) fn root(
    connection: &Connection,
    root_id: &str,
) -> Result<(), ScanError> {
    super::recovery::root(connection, root_id)?;
    if !has_records(connection)? {
        return Ok(());
    }
    let mut after = 0;
    loop {
        let generations = connection.prepare(&format!(
            "SELECT DISTINCT l.source_generation FROM {CURRENT_LOCATIONS}
             LEFT JOIN content_fingerprints AS f ON {COMPATIBLE_FINGERPRINT}
             WHERE l.root_id = ?2 AND l.source_generation > ?3 AND
               (EXISTS(SELECT 1 FROM organization_bindings AS known
                  WHERE known.fingerprint_scheme = f.scheme AND known.digest = f.digest AND known.byte_length = f.byte_length)
                OR EXISTS(SELECT 1 FROM organization_bindings AS binding
                 WHERE binding.asset_id = l.asset_id AND binding.source_generation = l.source_generation)
                OR EXISTS(SELECT 1 FROM organization_bindings AS binding
                  WHERE binding.file_identity_scheme = l.file_identity_scheme
                    AND binding.file_identity_value = l.file_identity_value
                    AND binding.source_revision_token = l.source_revision_token
                    AND binding.byte_length = l.file_size))
             ORDER BY l.source_generation LIMIT ?4"
        )).map_err(database_error)?.query_map(
            params![EXACT_FINGERPRINT_SCHEME, root_id, after, EXACT_PUBLICATION_BATCH_SIZE as i64],
            |row| row.get::<_, i64>(0),
        ).map_err(database_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(database_error)?;
        let Some(last) = generations.last() else {
            break;
        };
        after = *last;
        for generation in generations {
            records::publish_generation(connection, generation)?;
        }
    }
    super::association_lifetime::refresh_root(connection, root_id)
}

fn has_records(connection: &Connection) -> Result<bool, ScanError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM organization_records)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)
}
