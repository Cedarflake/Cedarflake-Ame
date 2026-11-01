use super::*;

impl SqliteClassificationSelection {
    pub(super) fn apply_inner(
        &mut self,
        id: &ClassificationRecordId,
        action: &ClassificationAction,
    ) -> Result<ClassificationWriteReceipt, ScanError> {
        self.control.check()?;
        let prepared = self.prepared.as_ref().ok_or_else(changed)?;
        let mut hasher = blake3::Hasher::new_derive_key("ame.classification.selection.request.v1");
        operations::field(&mut hasher, prepared.digest.as_bytes());
        let kind = match action {
            ClassificationAction::Confirm => "confirm",
            ClassificationAction::Correct(category) => {
                operations::field(&mut hasher, category.as_str().as_bytes());
                "correct"
            }
            ClassificationAction::Defer => "defer",
        };
        operations::field(&mut hasher, kind.as_bytes());
        let digest = hasher.finalize().to_hex().to_string();
        let transaction = self.catalog.begin_user_interactive_write()?;
        if let Some(receipt) = operations::replay(&transaction, id, &digest)? {
            transaction.commit().map_err(database_error)?;
            return Ok(receipt);
        }
        let state = configuration::load(&transaction)?;
        let taxonomy = state.taxonomy.as_ref().ok_or_else(changed)?;
        let revision = operations::begin(
            &transaction,
            id,
            &digest,
            if kind == "defer" { "defer" } else { "category" },
        )?;
        let total = prepared.summary.group_count;
        self.control
            .report(ClassificationSelectionPhase::Saving, 0, total);
        let mut after = 0;
        let mut completed = 0;
        let mut changed_groups = 0;
        loop {
            let groups = staging::groups(&transaction, after)?;
            if groups.is_empty() {
                break;
            }
            for group in groups {
                self.control.check()?;
                let (payload, confirmable): (String, bool) = transaction.query_row(
                    "SELECT observation, confirmable FROM temp.classification_selection_observations WHERE sequence = ?1",
                    [group.sequence], |row| Ok((row.get(0)?, row.get(1)?)),
                ).map_err(database_error)?;
                let observed = codec::observation::load(&payload)?;
                let target = organization::selection::target(&transaction, &group.key)?;
                if group.target.as_ref() != Some(&target) {
                    return Err(changed());
                }
                let current = observations::for_target(&transaction, &target, &state)?;
                if !matches!(action, ClassificationAction::Confirm) || confirmable {
                    let plan = plan_classification_change(&observed, &current, action, taxonomy)
                        .map_err(|error| {
                            ScanError::new("classification_changed", error.to_string())
                        })?;
                    changed_groups += u64::from(writes::apply_group(
                        &transaction,
                        id,
                        revision,
                        &current,
                        plan,
                    )?);
                }
                after = group.sequence;
                completed += 1;
                self.control
                    .report(ClassificationSelectionPhase::Saving, completed, total);
            }
        }
        self.control.check()?;
        let receipt = operations::finish(&transaction, id, revision, changed_groups, false)?;
        self.control.check()?;
        transaction.commit().map_err(database_error)?;
        Ok(receipt)
    }
}
