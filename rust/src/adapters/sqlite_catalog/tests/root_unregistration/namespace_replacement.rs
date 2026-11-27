use super::*;
use crate::domain::FileIdentityEvidence;

fn relocate(fixture: &mut Fixture) -> LibraryChangeId {
    let leased = fixture.authorize("retained", "unstarted-recovery", 2_000);
    let expected = fixture
        .catalog
        .load_incremental_catalog_root("retained")
        .unwrap()
        .unwrap();
    let destination = fixture.source.path().join("replacement");
    fs::create_dir(&destination).unwrap();
    let request = ScanRequest {
        scan_id: "replacement".to_owned(),
        root_path: destination.to_string_lossy().into_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    fixture
        .catalog
        .begin_relocated_scan(
            &request,
            &expected,
            &request.root_path,
            &FileIdentityEvidence {
                scheme: "windows-file-id-128-v1".to_owned(),
                value: "0000000000000001:00000000000000000000000000000002".to_owned(),
            },
        )
        .unwrap();
    leased.change.id
}

#[test]
fn namespace_replacement_retires_unstarted_recovery_before_catalog_reopen() {
    let mut fixture = Fixture::new(&["retained", "unrelated"]);
    let unrelated = fixture.authorize("unrelated", "still-active", 2_100);
    let before = fixture.authority(unrelated.change.id);
    let replaced = relocate(&mut fixture);
    assert_eq!(fixture.queue_status(replaced), "superseded");
    assert!(fixture.authority(replaced).retired_unix_ms.is_some());
    fixture.reopen();
    assert_eq!(fixture.authority(unrelated.change.id), before);
    assert_eq!(fixture.queue_status(unrelated.change.id), "leased");
}

#[test]
fn namespace_replacement_legacy_repair_preserves_history_and_rejects_unrelated_damage() {
    {
        let mut fixture = Fixture::new(&["retained", "unrelated"]);
        fixture.authorize("unrelated", "unrelated-inventory", 2_100);
        fixture.advance("unrelated-inventory", "unrelated", InventoryPhase::Running);
        let replaced = relocate(&mut fixture);
        fixture.catalog.connection.execute(
            "UPDATE library_recovery_authorities SET retired_unix_ms = NULL WHERE change_id = ?1",
            [i64::try_from(replaced.value()).unwrap()],
        ).unwrap();
        let old = fixture.authority(replaced);
        fixture.catalog.connection.execute_batch(
            "UPDATE library_change_queue SET root_generation = 2 WHERE root_id = 'retained' AND id IN
             (SELECT change_id FROM library_recovery_authorities WHERE run_id = 'unstarted-recovery');"
        ).unwrap();
        assert!(SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf()).is_err());
        assert_eq!(fixture.authority(replaced), old);
        fixture.catalog.connection.execute_batch(
            "UPDATE library_change_queue SET root_generation = 1 WHERE root_id = 'retained' AND id IN
             (SELECT change_id FROM library_recovery_authorities WHERE run_id = 'unstarted-recovery');"
        ).unwrap();
        fixture
            .catalog
            .connection
            .execute_batch(
                "UPDATE library_metadata_inventory_runs SET staged_entry_count = 1
             WHERE id = 'unrelated-inventory';",
            )
            .unwrap();
        assert!(SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf()).is_err());
        assert_eq!(
            fixture.authority(replaced),
            old,
            "unrelated failure rolls repair back"
        );
        fixture
            .catalog
            .connection
            .execute_batch(
                "UPDATE library_metadata_inventory_runs SET staged_entry_count = 0
             WHERE id = 'unrelated-inventory';",
            )
            .unwrap();
        fixture.reopen();
        let repaired = fixture.authority(replaced);
        assert!(repaired.retired_unix_ms.is_some());
        assert_eq!(
            repaired,
            LibraryRecoveryAuthority {
                retired_unix_ms: repaired.retired_unix_ms,
                ..old
            }
        );
        fixture.reopen();
        assert_eq!(fixture.authority(replaced), repaired);
    }
}
