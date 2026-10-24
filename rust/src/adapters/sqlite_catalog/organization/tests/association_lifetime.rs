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
