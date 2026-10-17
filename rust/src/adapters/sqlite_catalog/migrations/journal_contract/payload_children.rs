use rusqlite::Connection;

use crate::domain::{
    ScanError, persistent_journal_batch_payload_children,
    persistent_journal_pending_rename_from_payload_entry,
};

use super::super::database_error;

pub(in crate::adapters::sqlite_catalog) fn validate_payload_children(
    connection: &Connection,
) -> Result<(), ScanError> {
    let payloads = load_payload_children(connection)?;
    super::payload_intents::validate(connection, &payloads)?;
    let lineage_proofs = super::lineage_payload::load_lineage_proofs(connection)?;
    let pending_renames = super::lineage_payload::load_pending_rename_proofs(connection)?;
    let mut lineage_slots = std::collections::BTreeMap::<String, Vec<Vec<String>>>::new();
    let mut consumed_by_range = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut consumed_proofs = std::collections::BTreeMap::<
        String,
        &super::lineage_payload::PersistentJournalLineagePayloadProof,
    >::new();
    for proof in &lineage_proofs {
        let lineage = &proof.lineage;
        if let Some(carry_id) = &lineage.previous_carry_id {
            if consumed_proofs.insert(carry_id.clone(), proof).is_some() {
                return Err(super::values::unverifiable_contract());
            }
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(super::lineage_payload::lineage_payload_candidates(
                    lineage,
                    &proof.previous_range_id,
                    &proof.current_range_id,
                ));
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(super::lineage_payload::lineage_payload_candidates(
                    lineage,
                    &proof.current_range_id,
                    &proof.current_range_id,
                ));
            consumed_by_range
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(carry_id.clone());
        } else {
            lineage_slots
                .entry(proof.previous_range_id.clone())
                .or_default()
                .push(super::lineage_payload::lineage_payload_candidates(
                    lineage,
                    &proof.previous_range_id,
                    &proof.previous_range_id,
                ));
            lineage_slots
                .entry(proof.current_range_id.clone())
                .or_default()
                .push(super::lineage_payload::lineage_payload_candidates(
                    lineage,
                    &proof.current_range_id,
                    &proof.current_range_id,
                ));
        }
    }
    for expected in consumed_by_range.values_mut() {
        expected.sort();
    }

    let mut durable_pending = std::collections::BTreeMap::new();
    for pending in pending_renames {
        if consumed_proofs.contains_key(&pending.carry_id)
            || durable_pending
                .insert(pending.carry_id.clone(), pending)
                .is_some()
        {
            return Err(super::values::unverifiable_contract());
        }
    }
    let mut payload_pending = std::collections::BTreeMap::new();
    for (range_id, children) in &payloads {
        let slots = lineage_slots
            .get(range_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !lineage_payload_slots_match(&children.lineage, slots) {
            return Err(super::values::unverifiable_contract());
        }
        let expected_consumed = consumed_by_range
            .get(range_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if children.consumed_pending_rename_ids != expected_consumed {
            return Err(super::values::unverifiable_contract());
        }
        for entry in &children.pending_renames {
            let pending = persistent_journal_pending_rename_from_payload_entry(entry, range_id)
                .ok_or_else(super::values::unverifiable_contract)?;
            if payload_pending
                .insert(pending.carry_id.clone(), pending)
                .is_some()
            {
                return Err(super::values::unverifiable_contract());
            }
        }
    }

    for (carry_id, pending) in &payload_pending {
        if let Some(durable) = durable_pending.get(carry_id) {
            if pending != durable {
                return Err(super::values::unverifiable_contract());
            }
            continue;
        }
        let proof = consumed_proofs
            .get(carry_id)
            .ok_or_else(super::values::unverifiable_contract)?;
        if !super::lineage_payload::pending_rename_matches_consumed_lineage(pending, proof) {
            return Err(super::values::unverifiable_contract());
        }
    }
    for (carry_id, pending) in &durable_pending {
        if payload_pending.get(carry_id) != Some(pending) {
            return Err(super::values::unverifiable_contract());
        }
    }
    for (carry_id, proof) in consumed_proofs {
        let pending = payload_pending
            .get(&carry_id)
            .ok_or_else(super::values::unverifiable_contract)?;
        if !super::lineage_payload::pending_rename_matches_consumed_lineage(pending, proof) {
            return Err(super::values::unverifiable_contract());
        }
    }
    Ok(())
}

pub(super) fn load_payload_children(
    connection: &Connection,
) -> Result<
    std::collections::BTreeMap<String, crate::domain::PersistentJournalBatchPayloadChildren>,
    ScanError,
> {
    let mut statement = connection
        .prepare(
            "SELECT id, canonical_payload
             FROM library_persistent_journal_source_ranges ORDER BY id",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .map_err(database_error)?;
    let mut payloads = std::collections::BTreeMap::new();
    for row in rows {
        let (range_id, payload) = row.map_err(database_error)?;
        let children = persistent_journal_batch_payload_children(&payload)
            .ok_or_else(super::values::unverifiable_contract)?;
        if payloads.insert(range_id, children).is_some() {
            return Err(super::values::unverifiable_contract());
        }
    }
    Ok(payloads)
}

pub(super) fn lineage_payload_slots_match(actual: &[String], expected: &[Vec<String>]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }
    let mut matched = vec![false; expected.len()];
    for entry in actual {
        let candidates = expected
            .iter()
            .enumerate()
            .filter(|(index, slot)| !matched[*index] && slot.contains(entry))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [index] = candidates.as_slice() else {
            return false;
        };
        matched[*index] = true;
    }
    matched.into_iter().all(|is_matched| is_matched)
}
