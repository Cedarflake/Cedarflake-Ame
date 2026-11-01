use crate::domain::classification_catalog::PreparedClassificationSelection;
use crate::ports::classification_review::ClassificationReviewRepository;

use super::*;

impl SqliteClassificationSelection {
    pub(super) fn apply_review_inner(
        &mut self,
        id: &ClassificationRecordId,
        session: &ClassificationReviewSession,
        action: &ClassificationAction,
    ) -> Result<ClassificationReviewReceipt, ScanError> {
        self.control.check()?;
        if self
            .prepared
            .as_ref()
            .is_none_or(|value| value.summary.group_count != 1)
        {
            return Err(changed());
        }
        let group = staging::groups(&self.catalog.connection, 0)?
            .into_iter()
            .next()
            .ok_or_else(changed)?;
        let target = group.target.ok_or_else(changed)?;
        let payload: String = self.catalog.connection.query_row(
            "SELECT observation FROM temp.classification_selection_observations WHERE sequence = ?1",
            [group.sequence], |row| row.get(0),
        ).map_err(database_error)?;
        let prepared = PreparedClassificationSelection {
            selections: vec![target.representative],
            groups: vec![codec::observation::load(&payload)?],
        };
        self.control
            .report(ClassificationSelectionPhase::Saving, 0, 1);
        self.control.check()?;
        self.catalog
            .change_current_classification_review(id, session, &prepared, action)
    }
}
