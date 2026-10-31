use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::{
    ClassificationContent, ClassificationRevision, ClassificationRun, ClassificationSubject,
    PredictionEvidence, Taxonomy,
};
use crate::domain::organization::OrganizationSelection;

use super::super::{database_error, organization, sqlite_integer};
use super::{changed, codec, configuration, invalid_record, observations, operations};

pub(super) fn begin(
    connection: &Connection,
    run: &ClassificationRun,
    refinement: Option<&Taxonomy>,
) -> Result<(), ScanError> {
    let state = configuration::load(connection)?;
    let primary = state.taxonomy.ok_or_else(changed)?;
    if primary.revision() != run.primary_rules() {
        return Err(changed());
    }
    let payload = codec::encode_run(run)?;
    let previous: Option<String> = connection
        .query_row(
            "SELECT payload FROM classification_runs WHERE run_id = ?1",
            [run.id().as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(previous) = previous {
        if previous != payload || state.active_run_id.as_deref() != Some(run.id().as_str()) {
            return Err(changed());
        }
        return Ok(());
    }
    let revision = operations::next_revision(connection)?;
    match (run.refinement(), refinement) {
        (Some(identity), Some(taxonomy))
            if identity.rules == taxonomy.revision()
                && category_ids(taxonomy) == category_ids(&primary) =>
        {
            configuration::save_rules(connection, taxonomy, revision, false)?;
        }
        (None, None) => {}
        _ => return Err(invalid_record()),
    }
    connection.execute(
        "INSERT INTO classification_runs(run_id, primary_rules, configuration_revision, payload) VALUES (?1, ?2, ?3, ?4)",
        params![run.id().as_str(), codec::revision(run.primary_rules()), codec::revision(run.configuration()), payload],
    ).map_err(database_error)?;
    connection.execute(
        "UPDATE classification_state SET revision = ?1, active_run_id = ?2, configuration_revision = ?3 WHERE singleton = 1",
        params![revision, run.id().as_str(), codec::revision(run.configuration())],
    ).map_err(database_error)?;
    operations::publish_catalog(connection)
}

pub(super) fn publish(
    connection: &Connection,
    source: &OrganizationSelection,
    evidence: &PredictionEvidence,
) -> Result<bool, ScanError> {
    let payload = codec::prediction::save(evidence)?;
    let existing: Option<String> = connection
        .query_row(
            "SELECT payload FROM classification_predictions WHERE prediction_id = ?1",
            [evidence.id().as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)?;
    if let Some(existing) = existing {
        if existing != payload {
            return Err(invalid_record());
        }
        return Ok(false);
    }
    let state = configuration::load(connection)?;
    let run = evidence.run();
    if state.active_run_id.as_deref() != Some(run.id().as_str())
        || state.configuration != Some(run.configuration())
        || state.taxonomy.as_ref().map(Taxonomy::revision) != Some(run.primary_rules())
    {
        return Ok(false);
    }
    let saved_run = load_run(connection, run.id().as_str())?;
    if saved_run != *run {
        return Err(invalid_record());
    }
    let taxonomy = state.taxonomy.ok_or_else(invalid_record)?;
    let categories: BTreeSet<_> = evidence
        .initial()
        .categories()
        .iter()
        .map(|score| score.category_id.as_str())
        .collect();
    if categories != category_ids(&taxonomy) {
        return Err(invalid_record());
    }
    let selected =
        organization::selection::selected_keys(connection, std::slice::from_ref(source))?;
    if selected.len() != 1
        || selected
            .iter()
            .next()
            .map(observations::subject)
            .transpose()?
            != Some(ClassificationSubject::Exact(evidence.content().clone()))
    {
        return Ok(false);
    }
    let content = evidence.content();
    connection
        .execute(
            "INSERT INTO classification_predictions(prediction_id, run_id, configuration_revision,
         fingerprint_scheme, digest, byte_length, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                evidence.id().as_str(),
                run.id().as_str(),
                codec::revision(run.configuration()),
                content.scheme.as_str(),
                codec::digest(&content.digest),
                sqlite_integer(content.byte_length, "classification content length")?,
                payload
            ],
        )
        .map_err(database_error)?;
    operations::publish_catalog(connection)?;
    Ok(true)
}

pub(super) fn latest(
    connection: &Connection,
    content: &ClassificationContent,
    configuration: ClassificationRevision,
) -> Result<Option<PredictionEvidence>, ScanError> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT prediction.payload, run.payload FROM classification_predictions AS prediction
         JOIN classification_runs AS run USING(run_id)
         WHERE prediction.fingerprint_scheme = ?1 AND prediction.digest = ?2
           AND prediction.byte_length = ?3 AND prediction.configuration_revision = ?4
         ORDER BY prediction.publication_id DESC LIMIT 1",
            params![
                content.scheme.as_str(),
                codec::digest(&content.digest),
                sqlite_integer(content.byte_length, "classification content length")?,
                codec::revision(configuration)
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(database_error)?;
    row.map(|(payload, run)| {
        let evidence = codec::prediction::load(&payload, codec::decode_run(&run)?)?;
        if evidence.content() != content || evidence.run().configuration() != configuration {
            return Err(invalid_record());
        }
        Ok(evidence)
    })
    .transpose()
}

pub(super) fn load_run(connection: &Connection, id: &str) -> Result<ClassificationRun, ScanError> {
    let payload: String = connection
        .query_row(
            "SELECT payload FROM classification_runs WHERE run_id = ?1",
            [id],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    let run = codec::decode_run(&payload)?;
    if run.id().as_str() != id {
        return Err(invalid_record());
    }
    Ok(run)
}

fn category_ids(taxonomy: &Taxonomy) -> BTreeSet<&str> {
    taxonomy
        .categories()
        .iter()
        .map(|category| category.id().as_str())
        .collect()
}
