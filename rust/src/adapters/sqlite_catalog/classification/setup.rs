use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;
use crate::domain::classification::work::{
    ClassificationDispatchCommand, ClassificationDispatchState,
};
use crate::domain::classification::{
    ClassificationAnalysisSetup, ClassificationRecordId, ClassificationSetupActivation,
};
use crate::ports::classification_setup::ClassificationSetupRepository;

use super::super::{SqliteCatalog, database_error};
use super::{changed, codec, configuration, invalid_record, predictions, work::control};

mod profile;

impl ClassificationSetupRepository for SqliteCatalog {
    fn load_classification_setup(
        &self,
        run_id: &ClassificationRecordId,
    ) -> Result<Option<ClassificationAnalysisSetup>, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let setup = load(&transaction, run_id.as_str())?;
        transaction.commit().map_err(database_error)?;
        Ok(setup)
    }

    fn activate_classification_setup(
        &mut self,
        expected_catalog_revision: u64,
        expected_dispatch_revision: u64,
        setup: &ClassificationAnalysisSetup,
        activation: ClassificationSetupActivation,
    ) -> Result<ClassificationDispatchState, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let state = activate(
            &transaction,
            expected_catalog_revision,
            expected_dispatch_revision,
            setup,
            activation,
        )?;
        transaction.commit().map_err(database_error)?;
        Ok(state)
    }
}

pub(super) fn activate(
    connection: &Connection,
    expected_catalog_revision: u64,
    expected_dispatch_revision: u64,
    setup: &ClassificationAnalysisSetup,
    activation: ClassificationSetupActivation,
) -> Result<ClassificationDispatchState, ScanError> {
    let state = configuration::load(connection)?;
    let dispatch = control::load(connection)?.state;
    let run = setup.run();
    let payload = setup
        .refinement()
        .map(|(_, profile)| profile::save(profile))
        .transpose()?;
    let previous = stored_profile(connection, run.id().as_str())?;
    if let Some(previous) = previous {
        if previous != payload
            || predictions::load_run(connection, run.id().as_str())? != *run
            || state.active_run_id.as_deref() != Some(run.id().as_str())
            || state.configuration != Some(run.configuration())
            || state.taxonomy.as_ref().map(|rules| rules.revision()) != Some(run.primary_rules())
            || dispatch.run_id.as_ref() != Some(run.id())
        {
            return Err(changed());
        }
        // Retrying activation must preserve a pause, disable or stop committed afterward.
        return Ok(dispatch);
    }
    if state.revision != expected_catalog_revision
        || dispatch.revision != expected_dispatch_revision
    {
        return Err(changed());
    }
    predictions::begin(connection, run, setup.refinement().map(|(rules, _)| rules))?;
    connection
        .execute(
            "INSERT INTO classification_analysis_setups(run_id, profile_payload) VALUES (?1, ?2)",
            params![run.id().as_str(), payload],
        )
        .map_err(database_error)?;
    let command = match activation {
        ClassificationSetupActivation::Enable => ClassificationDispatchCommand::Enable {
            run_id: run.id().clone(),
        },
        ClassificationSetupActivation::ReplaceRun => ClassificationDispatchCommand::ReplaceRun {
            run_id: run.id().clone(),
        },
    };
    control::change(connection, expected_dispatch_revision, &command)
}

pub(super) fn load(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<ClassificationAnalysisSetup>, ScanError> {
    let Some(payload) = stored_profile(connection, run_id)? else {
        return Ok(None);
    };
    let run = predictions::load_run(connection, run_id)?;
    let primary = configuration::load_rules(connection, &codec::revision(run.primary_rules()))?;
    let refinement = match (run.refinement(), payload) {
        (None, None) => None,
        (Some(identity), Some(payload)) => Some((
            configuration::load_rules(connection, &codec::revision(identity.rules))?,
            profile::load(&payload, &run)?,
        )),
        _ => return Err(invalid_record()),
    };
    ClassificationAnalysisSetup::new(run, primary, refinement)
        .map(Some)
        .map_err(|_| invalid_record())
}

fn stored_profile(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<Option<String>>, ScanError> {
    connection
        .query_row(
            "SELECT profile_payload FROM classification_analysis_setups WHERE run_id = ?1",
            [run_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(database_error)
}

pub(in crate::adapters::sqlite_catalog) fn validate_active(
    connection: &Connection,
) -> Result<(), ScanError> {
    let run: Option<String> = connection
        .query_row(
            "SELECT run_id FROM classification_dispatch_state WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if let Some(run) = run {
        load(connection, &run)?;
    }
    Ok(())
}
