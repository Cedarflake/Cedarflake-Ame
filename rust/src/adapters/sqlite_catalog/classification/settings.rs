use rusqlite::Connection;

use crate::domain::ScanError;
use crate::domain::classification::ClassificationSetupActivation;
use crate::domain::classification::work::ClassificationDispatchMode;
use crate::domain::classification_configuration::{
    ClassificationConfigurationAction, ClassificationConfigurationChange,
    ClassificationConfigurationSnapshot,
};
use crate::ports::classification_configuration::ClassificationConfigurationRepository;

use super::super::{SqliteCatalog, database_error};
use super::{changed, codec, configuration, operations, setup, work::control};

impl ClassificationConfigurationRepository for SqliteCatalog {
    fn load_classification_configuration(
        &self,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let snapshot = load(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(snapshot)
    }

    fn save_classification_configuration(
        &mut self,
        change: &ClassificationConfigurationChange,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        apply(&transaction, change)?;
        let snapshot = load(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(snapshot)
    }
}

fn load(connection: &Connection) -> Result<ClassificationConfigurationSnapshot, ScanError> {
    Ok(ClassificationConfigurationSnapshot {
        catalog: configuration::load(connection)?,
        dispatch: control::load(connection)?.state,
    })
}

fn apply(
    connection: &Connection,
    change: &ClassificationConfigurationChange,
) -> Result<(), ScanError> {
    let digest = request_digest(change)?;
    if operations::replay(connection, &change.operation_id, &digest)?.is_some() {
        return Ok(());
    }
    let before = load(connection)?;
    if before.catalog.revision != change.expected_catalog_revision
        || before.dispatch.revision != change.expected_dispatch_revision
    {
        return Err(changed());
    }
    let activation = match change.action {
        ClassificationConfigurationAction::Enable => {
            if !matches!(
                before.dispatch.mode,
                ClassificationDispatchMode::Disabled | ClassificationDispatchMode::Stopped
            ) {
                return Err(changed());
            }
            Some(ClassificationSetupActivation::Enable)
        }
        ClassificationConfigurationAction::Save => {
            if matches!(
                before.dispatch.mode,
                ClassificationDispatchMode::Automatic | ClassificationDispatchMode::Paused
            ) && before.catalog.configuration != Some(change.setup.run().configuration())
            {
                Some(ClassificationSetupActivation::ReplaceRun)
            } else {
                None
            }
        }
    };
    let revision = operations::begin(
        connection,
        &change.operation_id,
        &digest,
        "configure_analysis",
    )?;
    let metadata_changed = configuration::apply_rules(
        connection,
        &before.catalog,
        change.setup.primary(),
        revision,
    )?;
    let receipt = operations::finish(
        connection,
        &change.operation_id,
        revision,
        0,
        metadata_changed,
    )?;
    if let Some(activation) = activation {
        setup::activate(
            connection,
            receipt.revision,
            before.dispatch.revision,
            &change.setup,
            activation,
        )?;
    }
    Ok(())
}

fn request_digest(change: &ClassificationConfigurationChange) -> Result<String, ScanError> {
    let mut hasher = blake3::Hasher::new_derive_key("ame.classification.settings.v1");
    operations::field(&mut hasher, &change.expected_catalog_revision.to_le_bytes());
    operations::field(
        &mut hasher,
        &change.expected_dispatch_revision.to_le_bytes(),
    );
    operations::field(
        &mut hasher,
        match change.action {
            ClassificationConfigurationAction::Save => b"save",
            ClassificationConfigurationAction::Enable => b"enable",
        },
    );
    operations::field(
        &mut hasher,
        codec::encode_run(change.setup.run())?.as_bytes(),
    );
    for category in change.setup.primary().categories() {
        operations::field(&mut hasher, category.id().as_str().as_bytes());
        operations::field(&mut hasher, category.label().as_bytes());
    }
    Ok(hasher.finalize().to_hex().to_string())
}
