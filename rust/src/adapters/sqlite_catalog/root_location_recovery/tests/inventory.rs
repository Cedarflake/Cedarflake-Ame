use super::*;
use crate::domain::{
    JournalFileReference, JournalIdentifier, JournalUsn, LibraryRecoveryAuthorityReason,
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryFrontierEntry,
    MetadataInventoryPage, MetadataInventoryPlaceholderState, MetadataInventoryRunRequest,
    MetadataInventoryScope, PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalContinuityState,
    PersistentJournalVolumeIdentity,
};
use crate::ports::{MetadataInventoryRepository, PersistentJournalRepository};

mod completed;
mod journal_claim;
mod payload;

#[test]
fn root_location_recovery_retires_active_inventory_and_unfinished_baseline_before_reopen() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_inventory(&mut catalog, &fixture.root, 2);
    drop(catalog);
    let mut catalog = fixture.catalog();
    let located = fixture.located();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let catalog = fixture.catalog();
    let pending: i64 = catalog.connection.query_row(
        "SELECT (SELECT COUNT(*) FROM library_metadata_inventory_runs WHERE id = 'old-inventory')
              + (SELECT COUNT(*) FROM library_persistent_journal_baselines AS baseline
                 JOIN library_recovery_authorities AS authority
                   ON authority.change_id = baseline.change_id
                 WHERE authority.run_id = 'old-inventory')
              + (SELECT COUNT(*) FROM library_recovery_authorities
                 WHERE run_id = 'old-inventory' AND retired_unix_ms IS NULL)",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(pending, 0);
    assert_eq!(
        catalog
            .load_incremental_catalog_root(&fixture.root.root_id)
            .unwrap()
            .unwrap()
            .root_generation
            .value(),
        fixture.root.root_generation.value() + 1
    );
}

#[test]
fn root_location_recovery_pages_large_inventory_before_publishing_the_binding() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_inventory(&mut catalog, &fixture.root, 3_073);
    let located = fixture.located();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let catalog = fixture.catalog();
    let current = catalog
        .load_incremental_catalog_root(&fixture.root.root_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        current.root_generation.value(),
        fixture.root.root_generation.value() + 1
    );
    assert_eq!(current.root_path, located.path());
    let entries: i64 = catalog.connection.query_row(
        "SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = 'old-inventory'",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(entries, 0);
    assert!(
        catalog
            .load_metadata_inventory_run("old-inventory")
            .unwrap()
            .is_none()
    );
}

pub(super) fn seed_inventory(
    catalog: &mut SqliteCatalog,
    root: &IncrementalCatalogRoot,
    entries: u32,
) {
    let now = unix_time_ms();
    catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: now,
        })
        .unwrap();
    let identity = root.publication_root_identity.as_ref().unwrap();
    let volume_serial = u64::from_str_radix(&identity.value[..16], 16).unwrap();
    let file_id = u128::from_str_radix(&identity.value[17..], 16)
        .unwrap()
        .to_le_bytes();
    catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "old-inventory".into(),
                root_id: root.root_id.clone(),
                root_generation: root.root_generation,
                authority_reason: LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "fixture-volume".into(),
                    volume_serial,
                },
                root_file_reference: JournalFileReference::V3(file_id),
                journal_id: JournalIdentifier::new(44).unwrap(),
                opening_next_usn: JournalUsn::new(20).unwrap(),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: now,
            },
            LibraryChangeQueuePolicy::default(),
        )
        .unwrap();
    catalog
        .begin_metadata_inventory(&MetadataInventoryRunRequest {
            run_id: "old-inventory".into(),
            root_id: root.root_id.clone(),
            root_generation: root.root_generation,
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: now,
        })
        .unwrap();
    catalog
        .stage_metadata_inventory_page(
            "old-inventory",
            &MetadataInventoryPage {
                page_index: 1,
                entries: (0..entries)
                    .map(|index| MetadataInventoryEntry {
                        relative_path: format!("entry-{index:04}.png"),
                        kind: MetadataInventoryEntryKind::File,
                        file_size: Some(1),
                        modified_unix_ms: now,
                        file_identity: None,
                        source_revision: None,
                        placeholder_state: MetadataInventoryPlaceholderState::Available,
                        is_reparse_point: false,
                    })
                    .collect(),
                cursor: entries
                    .checked_sub(1)
                    .map(|last| format!("entry-{last:04}.png")),
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed(
                    "",
                    Some(identity.clone()),
                )],
            },
            now,
        )
        .unwrap();
}
