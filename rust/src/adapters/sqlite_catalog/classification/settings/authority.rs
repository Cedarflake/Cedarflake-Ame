use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::ModelCompatibility;
use crate::domain::classification_configuration::ClassificationConfigurationSnapshot;

pub(super) fn active_model(
    connection: &Connection,
    configuration: &ClassificationConfigurationSnapshot,
) -> Result<Option<ModelCompatibility>, ScanError> {
    let Some(run_id) = configuration.dispatch.run_id.as_ref() else {
        return Ok(None);
    };
    Ok(super::setup::load(connection, run_id.as_str())?
        .filter(|setup| {
            configuration.catalog.active_run_id.as_deref() == Some(run_id.as_str())
                && configuration.catalog.configuration == Some(setup.run().configuration())
                && configuration.catalog.taxonomy.as_ref() == Some(setup.primary())
        })
        .map(|setup| setup.run().model()))
}
