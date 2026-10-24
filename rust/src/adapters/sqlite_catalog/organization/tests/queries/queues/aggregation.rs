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
