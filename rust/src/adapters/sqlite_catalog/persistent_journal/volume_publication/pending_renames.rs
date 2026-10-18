use rusqlite::{OptionalExtension, Transaction, params};

use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRootGeneration,
    PersistentJournalCrossRootLineage, PersistentJournalPendingRename,
    PersistentJournalVolumeBatch, PersistentJournalVolumeIdentity, ScanError,
};

use super::super::super::{database_error, sqlite_integer};

use super::super::invalid_batch;

pub(super) fn validate_pending_rename_mutations(
    transaction: &Transaction<'_>,
    batch: &PersistentJournalVolumeBatch,
) -> Result<(), ScanError> {
    let mut consumed = std::collections::BTreeSet::new();
    let mut created = std::collections::BTreeSet::new();
    let carried_ids = batch
        .pages
        .iter()
        .flat_map(|page| {
            page.enrollment
                .cross_root_lineage
                .iter()
                .chain(&page.enrollment.carried_cross_root_lineage)
        })
        .filter_map(|lineage| lineage.previous_carry_id.as_deref())
        .collect::<Vec<_>>();
    for page in &batch.pages {
        for carry_id in &page.enrollment.consumed_pending_rename_ids {
            if !consumed.insert(carry_id.as_str()) {
                return Err(invalid_batch("A pending rename carry is consumed twice"));
            }
            let carry = load_pending_rename_by_id(transaction, carry_id)?;
            let Some(carry) = carry else {
                return Err(invalid_batch(
                    "A consumed pending rename carry has no durable predecessor",
                ));
            };
            let lineages = batch
                .pages
                .iter()
                .flat_map(|candidate_page| {
                    candidate_page
                        .enrollment
                        .cross_root_lineage
                        .iter()
                        .chain(&candidate_page.enrollment.carried_cross_root_lineage)
                        .map(move |lineage| (candidate_page, lineage))
                })
                .filter(|(_, lineage)| lineage.previous_carry_id.as_deref() == Some(carry_id))
                .collect::<Vec<_>>();
            let [left, right] = lineages.as_slice() else {
                return Err(invalid_batch(
                    "A consumed carry must bind exactly two carried lineage owners",
                ));
            };
            let owner_matches =
                |candidate_page: &&crate::domain::PersistentJournalVolumePage,
                 lineage: &&PersistentJournalCrossRootLineage| {
                    let is_previous = lineage.owner_source_range_id == carry.source_range_id;
                    let is_current = lineage.owner_source_range_id
                        == candidate_page.enrollment.range.batch_id
                        && lineage.current_root_id == candidate_page.enrollment.range.root_id
                        && lineage.current_root_generation
                            == candidate_page.enrollment.range.root_generation
                        && lineage.new_usn.is_some_and(|usn| {
                            usn >= candidate_page.enrollment.range.requested_start_usn
                                && usn < candidate_page.enrollment.range.covered_until_usn
                        });
                    (is_previous || is_current)
                        && lineage.volume == carry.volume
                        && lineage.journal_id == carry.journal_id
                        && lineage.file_reference == carry.file_reference
                        && lineage.old_usn == Some(carry.old_usn)
                        && lineage.previous_root_id == carry.previous_root_id
                        && lineage.previous_root_generation == carry.previous_root_generation
                        && lineage.previous_relative_path == carry.previous_relative_path
                };
            if left.1.lineage_id != right.1.lineage_id
                || left.1.owner_source_range_id == right.1.owner_source_range_id
                || !owner_matches(&left.0, &left.1)
                || !owner_matches(&right.0, &right.1)
            {
                return Err(invalid_batch(
                    "A consumed carry does not exactly match both carried lineage owners",
                ));
            }
        }
        for carry in &page.enrollment.pending_renames {
            if !created.insert(carry.carry_id.as_str())
                || consumed.contains(carry.carry_id.as_str())
            {
                return Err(invalid_batch(
                    "A pending rename carry mutation is duplicated or contradictory",
                ));
            }
        }
    }
    if carried_ids.len()
        != consumed
            .len()
            .checked_mul(2)
            .ok_or_else(|| invalid_batch("The carried lineage count overflowed"))?
        || carried_ids
            .iter()
            .any(|carry_id| !consumed.contains(carry_id))
    {
        return Err(invalid_batch(
            "Carried lineage evidence must be consumed exactly once",
        ));
    }
    let current = transaction
        .query_row(
            "SELECT COUNT(*) FROM library_persistent_journal_pending_renames",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(database_error)?;
    let next = current
        .checked_sub(i64::try_from(consumed.len()).unwrap_or(i64::MAX))
        .and_then(|value| value.checked_add(i64::try_from(created.len()).ok()?))
        .ok_or_else(|| invalid_batch("The pending rename carry count overflowed"))?;
    if !(0..=1_024).contains(&next) {
        return Err(ScanError::new(
            "persistent_journal_pending_rename_capacity",
            "The durable pending rename carry cannot exceed 1024 records",
        ));
    }
    Ok(())
}

fn load_pending_rename_by_id(
    transaction: &Transaction<'_>,
    carry_id: &str,
) -> Result<Option<PersistentJournalPendingRename>, ScanError> {
    let stored = transaction
        .query_row(
            "SELECT source_range_id, volume_guid, volume_serial, journal_id,
                    file_reference_version, file_reference, old_usn, previous_root_id,
                    previous_root_generation, previous_relative_path, is_directory,
                    enrolled_unix_ms
             FROM library_persistent_journal_pending_renames WHERE carry_id = ?1",
            [carry_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, bool>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    let file_reference = JournalFileReference::from_bytes(&stored.5)?;
    if i64::from(file_reference.record_version()) != stored.4 {
        return Err(invalid_batch(
            "The durable carry reference version is invalid",
        ));
    }
    let carry = PersistentJournalPendingRename {
        carry_id: carry_id.to_owned(),
        source_range_id: stored.0,
        volume: PersistentJournalVolumeIdentity {
            volume_guid: stored.1,
            volume_serial: stored
                .2
                .parse()
                .map_err(|_| invalid_batch("The durable carry serial is invalid"))?,
        },
        journal_id: JournalIdentifier::parse_canonical(&stored.3)?,
        file_reference,
        old_usn: JournalUsn::parse_canonical(&stored.6)?,
        previous_root_id: stored.7,
        previous_root_generation: LibraryRootGeneration::new(
            u64::try_from(stored.8)
                .map_err(|_| invalid_batch("The durable carry generation is invalid"))?,
        )
        .ok_or_else(|| invalid_batch("The durable carry generation is invalid"))?,
        previous_relative_path: stored.9,
        is_directory: stored.10,
        enrolled_unix_ms: stored.11,
    };
    carry.validate()?;
    Ok(Some(carry))
}

pub(super) fn insert_pending_rename(
    transaction: &Transaction<'_>,
    carry: &PersistentJournalPendingRename,
) -> Result<(), ScanError> {
    carry.validate()?;
    transaction
        .execute(
            "INSERT INTO library_persistent_journal_pending_renames(
               carry_id, source_range_id, volume_guid, volume_serial, journal_id,
               file_reference_version, file_reference, old_usn,
               previous_root_id, previous_root_generation, previous_relative_path,
               is_directory, enrolled_unix_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                carry.carry_id,
                carry.source_range_id,
                carry.volume.volume_guid,
                carry.volume.canonical_serial(),
                carry.journal_id.to_canonical_text(),
                i64::from(carry.file_reference.record_version()),
                carry.file_reference.as_bytes(),
                carry.old_usn.to_canonical_text(),
                carry.previous_root_id,
                sqlite_integer(carry.previous_root_generation.value(), "root generation")?,
                carry.previous_relative_path,
                carry.is_directory,
                carry.enrolled_unix_ms,
            ],
        )
        .map_err(database_error)?;
    Ok(())
}
