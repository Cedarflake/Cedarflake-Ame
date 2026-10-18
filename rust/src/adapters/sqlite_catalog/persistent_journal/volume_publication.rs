use rusqlite::params;

use crate::domain::{
    LibraryChangeCatchUpEvidence, LibraryChangeQueuePolicy, PersistentJournalContinuityState,
    PersistentJournalEnrollmentReport, PersistentJournalRangeState, PersistentJournalVolumeBatch,
    ScanError, persistent_journal_batch_id,
};

use super::super::change_queue::{
    PERSISTENT_JOURNAL_CATCH_UP_SOURCE, cleanup_for_enqueue,
    consume_live_gap_claims_with_source_range, enqueue_intents_in_transaction,
    normalize_persistent_journal_intents, validate_catch_up_evidence, validate_enqueue_batch,
};

use super::super::{SqliteCatalog, database_error};

use super::{
    baseline_window, finalize_persistent_journal_ranges, invalid_batch, load_checkpoint,
    load_checkpoint_range, map_queue_backpressure, publication_admission,
};

mod lineage;
mod pending_renames;
mod source_ranges;

pub(super) use lineage::enroll_cross_root_lineage;
use lineage::validate_atomic_lineage;
use pending_renames::{insert_pending_rename, validate_pending_rename_mutations};
use source_ranges::{
    apply_checkpoint_advance, reject_conflicting_range_coordinates, validate_checkpoint_advance,
};
pub(super) use source_ranges::{insert_source_range, require_supported_root, source_range_exists};

pub(super) fn publish_volume_batch(
    catalog: &mut SqliteCatalog,
    batch: &PersistentJournalVolumeBatch,
    enqueued_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    fail_after_owner: Option<usize>,
) -> Result<PersistentJournalEnrollmentReport, ScanError> {
    batch.validate()?;
    if enqueued_unix_ms < 0
        || batch.pages.iter().any(|page| {
            page.enrollment.range.batch_id != persistent_journal_batch_id(&page.enrollment)
                || page.enrollment.range.state != PersistentJournalRangeState::Enrolled
                || page.enrollment.range.enrolled_unix_ms != enqueued_unix_ms
                || page.checkpoint.continuity != PersistentJournalContinuityState::CatchingUp
                || page.checkpoint.failure.is_some()
        })
    {
        return Err(invalid_batch(
            "A volume publication must contain new ranges and non-authoritative checkpoints",
        ));
    }
    for page in &batch.pages {
        validate_enqueue_batch(&page.enrollment.intents)?;
        let evidence = LibraryChangeCatchUpEvidence {
            source: PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
            watermark: page.enrollment.range.batch_id.clone(),
        };
        validate_catch_up_evidence(&evidence)?;
    }
    validate_atomic_lineage(batch)?;

    let transaction = catalog.begin_write_in_lane(publication_admission::write_lane(batch))?;
    baseline_window::validate_publication(&transaction, batch)?;
    let mut existing = Vec::with_capacity(batch.pages.len());
    for page in &batch.pages {
        require_supported_root(&transaction, &page.enrollment)?;
        reject_conflicting_range_coordinates(&transaction, &page.enrollment)?;
        existing.push(source_range_exists(&transaction, &page.enrollment)?);
    }
    if existing.iter().any(|value| *value) {
        if !existing.iter().all(|value| *value) {
            return Err(invalid_batch(
                "An atomic volume replay cannot mix durable and missing root pages",
            ));
        }
        for page in &batch.pages {
            let stored = load_checkpoint(
                &transaction,
                &page.checkpoint.root_id,
                page.checkpoint.root_generation,
            )?
            .ok_or_else(|| invalid_batch("A replayed volume page has no checkpoint"))?;
            let range = load_checkpoint_range(&transaction, &page.enrollment.range.batch_id)?;
            if stored != page.checkpoint || range.state != PersistentJournalRangeState::Checkpointed
            {
                return Err(invalid_batch(
                    "A replayed volume batch conflicts with durable normalized content",
                ));
            }
        }
        transaction.commit().map_err(database_error)?;
        return Ok(PersistentJournalEnrollmentReport {
            enrolled_root_count: u32::try_from(batch.pages.len()).map_err(|_| {
                invalid_batch("The volume replay root count exceeded its supported bound")
            })?,
            ..PersistentJournalEnrollmentReport::default()
        });
    }

    for page in &batch.pages {
        validate_checkpoint_advance(&transaction, &page.enrollment.range, &page.checkpoint)?;
    }
    validate_pending_rename_mutations(&transaction, batch)?;
    cleanup_for_enqueue(&transaction, enqueued_unix_ms, policy)?;

    for (index, page) in batch.pages.iter().enumerate() {
        insert_source_range(&transaction, &page.enrollment)?;
        consume_live_gap_claims_with_source_range(
            &transaction,
            &page.enrollment,
            enqueued_unix_ms,
        )?;
        if !page.enrollment.intents.is_empty() {
            let evidence = LibraryChangeCatchUpEvidence {
                source: PERSISTENT_JOURNAL_CATCH_UP_SOURCE.to_owned(),
                watermark: page.enrollment.range.batch_id.clone(),
            };
            let normalized_intents =
                normalize_persistent_journal_intents(&page.enrollment.intents)?;
            let queue_report = enqueue_intents_in_transaction(
                &transaction,
                &normalized_intents,
                Some(&evidence),
                enqueued_unix_ms,
                policy,
            )
            .map_err(|error| {
                map_queue_backpressure(
                    error,
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete volume batch",
                )
            })?;
            if queue_report.capacity_degraded || queue_report.stale_generation_count != 0 {
                return Err(ScanError::new(
                    "persistent_journal_queue_capacity",
                    "The durable final-state queue could not own the complete volume batch",
                ));
            }
            transaction
                .execute(
                    "INSERT INTO library_persistent_journal_queue_lineage(
                       source_range_id, change_id, enrolled_unix_ms
                     )
                     SELECT ?1, change_id, ?2
                     FROM library_change_queue_catch_up_lineage AS lineage
                     JOIN library_change_queue AS changes ON changes.id = lineage.change_id
                     WHERE lineage.catch_up_source = ?3 AND lineage.catch_up_watermark = ?1
                       AND changes.status IN ('pending', 'leased', 'retry_wait')
                     ON CONFLICT(source_range_id, change_id) DO NOTHING",
                    params![
                        page.enrollment.range.batch_id,
                        enqueued_unix_ms,
                        PERSISTENT_JOURNAL_CATCH_UP_SOURCE,
                    ],
                )
                .map_err(database_error)?;
        }
        super::super::journal_namespace::publication::publish(&transaction, &page.enrollment)?;
        if fail_after_owner == Some(index) {
            return Err(ScanError::new(
                "persistent_journal_test_crash",
                "Injected failure before the next volume owner was published",
            ));
        }
    }

    for page in &batch.pages {
        for carry_id in &page.enrollment.consumed_pending_rename_ids {
            let deleted = transaction
                .execute(
                    "DELETE FROM library_persistent_journal_pending_renames
                     WHERE carry_id = ?1",
                    [carry_id],
                )
                .map_err(database_error)?;
            if deleted != 1 {
                return Err(invalid_batch(
                    "A consumed pending rename carry changed during publication",
                ));
            }
        }
        for carry in &page.enrollment.pending_renames {
            insert_pending_rename(&transaction, carry)?;
        }
    }

    for page in &batch.pages {
        for lineage in page
            .enrollment
            .cross_root_lineage
            .iter()
            .chain(&page.enrollment.carried_cross_root_lineage)
        {
            enroll_cross_root_lineage(&transaction, lineage, enqueued_unix_ms)?;
        }
    }
    for page in &batch.pages {
        apply_checkpoint_advance(&transaction, &page.enrollment.range, &page.checkpoint)?;
    }
    finalize_persistent_journal_ranges(&transaction, enqueued_unix_ms)?;
    transaction.commit().map_err(database_error)?;
    Ok(PersistentJournalEnrollmentReport {
        enrolled_root_count: u32::try_from(batch.pages.len())
            .map_err(|_| invalid_batch("The volume root count exceeded its supported bound"))?,
        observation_count: batch_observation_count(batch),
        advanced_checkpoint_count: u32::try_from(batch.pages.len()).map_err(|_| {
            invalid_batch("The volume checkpoint count exceeded its supported bound")
        })?,
        ..PersistentJournalEnrollmentReport::default()
    })
}

fn batch_observation_count(batch: &PersistentJournalVolumeBatch) -> u64 {
    batch
        .pages
        .iter()
        .flat_map(|page| &page.enrollment.intents)
        .map(|intent| u64::from(intent.coalesced_observation_count))
        .sum()
}
