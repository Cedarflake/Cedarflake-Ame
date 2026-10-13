use super::*;

#[test]
fn live_gap_claim_retains_older_control_without_starving_its_own_retirement() {
    let storage = tempdir().expect("controlled catalog storage");
    let path = storage.path().join("catalog.sqlite3");
    let (mut catalog, authority) =
        recovery_inventory_catalog(path.clone(), "retained-control", &[]);
    let control_id = i64::try_from(authority.change.id.value()).expect("SQLite control id");
    catalog
        .authorize_metadata_inventory_absence("retained-control", 1_100)
        .expect("authorize empty absence set");
    catalog
        .complete_metadata_inventory("retained-control", 1_100)
        .expect("finish empty comparison");
    assert_eq!(
        catalog
            .finish_metadata_inventory_recovery(
                authority.change.id,
                authority.lease_generation,
                0,
                1_200
            )
            .expect("complete control"),
        Some(LibraryChangeLeaseUpdateOutcome::Applied)
    );
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET origin = 'metadata_inventory' WHERE id = ?1",
            [control_id],
        )
        .expect("bind metadata control lane");
    let gap_id = seed_explicit_recovery_claim(&catalog, 2_000);
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
         SET origin = 'live_notification', status = 'superseded',
             superseded_by_change_id = ?2, updated_unix_ms = 3000,
             last_failure_code = NULL, last_failure_message = NULL
         WHERE id = ?1",
            rusqlite::params![gap_id, control_id],
        )
        .expect("retain completed live-gap origin");
    catalog
        .connection
        .execute(
            "UPDATE library_live_gap_recovery_claims
         SET consumer_kind = 'metadata_inventory_control', recovery_change_id = ?2,
             consumed_unix_ms = 3000 WHERE gap_change_id = ?1",
            rusqlite::params![gap_id, control_id],
        )
        .expect("retain consumed control claim");
    drop(catalog);
    let mut catalog =
        SqliteCatalog::open(path.clone()).expect("full validation before retention cleanup");

    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(4_000, 1)
            .expect("retire eligible gap before older referenced control"),
        1
    );
    let retained: (bool, i64) = catalog
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM library_change_queue WHERE id = ?1),
                (SELECT COUNT(*) FROM library_live_gap_recovery_claims)",
            [control_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retained control and retired claim");
    assert_eq!(retained, (true, 0));
    assert_eq!(
        catalog
            .cleanup_terminal_library_changes(4_000, 1)
            .expect("retire released control"),
        1
    );
    drop(catalog);
    SqliteCatalog::open(path).expect("full reopen after ordered retirement");
}
