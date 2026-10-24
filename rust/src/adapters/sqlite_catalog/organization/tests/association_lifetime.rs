use super::*;

fn retain_favorite(catalog: &mut SqliteCatalog, ids: &[&str]) {
    let prepared = prepare(catalog, ids);
    catalog
        .set_organization_favorite("remember", &prepared, true)
        .unwrap();
}

fn timestamp(catalog: &SqliteCatalog) -> Option<i64> {
    catalog
        .connection
        .query_row(
            "SELECT MAX(unassociated_since_unix_ms) FROM organization_records",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn publication_records_first_loss_and_keeps_it_until_reassociation() {
    let (_directory, mut catalog) = fixture();
    retain_favorite(&mut catalog, &["a"]);
    let generation = selected(&catalog, "a").source_generation;
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = source_generation + 100,
               source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'
             WHERE location_id = 'a'",
            [],
        )
        .unwrap();
    super::super::publication::root(&catalog.connection, "root-a").unwrap();
    let first = timestamp(&catalog).expect("the replaced version is unassociated");
    super::super::association_lifetime::refresh_root(&catalog.connection, "root-a").unwrap();
    assert_eq!(timestamp(&catalog), Some(first));
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = ?1,
               source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'
             WHERE location_id = 'a'",
            [generation as i64],
        )
        .unwrap();
    super::super::records::publish_generation(&catalog.connection, generation as i64).unwrap();
    assert_eq!(timestamp(&catalog), None);
}

#[test]
fn losing_one_copy_preserves_the_group_and_offline_source_evidence() {
    let (_directory, mut catalog) = fixture();
    publish(&mut catalog, "a", 'a');
    publish(&mut catalog, "c", 'a');
    retain_favorite(&mut catalog, &["a"]);
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE location_id = 'a'", [])
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE library_change_root_state SET is_active = 0 WHERE root_id = 'root-c'",
            [],
        )
        .unwrap();
    super::super::association_lifetime::refresh_root(&catalog.connection, "root-a").unwrap();
    assert_eq!(timestamp(&catalog), None);
    assert_eq!(
        scalar(&catalog, "SELECT SUM(favorite) FROM organization_records"),
        1
    );
}

#[test]
fn delta_removal_lifetime_rolls_back_with_the_location_transaction() {
    let (_directory, mut catalog) = fixture();
    retain_favorite(&mut catalog, &["a"]);
    {
        let transaction = catalog.connection.unchecked_transaction().unwrap();
        let group = super::super::association_lifetime::location_group(
            &transaction,
            "scan-a",
            "root-a",
            "a",
        )
        .unwrap()
        .unwrap();
        transaction
            .execute("DELETE FROM asset_locations WHERE location_id = 'a'", [])
            .unwrap();
        super::super::association_lifetime::refresh_groups(&transaction, &[group], 1_000).unwrap();
        let first: i64 = transaction
            .query_row(
                "SELECT unassociated_since_unix_ms FROM organization_records",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(first, 1_000);
    }
    assert_eq!(timestamp(&catalog), None);
    assert!(catalog.load_active_location("a").unwrap().is_some());
}

#[test]
fn restoring_one_group_skips_associated_history_and_unrelated_unassociated_records() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (_directory, mut catalog) = fixture();
    retain_favorite(&mut catalog, &["a"]);
    let generation = selected(&catalog, "a").source_generation;
    let record: String = catalog
        .connection
        .query_row("SELECT record_id FROM organization_records", [], |row| {
            row.get(0)
        })
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "WITH RECURSIVE sequence(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50000)
         INSERT INTO organization_records
         SELECT 'associated-' || value, NULL, 'pending', 0, 0, 0, 1, 1, NULL FROM sequence;
         WITH RECURSIVE sequence(value) AS (
           VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 50000)
         INSERT INTO organization_records
         SELECT 'detached-' || value, NULL, 'pending', 0, 0, 0, 1, 1, 10 FROM sequence;
         INSERT INTO organization_record_groups
         SELECT record_id, record_id FROM organization_records WHERE record_id LIKE 'detached-%';",
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO organization_record_groups
         SELECT record_id, ?1 FROM organization_records WHERE record_id LIKE 'associated-%'",
            [&record],
        )
        .unwrap();
    for by_generation in [false, true] {
        catalog.connection.execute(
            "UPDATE organization_records SET unassociated_since_unix_ms = 20 WHERE record_id = ?1",
            [&record],
        ).unwrap();
        let ticks = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&ticks);
        catalog
            .connection
            .progress_handler(
                100,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        let result = if by_generation {
            super::super::association_lifetime::restore_generation(
                &catalog.connection,
                generation as i64,
            )
        } else {
            super::super::association_lifetime::restore_group(&catalog.connection, &record)
        };
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        result.unwrap();
        let steps = ticks.load(Ordering::Relaxed) * 100;
        assert!(
            steps < 5_000,
            "generation={by_generation}: {steps} SQLite steps"
        );
        assert_eq!(
            scalar(
                &catalog,
                "SELECT COUNT(*) FROM organization_records WHERE unassociated_since_unix_ms IS NOT NULL"
            ),
            50_000
        );
        assert_eq!(
            scalar(
                &catalog,
                "SELECT COUNT(*) FROM organization_unassociated_records WHERE group_id LIKE 'detached-%'"
            ),
            50_000
        );
    }
}
