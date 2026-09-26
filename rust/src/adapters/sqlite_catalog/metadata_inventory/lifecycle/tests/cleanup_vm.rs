use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use rusqlite::{Connection, params};

use crate::domain::{
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryFrontierEntry,
    MetadataInventoryPage, MetadataInventoryPlaceholderState, MetadataInventoryRunStatus,
};
use crate::ports::{CatalogRepository, MetadataInventoryRepository};

use super::SqliteCatalog;
use super::races::{catalog_with_baseline, request};

#[derive(Clone, Copy, Debug)]
enum CleanupPayload {
    LogicalEntries,
    CandidateOwners,
}

#[test]
fn bounded_logical_cleanup_vm_work_does_not_scale_with_remaining_entries() {
    assert_fixed_batch_work(CleanupPayload::LogicalEntries);
}

#[test]
fn bounded_candidate_cleanup_vm_work_does_not_scale_with_remaining_owners() {
    assert_fixed_batch_work(CleanupPayload::CandidateOwners);
}

fn assert_fixed_batch_work(payload: CleanupPayload) {
    let small = cleanup_steps(payload, 1_024);
    let large = cleanup_steps(payload, 8_192);
    eprintln!("bounded cleanup payload={payload:?} VM steps: small={small}, large={large}");
    assert!(
        small > 0 && large > 0,
        "both actual SQL batches must be observed"
    );
    assert!(
        large <= small.saturating_mul(2),
        "an eightfold retained {payload:?} payload cannot expand a one-row cleanup batch: \
         small={small}, large={large}",
    );
}

fn cleanup_steps(payload: CleanupPayload, count: u32) -> usize {
    let (_source, _storage, mut catalog) = catalog_with_baseline();
    let request = request("inventory-cleanup-vm");
    catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin inventory against the published baseline");
    match payload {
        CleanupPayload::LogicalEntries => stage_entries(&mut catalog, &request.run_id, count),
        CleanupPayload::CandidateOwners => {
            seed_candidate_owners(&mut catalog, &request.run_id, &request.root_id, count);
        }
    }
    catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "controlled terminal cleanup fixture")),
            2_200,
        )
        .expect("retire the actual inventory before cleanup");
    assert_valid_catalog(&catalog);
    let before = payload_counts(&catalog, &request.run_id);
    let expected = match payload {
        CleanupPayload::LogicalEntries => (i64::from(count), 0, 0),
        CleanupPayload::CandidateOwners => (0, i64::from(count), i64::from(count)),
    };
    assert_eq!(before, expected);

    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    let transaction = catalog
        .connection
        .transaction()
        .expect("one SQL cleanup batch");
    transaction
        .progress_handler(
            1,
            Some(move || {
                observed.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
        .expect("count every executed SQLite VM instruction");
    let progress = ClearProgress(&transaction);
    // Observe the real SQL owner directly; the admission wrapper installs its cancellation handler.
    let result = super::super::cleanup_in_transaction(&transaction, 3_000, 1, 1);
    drop(progress);
    let report = result.expect("one bounded production SQL batch");
    transaction
        .commit()
        .expect("persist exactly one cleanup batch");
    assert_eq!(
        report.removed_entry_count,
        u32::from(matches!(payload, CleanupPayload::LogicalEntries)),
    );
    assert_eq!(report.removed_run_count, 0);
    assert!(report.has_more);
    let after = payload_counts(&catalog, &request.run_id);
    match payload {
        CleanupPayload::LogicalEntries => assert_eq!(after, (before.0 - 1, 0, 0)),
        CleanupPayload::CandidateOwners => {
            assert_eq!(
                after,
                (0, before.1 - 1, before.2),
                "queue lineage is not pruned by owner cleanup"
            );
        }
    }
    assert!(
        catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("read retained summary")
            .is_some(),
        "a one-row batch cannot cascade the remaining payload through its run",
    );
    assert_valid_catalog(&catalog);
    steps.load(Ordering::Relaxed)
}

fn stage_entries(catalog: &mut SqliteCatalog, run_id: &str, count: u32) {
    for (page, start) in (0..count).step_by(4_096).enumerate() {
        let end = (start + 4_096).min(count);
        let cursor = format!("entry-{:05}.txt", end - 1);
        catalog
            .stage_metadata_inventory_page(
                run_id,
                &MetadataInventoryPage {
                    page_index: u64::try_from(page).expect("bounded fixture page") + 1,
                    entries: (start..end)
                        .map(|index| MetadataInventoryEntry {
                            relative_path: format!("entry-{index:05}.txt"),
                            kind: MetadataInventoryEntryKind::File,
                            file_size: Some(1),
                            modified_unix_ms: 1,
                            file_identity: None,
                            source_revision: None,
                            placeholder_state: MetadataInventoryPlaceholderState::Available,
                            is_reparse_point: false,
                        })
                        .collect(),
                    cursor: Some(cursor.clone()),
                    is_complete: false,
                    frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                        0,
                        "",
                        None,
                        Some(cursor),
                        u64::from(end),
                    )],
                },
                2_100,
            )
            .expect("stage a real bounded logical page and frontier");
    }
}

fn seed_candidate_owners(catalog: &mut SqliteCatalog, run_id: &str, root_id: &str, count: u32) {
    let transaction = catalog
        .connection
        .transaction()
        .expect("atomic candidate lineage fixture");
    {
        let mut queue = transaction.prepare(
            "INSERT INTO library_change_queue(
               id, root_id, root_generation, intent_kind, scope, relative_path, origin,
               first_observed_unix_ms, most_recent_observed_unix_ms, first_sequence, most_recent_sequence,
               coalesced_observation_count, status, ready_unix_ms, catalog_revision_at_enqueue,
               created_unix_ms, updated_unix_ms)
             VALUES (?1, ?2, 1, 'reconcile', 'path', ?3, 'metadata_inventory',
                     2100, 2100, '1', '1', 1, 'pending', 2100, 0, 2100, 2100)",
        ).expect("prepare actual rooted candidate queue rows");
        let mut owners = transaction
            .prepare(
                "INSERT INTO library_metadata_inventory_candidate_owners(
               run_id, candidate_key, change_id, candidate_role, relative_path, owned_unix_ms)
             VALUES (?1, ?2, ?3, 'present', ?2, 2100)",
            )
            .expect("prepare ownership with both foreign-key parents");
        for index in 0..count {
            let path = format!("candidate-{index:05}.txt");
            let change_id = i64::from(index) + 10_000;
            queue
                .execute(params![change_id, root_id, path])
                .expect("persist queue parent");
            owners
                .execute(params![run_id, path, change_id])
                .expect("persist scoped owner");
        }
    }
    transaction
        .commit()
        .expect("publish complete candidate lineage");
}

fn payload_counts(catalog: &SqliteCatalog, run_id: &str) -> (i64, i64, i64) {
    catalog.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = ?1),
                (SELECT COUNT(*) FROM library_change_queue WHERE origin = 'metadata_inventory')",
        [run_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).expect("exact logical payload and queue-parent counts")
}

fn assert_valid_catalog(catalog: &SqliteCatalog) {
    let foreign_keys: bool = catalog
        .connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("read actual foreign-key enforcement");
    assert!(
        foreign_keys,
        "fixture must retain production foreign-key enforcement"
    );
    let violation = catalog
        .connection
        .prepare("PRAGMA foreign_key_check")
        .expect("prepare full foreign-key proof")
        .exists([])
        .expect("read full foreign-key proof");
    assert!(!violation);
    SqliteCatalog::open(catalog.catalog_path().to_path_buf())
        .expect("fresh FULL open proves the complete catalog contract");
}

struct ClearProgress<'connection>(&'connection Connection);

impl Drop for ClearProgress<'_> {
    fn drop(&mut self) {
        let _ = self.0.progress_handler(0, None::<fn() -> bool>);
    }
}
