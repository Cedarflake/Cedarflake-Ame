use rusqlite::{Connection, params};
use tempfile::TempDir;

use crate::adapters::SqliteCatalog;
use crate::adapters::sqlite_catalog::change_queue::activate_root_change_queue;
use crate::domain::{
    JournalScopeRecoveryOutcome, LibraryChangeCatchUpEvidence, LibraryChangeFailure,
    LibraryChangeId, LibraryChangeIntent, LibraryChangeIntentKind, LibraryChangeLeaseUpdateOutcome,
    LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope, LibraryRootGeneration,
    ScanError,
};
use crate::ports::LibraryChangeQueue;

pub(super) struct RecoveryFixture {
    pub(super) catalog: SqliteCatalog,
    pub(super) parent: LibraryChangeId,
    pub(super) lease_generation: u64,
    _directory: TempDir,
}

fn policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        ..LibraryChangeQueuePolicy::default()
    }
}

impl RecoveryFixture {
    pub(super) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        let transaction = catalog.connection.transaction().unwrap();
        transaction.execute(
            "INSERT INTO library_roots(id, path, created_unix_ms) VALUES ('root-a', 'C:/fixture', 1)",
            [],
        ).unwrap();
        let generation = activate_root_change_queue(&transaction, "root-a", 1).unwrap();
        transaction.commit().unwrap();
        let intent = LibraryChangeIntent {
            root_id: "root-a".to_owned(),
            root_generation: generation,
            kind: LibraryChangeIntentKind::Reconcile,
            scope: LibraryChangeScope::Subtree,
            relative_path: "album".to_owned(),
            previous_relative_path: None,
            origin: LibraryChangeOrigin::StartupCatchUp,
            first_observed_unix_ms: 1_000,
            most_recent_observed_unix_ms: 1_000,
            first_sequence: 1,
            most_recent_sequence: 1,
            coalesced_observation_count: 1,
        };
        let evidence = LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: "scope-range".to_owned(),
        };
        catalog
            .enqueue_library_change_intents_with_catch_up(&[intent], &evidence, 1_000, policy())
            .unwrap();
        let lease = catalog
            .lease_authoritative_library_change("root-a", generation, 1_000, policy())
            .unwrap()
            .unwrap();
        assert_eq!(
            catalog
                .promote_journal_scope_to_metadata_inventory(
                    lease.change.id,
                    lease.lease_generation,
                    &LibraryChangeFailure {
                        code: "metadata_inventory_required".to_owned(),
                        message: "The namespace exceeds the bounded delta".to_owned(),
                    },
                    1_001,
                    policy(),
                )
                .unwrap(),
            LibraryChangeLeaseUpdateOutcome::Applied
        );
        catalog
            .connection
            .execute_batch(
                "UPDATE library_change_queue SET status = 'completed',
               catalog_revision_at_success = 0, updated_unix_ms = 1002
             WHERE id IN (SELECT current_change_id FROM library_journal_scope_recoveries);
             UPDATE library_recovery_authorities SET retired_unix_ms = 1002;",
            )
            .unwrap();
        let lease = catalog
            .lease_authoritative_library_change("root-a", generation, 1_003, policy())
            .unwrap()
            .unwrap();
        let fixture = Self {
            catalog,
            parent: lease.change.id,
            lease_generation: lease.lease_generation,
            _directory: directory,
        };
        fixture.add_handoff("scope-range", "retained-asset");
        fixture
    }

    pub(super) fn add_handoff(&self, watermark: &str, asset: &str) {
        self.catalog
            .connection
            .execute(
                "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
                [asset],
            )
            .unwrap();
        self.catalog
            .connection
            .execute(
                "INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, modified_unix_ms, width, height, preview_status,
               metadata_engine_id, metadata_engine_version, updated_unix_ms
             ) VALUES ('windows_usn_v1', ?1, 'windows-file-id-128-v1', ?2,
               ?2, 'location-a', 'root-a', 'C:/fixture/photo.jpg', 'photo.jpg',
               '', 1, 1, 1, 1, 'pending', 'metadata', '1', 1001)",
                params![watermark, asset],
            )
            .unwrap();
    }

    pub(super) fn complete(&mut self) -> Result<Option<JournalScopeRecoveryOutcome>, ScanError> {
        self.catalog.continue_journal_scope_recovery(
            self.parent,
            self.lease_generation,
            0,
            1_004,
            policy(),
        )
    }

    pub(super) fn retain_legacy_completion(&self) {
        self.catalog.connection.execute(
            "UPDATE library_change_queue SET status = 'completed', lease_expires_unix_ms = NULL,
               catalog_revision_at_success = 0, updated_unix_ms = 1004 WHERE id = ?1",
            [i64::try_from(self.parent.value()).unwrap()],
        ).unwrap();
    }

    pub(super) fn reopen(&mut self) -> Result<(), ScanError> {
        self.catalog = SqliteCatalog::open(self.catalog.path.clone())?;
        Ok(())
    }
}

pub(super) fn handoffs(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM library_change_catch_up_handoffs",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn active_sibling(fixture: &mut RecoveryFixture) {
    let intent = LibraryChangeIntent {
        root_id: "root-a".to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind: LibraryChangeIntentKind::Reconcile,
        scope: LibraryChangeScope::Path,
        relative_path: "other/photo.jpg".to_owned(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::StartupCatchUp,
        first_observed_unix_ms: 1003,
        most_recent_observed_unix_ms: 1003,
        first_sequence: 2,
        most_recent_sequence: 2,
        coalesced_observation_count: 1,
    };
    fixture
        .catalog
        .enqueue_library_change_intents_with_catch_up(
            &[intent],
            &LibraryChangeCatchUpEvidence {
                source: "windows_usn_v1".to_owned(),
                watermark: "scope-range".to_owned(),
            },
            1003,
            policy(),
        )
        .unwrap();
}
