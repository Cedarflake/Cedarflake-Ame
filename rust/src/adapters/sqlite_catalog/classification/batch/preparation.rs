use crate::domain::classification::ClassificationReview;

use super::super::super::exact_duplicates::read_unsigned;
use super::super::selection;
use super::*;

impl SqliteClassificationSelection {
    pub(super) fn prepare_inner(&mut self) -> Result<ClassificationSelectionSummary, ScanError> {
        self.control.check()?;
        if let Some(prepared) = &self.prepared {
            return Ok(prepared.summary.clone());
        }
        self.control
            .report(ClassificationSelectionPhase::Preparing, 0, 0);
        let transaction = self
            .catalog
            .connection
            .unchecked_transaction()
            .map_err(database_error)?;
        let revision = transaction
            .query_row("SELECT revision FROM catalog_state", [], |row| {
                read_unsigned(row, 0)
            })
            .map_err(database_error)?;
        if revision != self.catalog_revision {
            return Err(changed());
        }
        staging::materialize(&transaction, &self.query, self.all_matching)?;
        let group_count = transaction
            .query_row(
                "SELECT COUNT(*) FROM temp.organization_batch_groups",
                [],
                |row| read_unsigned(row, 0),
            )
            .map_err(database_error)?;
        if group_count == 0 {
            return Err(ScanError::new(
                "classification_selection_empty",
                "No current image groups are selected",
            ));
        }
        let state = configuration::load(&transaction)?;
        let taxonomy = state.taxonomy.as_ref().ok_or_else(|| {
            ScanError::new(
                "classification_not_configured",
                "Choose category definitions before classifying images",
            )
        })?;
        let mut summary = ClassificationSelectionSummary {
            group_count,
            ..Default::default()
        };
        let mut hasher = blake3::Hasher::new_derive_key("ame.classification.selection.v1");
        let mut after = 0;
        let mut completed = 0;
        loop {
            let groups = staging::groups(&transaction, after)?;
            if groups.is_empty() {
                break;
            }
            for group in groups {
                self.control.check()?;
                let target = organization::selection::target(&transaction, &group.key)?;
                let observation = observations::for_target(&transaction, &target, &state)?;
                if group_count == 1 {
                    summary.single_item = Some(selection::item_state(
                        &target.representative.asset_id,
                        &observation,
                        taxonomy,
                    ));
                }
                let effective = observation.effective();
                let category = effective
                    .category
                    .map(|category| category.as_str().to_owned());
                if completed == 0 {
                    summary.common_category_id = category;
                } else if summary.common_category_id != category {
                    summary.common_category_id = None;
                }
                summary.confirmed_count +=
                    u64::from(effective.review == ClassificationReview::Confirmed);
                let confirmable = selection::can_confirm(&observation, taxonomy);
                summary.confirmable_count += u64::from(confirmable);
                let payload = codec::observation::save(&observation)?;
                operations::field(&mut hasher, payload.as_bytes());
                staging::retain_target(&transaction, group.sequence, &target)?;
                transaction.execute("INSERT INTO temp.classification_selection_observations VALUES (?1, ?2, ?3)",
                    params![group.sequence, payload, confirmable]).map_err(database_error)?;
                after = group.sequence;
                completed += 1;
                self.control.report(
                    ClassificationSelectionPhase::Preparing,
                    completed,
                    group_count,
                );
            }
        }
        self.control.check()?;
        transaction.commit().map_err(database_error)?;
        self.prepared = Some(PreparedSelection {
            summary: summary.clone(),
            digest: hasher.finalize().to_hex().to_string(),
        });
        self.control.report(
            ClassificationSelectionPhase::Ready,
            group_count,
            group_count,
        );
        Ok(summary)
    }
}
