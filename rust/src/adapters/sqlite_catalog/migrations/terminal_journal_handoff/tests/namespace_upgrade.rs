use crate::adapters::sqlite_catalog::change_queue::{
    activate_root_change_queue, retire_root_change_queue,
};
use crate::adapters::sqlite_catalog::migrations::{SCHEMA_VERSION, downgrade_to_v81_for_test};
use crate::domain::{
    LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeOrigin,
    LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRecoveryAuthority,
    LibraryRecoveryAuthorityReason,
};
use crate::ports::{LibraryChangeQueue, MetadataInventoryRepository};

use super::fixture::{RecoveryFixture, handoffs};

fn replace_unstarted_namespace(fixture: &mut RecoveryFixture) -> LibraryChangeId {
    let transaction = fixture.catalog.connection.transaction().unwrap();
    transaction
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('root-b', 'C:/other-fixture', 1)",
            [],
        )
        .unwrap();
    let generation = activate_root_change_queue(&transaction, "root-b", 1).unwrap();
    transaction.commit().unwrap();
    let policy = LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: "root-b".to_owned(),
                root_generation: generation,
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .unwrap();
    let lease = fixture
        .catalog
        .lease_authoritative_library_change("root-b", generation, 2_000, policy)
        .unwrap()
        .unwrap();
    fixture
        .catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: lease.change.id,
            run_id: "unstarted-replacement".to_owned(),
            root_id: "root-b".to_owned(),
            root_generation: generation,
            reason: LibraryRecoveryAuthorityReason::ContainmentFailure,
            opening_boundary: None,
            authorized_unix_ms: 2_000,
            retired_unix_ms: None,
        })
        .unwrap();
    let transaction = fixture.catalog.connection.transaction().unwrap();
    retire_root_change_queue(&transaction, "root-b", 3_000).unwrap();
    activate_root_change_queue(&transaction, "root-b", 3_001).unwrap();
    transaction.commit().unwrap();
    // Reconstruct the omitted retirement in catalogs written before the repair.
    fixture
        .catalog
        .connection
        .execute(
            "UPDATE library_recovery_authorities SET retired_unix_ms = NULL WHERE change_id = ?1",
            [i64::try_from(lease.change.id.value()).unwrap()],
        )
        .unwrap();
    lease.change.id
}

#[test]
fn namespace_and_journal_repairs_commit_together_before_upgrade() {
    for version in [81, SCHEMA_VERSION] {
        let mut fixture = RecoveryFixture::new();
        let replaced = replace_unstarted_namespace(&mut fixture);
        fixture.retain_legacy_completion();
        if version == 81 {
            downgrade_to_v81_for_test(&fixture.catalog.connection);
        }
        let original = fixture
            .catalog
            .load_metadata_inventory_recovery_authority(replaced)
            .unwrap()
            .unwrap();
        fixture.add_handoff("unproven-range", "unproven-asset");
        assert_eq!(
            fixture.reopen().unwrap_err().code,
            "catalog_change_catch_up_contract_unverifiable"
        );
        assert_eq!(handoffs(&fixture.catalog.connection), 2);
        assert_eq!(
            fixture
                .catalog
                .load_metadata_inventory_recovery_authority(replaced)
                .unwrap()
                .unwrap(),
            original,
            "unrelated damage rolls both repairs back"
        );
        fixture.catalog.connection.execute(
            "DELETE FROM library_change_catch_up_handoffs WHERE catch_up_watermark = 'unproven-range'",
            [],
        ).unwrap();
        fixture
            .reopen()
            .expect("both proven legacy repairs precede upgrade validation");
        assert_eq!(handoffs(&fixture.catalog.connection), 0);
        let repaired = fixture
            .catalog
            .load_metadata_inventory_recovery_authority(replaced)
            .unwrap()
            .unwrap();
        assert_eq!(
            repaired,
            LibraryRecoveryAuthority {
                retired_unix_ms: Some(3_001),
                ..original
            }
        );
        fixture.reopen().expect("combined repair is idempotent");
        assert_eq!(
            fixture
                .catalog
                .load_metadata_inventory_recovery_authority(replaced)
                .unwrap()
                .unwrap(),
            repaired
        );
    }
}
