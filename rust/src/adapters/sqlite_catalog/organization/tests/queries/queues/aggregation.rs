use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use super::*;

#[test]
fn scoped_counts_group_members_without_resolving_preferred_copies() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    change(&mut catalog, "defer", &["c"], OrganizationAction::Defer);
    change(&mut catalog, "ignore", &["b"], OrganizationAction::Ignore);
    let prepared = prepare(&catalog, &["c"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
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
    catalog
        .connection
        .authorizer(Some(|context: AuthContext<'_>| {
            if matches!(
                context.action,
                AuthAction::Read {
                    table_name: "exact_duplicate_decisions",
                    ..
                }
            ) {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        }))
        .unwrap();
    let all = OrganizationCounts {
        total: 2,
        deferred: 1,
        ignored: 1,
        ..OrganizationCounts::default()
    };
    let one = OrganizationCounts {
        total: 1,
        deferred: 1,
        ..OrganizationCounts::default()
    };
    let pending = queue(OrganizationQueue::Pending);
    let transaction = catalog.connection.transaction().unwrap();
    for (query, expected) in [
        (pending.clone(), all.clone()),
        (
            GalleryQuery {
                root_id: Some("root-a".into()),
                ..pending.clone()
            },
            all,
        ),
        (
            GalleryQuery {
                root_id: Some("root-c".into()),
                ..pending.clone()
            },
            one.clone(),
        ),
        (
            GalleryQuery {
                collection: GalleryCollection::Favorites,
                ..pending.clone()
            },
            one,
        ),
        (
            GalleryQuery {
                search_text: "not-present".into(),
                ..pending
            },
            OrganizationCounts::default(),
        ),
    ] {
        assert_eq!(
            crate::adapters::sqlite_catalog::gallery::organization_queue::load_counts(
                &transaction,
                &query,
            )
            .unwrap(),
            expected,
        );
    }
    transaction.commit().unwrap();
}

#[test]
fn unbound_copy_counts_keep_collection_scope_and_reject_changed_content() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let album = create_album(&mut catalog, "Organized");
    assign(&mut catalog, "assign", &["a"], &album);
    let prepared = prepare(&catalog, &["a"]);
    catalog
        .set_organization_favorite("favorite", &prepared, true)
        .unwrap();
    let copy = selected(&catalog, "c");
    catalog
        .connection
        .execute(
            "DELETE FROM organization_bindings WHERE asset_id = ?1 AND source_generation = ?2",
            params![
                copy.asset_id,
                i64::try_from(copy.source_generation).unwrap()
            ],
        )
        .unwrap();
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET relative_path = 'Folder/c.png',
             parent_relative_path = 'Folder' WHERE location_id = 'c';",
        )
        .unwrap();
    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        folder_relative_path: Some("Folder".into()),
        search_text: "c.png".into(),
        ..queue(OrganizationQueue::Pending)
    };
    for changed in [false, true] {
        if changed {
            catalog
                .connection
                .execute(
                    "UPDATE asset_locations SET file_size = 21 WHERE location_id = 'c'",
                    [],
                )
                .unwrap();
        }
        for collection in [
            GalleryCollection::Library,
            GalleryCollection::Favorites,
            GalleryCollection::Album {
                album_id: album.clone(),
            },
        ] {
            let visible = !changed || collection == GalleryCollection::Library;
            let query = GalleryQuery {
                collection,
                ..scoped.clone()
            };
            assert_eq!(
                counts(&mut catalog, &query),
                OrganizationCounts {
                    total: u64::from(visible),
                    pending: u64::from(visible && changed),
                    organized: u64::from(visible && !changed),
                    ..OrganizationCounts::default()
                }
            );
        }
    }
}
