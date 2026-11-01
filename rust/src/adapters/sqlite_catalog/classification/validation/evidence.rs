use std::collections::BTreeSet;

use rusqlite::{Connection, Row};

use crate::domain::ScanError;
use crate::domain::classification::{CategoryId, ClassificationRun};

use super::super::super::exact_duplicates::read_unsigned;
use super::{codec, configuration, database_error, invalid_record};

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    validate_runs(connection)?;
    let mut statement = connection.prepare(
        "SELECT prediction.prediction_id, prediction.configuration_revision, fingerprint_scheme, digest, byte_length,
                prediction.payload, run.run_id, run.payload FROM classification_predictions AS prediction
         JOIN classification_runs AS run USING(run_id)",
    ).map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    let mut previous: Option<PredictionRun> = None;
    while let Some(row) = rows.next().map_err(database_error)? {
        let run_id: String = row.get(6).map_err(database_error)?;
        // Reuse only within this validation snapshot, with one bounded run/rule set.
        let current = match previous.take() {
            Some(current) if current.run.id().as_str() == run_id => current,
            _ => PredictionRun::load(connection, row)?,
        };
        current.validate(row)?;
        previous = Some(current);
    }
    Ok(())
}

struct PredictionRun {
    run: ClassificationRun,
    categories: BTreeSet<CategoryId>,
}

impl PredictionRun {
    fn load(connection: &Connection, row: &Row<'_>) -> Result<Self, ScanError> {
        let run = codec::decode_run(&row.get::<_, String>(7).map_err(database_error)?)?;
        let taxonomy =
            configuration::load_rules(connection, &codec::revision(run.primary_rules()))?;
        let categories = taxonomy
            .categories()
            .iter()
            .map(|category| category.id().clone())
            .collect();
        Ok(Self { run, categories })
    }

    fn validate(&self, row: &Row<'_>) -> Result<(), ScanError> {
        let evidence = codec::prediction::load(
            &row.get::<_, String>(5).map_err(database_error)?,
            self.run.clone(),
        )?;
        let content = evidence.content();
        let actual: BTreeSet<_> = evidence
            .initial()
            .categories()
            .iter()
            .map(|value| &value.category_id)
            .collect();
        if !self.categories.iter().eq(actual)
            || evidence.id().as_str() != row.get::<_, String>(0).map_err(database_error)?
            || codec::revision(evidence.run().configuration())
                != row.get::<_, String>(1).map_err(database_error)?
            || content.scheme.as_str() != row.get::<_, String>(2).map_err(database_error)?
            || codec::digest(&content.digest) != row.get::<_, String>(3).map_err(database_error)?
            || content.byte_length != read_unsigned(row, 4).map_err(database_error)?
        {
            return Err(invalid_record());
        }
        Ok(())
    }
}

fn validate_runs(connection: &Connection) -> Result<(), ScanError> {
    let mut statement = connection.prepare("SELECT run_id, primary_rules, configuration_revision, payload FROM classification_runs").map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let run = codec::decode_run(&row.get::<_, String>(3).map_err(database_error)?)?;
        if run.id().as_str() != row.get::<_, String>(0).map_err(database_error)?
            || codec::revision(run.primary_rules())
                != row.get::<_, String>(1).map_err(database_error)?
            || codec::revision(run.configuration())
                != row.get::<_, String>(2).map_err(database_error)?
        {
            return Err(invalid_record());
        }
        if let Some(refinement) = run.refinement() {
            configuration::load_rules(connection, &codec::revision(refinement.rules))?;
        }
    }
    Ok(())
}
