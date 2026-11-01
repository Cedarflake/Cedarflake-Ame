use rusqlite::params;

use crate::domain::ScanError;
use crate::domain::classification::work::{ClassificationDispatchMode, ClassificationRetryOutcome};
use crate::domain::organization::OrganizationSelection;
use crate::ports::classification_work::ClassificationRetryRepository;

use super::super::super::{
    SqliteCatalog, database_error, exact_duplicates::read_unsigned, sqlite_integer,
};
use super::super::codec;
use super::{changed, control, queue, sources};

impl ClassificationRetryRepository for SqliteCatalog {
    fn retry_failed_classification(
        &mut self,
        expected_dispatch_revision: u64,
        target: &OrganizationSelection,
        now_unix_ms: u64,
    ) -> Result<ClassificationRetryOutcome, ScanError> {
        let transaction = self.begin_user_interactive_write()?;
        let dispatch = control::load(&transaction)?;
        if dispatch.state.revision != expected_dispatch_revision {
            return Err(changed());
        }
        if !matches!(
            dispatch.state.mode,
            ClassificationDispatchMode::Automatic | ClassificationDispatchMode::Paused
        ) {
            return Err(ScanError::new(
                "classification_retry_unavailable",
                "Enable analysis before retrying failed work",
            ));
        }
        let run = control::run(&transaction, &dispatch)?;
        queue::requested_sources(&transaction, std::slice::from_ref(target))?;
        let content = queue::verified_content(&transaction, target)?;
        let work = {
            let mut statement = transaction
                .prepare(
                    "SELECT work_id FROM classification_work_items
                 WHERE configuration_revision = ?1 AND kind = 'source'
                   AND source_generation = ?2 AND phase = 'failed'
                 UNION ALL
                 SELECT work_id FROM classification_work_items
                 WHERE configuration_revision = ?1 AND kind = 'content'
                   AND fingerprint_scheme = ?3 AND digest = ?4 AND byte_length = ?5
                   AND phase = 'failed'",
                )
                .map_err(database_error)?;
            statement
                .query_map(
                    params![
                        codec::revision(run.configuration()),
                        sqlite_integer(target.source_generation, "source generation")?,
                        content.as_ref().map(|value| value.scheme.as_str()),
                        content.as_ref().map(|value| codec::digest(&value.digest)),
                        content
                            .as_ref()
                            .map(|value| sqlite_integer(value.byte_length, "content length"))
                            .transpose()?,
                    ],
                    |row| read_unsigned(row, 0),
                )
                .map_err(database_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(database_error)?
        };
        for id in &work {
            sources::clear(&transaction, *id)?;
            transaction
                .execute(
                    "UPDATE classification_work_items
                 SET phase = 'queued', failure_count = 0, issue_code = NULL,
                     ready_unix_ms = ?1, priority = 0
                 WHERE work_id = ?2",
                    params![
                        sqlite_integer(now_unix_ms, "retry time")?,
                        sqlite_integer(*id, "work identity")?
                    ],
                )
                .map_err(database_error)?;
        }
        let outcome = if work.is_empty() {
            ClassificationRetryOutcome::Unchanged
        } else {
            transaction
                .execute("UPDATE catalog_state SET revision = revision + 1", [])
                .map_err(database_error)?;
            if dispatch.state.mode == ClassificationDispatchMode::Paused {
                ClassificationRetryOutcome::QueuedWhilePaused
            } else {
                ClassificationRetryOutcome::Queued
            }
        };
        transaction.commit().map_err(database_error)?;
        Ok(outcome)
    }
}
