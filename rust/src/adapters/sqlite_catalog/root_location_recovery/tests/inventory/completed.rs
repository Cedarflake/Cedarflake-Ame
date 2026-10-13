use super::*;

#[test]
fn root_location_recovery_retires_completed_baseline_without_transferring_journal_coverage() {
    let fixture = Fixture::new();
    let mut catalog = fixture.catalog();
    seed_completed_baseline(&mut catalog, &fixture.root);
    drop(catalog);

    // Validate the completed historical chain before exercising the new generation transition.
    let mut catalog = fixture.catalog();
    let located = fixture.located();
    catalog
        .recover_located_root(&fixture.root, &located, &AtomicBool::new(false))
        .unwrap();
    drop(catalog);
    let catalog = fixture.catalog();
    let (baselines, old_checkpoint, new_checkpoint, history): (i64, String, i64, i64) = catalog
        .connection
        .query_row(
            "SELECT
           (SELECT COUNT(*) FROM library_persistent_journal_baselines WHERE root_id = ?1),
           (SELECT continuity_state FROM library_persistent_journal_checkpoints
            WHERE root_id = ?1 AND root_generation = ?2),
           (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
            WHERE root_id = ?1 AND root_generation > ?2),
           (SELECT COUNT(*) FROM library_change_queue AS queue
            JOIN library_recovery_authorities AS authority ON authority.change_id = queue.id
            WHERE authority.run_id = 'old-inventory' AND authority.retired_unix_ms IS NOT NULL
              AND queue.status = 'completed')",
            params![
                fixture.root.root_id,
                sqlite_integer(fixture.root.root_generation.value(), "fixture generation").unwrap()
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(baselines, 0);
    assert_eq!(old_checkpoint, "current");
    assert_eq!(new_checkpoint, 0);
    assert_eq!(history, 1);
}

pub(super) fn seed_completed_baseline(catalog: &mut SqliteCatalog, root: &IncrementalCatalogRoot) {
    seed_inventory(catalog, root, 0);
    catalog.connection.execute_batch(
        "BEGIN IMMEDIATE;
         DELETE FROM library_metadata_inventory_runs WHERE id = 'old-inventory';
         INSERT INTO library_persistent_journal_checkpoints(
           root_id, root_generation, volume_guid, volume_serial,
           root_reference_version, root_file_reference, journal_id,
           next_unread_usn, captured_exclusive_end, covered_catalog_revision,
           protocol_version, contract_version, continuity_state, updated_unix_ms
         )
         SELECT root_id, root_generation, volume_guid, volume_serial,
                root_reference_version, root_file_reference, journal_id,
                opening_next_usn, opening_next_usn, (SELECT revision FROM catalog_state),
                protocol_version, contract_version, 'current', updated_unix_ms
         FROM library_persistent_journal_baselines;
         UPDATE library_persistent_journal_baselines
         SET closing_next_usn = opening_next_usn, phase = 'completed',
             completed_unix_ms = updated_unix_ms;
         UPDATE library_persistent_journal_root_state
         SET continuity_state = 'current';
         UPDATE library_change_queue
         SET status = 'completed', catalog_revision_at_success = (SELECT revision FROM catalog_state)
         WHERE id IN (SELECT change_id FROM library_recovery_authorities WHERE run_id = 'old-inventory');
         UPDATE library_recovery_authorities SET retired_unix_ms = authorized_unix_ms
         WHERE run_id = 'old-inventory';
         COMMIT;"
    ).unwrap();
}
