use crate::domain::ScanError;
use crate::domain::classification::CategoryDefinition;
use crate::domain::classification_configuration::{
    ClassificationConfigurationSnapshot, ClassificationTermChange, ClassificationTermsReset,
};
use crate::ports::classification_configuration::{
    ClassificationConfigurationRepository, ClassificationTermRepository,
};

use super::{SqliteCatalog, codec, database_error, operations};

impl ClassificationTermRepository for SqliteCatalog {
    fn save_classification_term(
        &mut self,
        change: &ClassificationTermChange,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let operation = change.baseline.run().id();
        let digest = request_digest(change)?;
        if operations::replay(&transaction, operation, &digest)?.is_none() {
            let before = super::load(&transaction)?;
            let current_setup = before
                .catalog
                .active_run_id
                .as_deref()
                .map(|run_id| {
                    super::super::setup::load(&transaction, run_id)?
                        .ok_or_else(super::super::invalid_record)
                })
                .transpose()?;
            let configuration = change.configuration(&before, current_setup.as_ref())?;
            super::apply(&transaction, &configuration, &digest)?;
            super::super::maintenance::protect_manual_changes(
                &transaction,
                before.catalog.taxonomy.as_ref(),
                configuration.setup.primary(),
            )?;
        }
        let snapshot = super::load(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(snapshot)
    }

    fn reset_all_classification_terms(
        &mut self,
        change: &ClassificationTermsReset,
    ) -> Result<ClassificationConfigurationSnapshot, ScanError> {
        if change.definitions().next().is_none() {
            return self.load_classification_configuration();
        }
        let transaction = self.begin_user_interactive_write()?;
        let operation = change.baseline().run().id();
        let digest = reset_digest(change)?;
        if operations::replay(&transaction, operation, &digest)?.is_none() {
            let before = super::load(&transaction)?;
            let current_setup = before
                .catalog
                .active_run_id
                .as_deref()
                .map(|run_id| {
                    super::super::setup::load(&transaction, run_id)?
                        .ok_or_else(super::super::invalid_record)
                })
                .transpose()?;
            if let Some(configuration) = change.configuration(&before, current_setup.as_ref())? {
                super::apply(&transaction, &configuration, &digest)?;
                super::super::maintenance::protect_manual_changes(
                    &transaction,
                    before.catalog.taxonomy.as_ref(),
                    configuration.setup.primary(),
                )?;
            }
        }
        let snapshot = super::load(&transaction)?;
        transaction.commit().map_err(database_error)?;
        Ok(snapshot)
    }
}

fn reset_digest(change: &ClassificationTermsReset) -> Result<String, ScanError> {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.term-reset.v1");
    operations::field(
        &mut hash,
        codec::encode_run(change.baseline().run())?.as_bytes(),
    );
    for (expected, replacement) in change.definitions() {
        definition(&mut hash, expected);
        definition(&mut hash, replacement);
    }
    Ok(hash.finalize().to_hex().to_string())
}

fn request_digest(change: &ClassificationTermChange) -> Result<String, ScanError> {
    let mut hash = blake3::Hasher::new_derive_key("ame.classification.term-edit.v1");
    operations::field(
        &mut hash,
        codec::encode_run(change.baseline.run())?.as_bytes(),
    );
    for category in [
        &change.expected,
        &change.replacement,
        &change.bundled_default,
    ] {
        match category {
            Some(category) => {
                operations::field(&mut hash, b"definition");
                definition(&mut hash, category);
            }
            None => operations::field(&mut hash, b"absent"),
        }
    }
    Ok(hash.finalize().to_hex().to_string())
}

fn definition(hash: &mut blake3::Hasher, category: &CategoryDefinition) {
    operations::field(hash, category.id().as_str().as_bytes());
    operations::field(hash, category.label().as_bytes());
    operations::field(hash, category.semantic_revision().bytes().as_slice());
}
