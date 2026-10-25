use super::*;

#[test]
fn an_identity_preserving_location_change_keeps_the_saved_group() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "b");
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET location_id = 'renamed-b', relative_path = 'renamed-b.png'
             WHERE location_id = 'b'",
            [],
        )
        .unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("renamed-b"));
}

#[test]
fn advancing_past_a_deferred_group_cannot_return_its_new_representative() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    change(
        &mut catalog,
        "defer",
        &["a", "b"],
        OrganizationAction::Defer,
    );
    let query = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Deferred),
        ..pending()
    };
    let session = catalog.begin_organization_review(&query).unwrap();
    let position = checkpoint(&mut catalog, &session, 1, "a");
    let prepared = prepare(&catalog, &["a"]);
    let receipt = catalog
        .change_current_organization_review(
            "defer-again",
            &position,
            &prepared,
            &OrganizationAction::Defer,
        )
        .unwrap();
    assert_eq!(receipt.operation.changed_groups, 0);
    let preferred = selected(&catalog, "c");
    catalog
        .set_duplicate_preference(
            &ExactDuplicateReviewKey {
                digest: "a".repeat(64),
                byte_length: 20,
            },
            Some(&ExactDuplicatePreference {
                location_id: "c".into(),
                asset_id: preferred.asset_id,
                source_generation: preferred.source_generation,
            }),
        )
        .unwrap();
    let result = catalog.load_organization_continuation(None).unwrap();
    assert_eq!(result.status, Status::EndOfQueue);
    assert!(result.target.is_none());
}

#[test]
fn parent_consolidation_preserves_the_saved_physical_folder_scope() {
    let (_directory, mut catalog) = fixture();
    let query = GalleryQuery {
        root_id: Some("root-a".into()),
        ..pending()
    };
    let session = catalog.begin_organization_review(&query).unwrap();
    checkpoint(&mut catalog, &session, 1, "b");
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "parent-scan",
        "parent",
        "C:\\",
        &[("parent-image", None, 0)],
    );
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    crate::adapters::sqlite_catalog::organization::lifecycle::transfer_source(
        &transaction,
        "root-a",
        "parent",
    )
    .unwrap();
    transaction
        .execute(
            "UPDATE asset_locations SET root_id = 'parent', scan_id = 'parent-scan',
           relative_path = 'Fixture/b.png', parent_relative_path = 'Fixture'
         WHERE location_id = 'b'",
            [],
        )
        .unwrap();
    transaction.commit().unwrap();
    catalog.unregister_root("root-a").unwrap();
    let continuation = catalog.load_organization_continuation(None).unwrap();
    let scope = continuation.query.unwrap();
    assert_eq!(scope.root_id.as_deref(), Some("parent"));
    assert_eq!(scope.folder_relative_path.as_deref(), Some("Fixture"));
    assert_eq!(continuation.target.unwrap().location_id, "b");
}
