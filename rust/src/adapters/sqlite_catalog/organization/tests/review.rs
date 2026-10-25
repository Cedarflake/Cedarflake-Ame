use crate::domain::GalleryQuery;
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::organization_review::{
    OrganizationContinuationStatus as Status, OrganizationReviewSession,
};
use crate::ports::organization_review::OrganizationReviewRepository;

use super::queries::{evolution::invalidate, publish_batch};
use super::transitions::{change, create_album};
use super::*;

mod lifecycle;
mod sorting;

fn pending() -> GalleryQuery {
    GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        sort_key: crate::domain::GallerySortKey::LibraryAdmission,
        ..GalleryQuery::default()
    }
}

fn checkpoint(
    catalog: &mut SqliteCatalog,
    session: &OrganizationReviewSession,
    sequence: u64,
    id: &str,
) -> OrganizationReviewSession {
    let requested = OrganizationReviewSession {
        session_id: session.session_id.clone(),
        position_sequence: sequence,
    };
    catalog
        .checkpoint_organization_review(&requested, &selected(catalog, id))
        .unwrap()
}

fn target(catalog: &SqliteCatalog) -> Option<String> {
    catalog
        .load_organization_continuation(None)
        .unwrap()
        .target
        .map(|value| value.location_id)
}

#[test]
fn mode_entry_does_not_create_activity_or_replace_saved_work() {
    let (directory, mut catalog) = fixture();
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::NoActivity
    );
    let session = catalog.begin_organization_review(&pending()).unwrap();
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::NoActivity
    );
    checkpoint(&mut catalog, &session, 1, "b");
    let deferred = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Deferred),
        ..pending()
    };
    catalog.begin_organization_review(&deferred).unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let reopened = catalog.begin_organization_review(&pending()).unwrap();
    assert_ne!(reopened.session_id, session.session_id);
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE disposition = 'pending' AND favorite = 0"
        ),
        1
    );
}

#[test]
fn new_arrivals_do_not_replace_the_anchor_or_the_next_item_after_ordinary_work() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "b");
    crate::adapters::sqlite_catalog::tests::publish_gallery_fixture(
        &mut catalog,
        "new-scan",
        "new-root",
        "E:\\New",
        &[("new", None, 0)],
    );
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    let album = create_album(&mut catalog, "Ordinary");
    change(
        &mut catalog,
        "ordinary",
        &["b"],
        OrganizationAction::AssignAlbum { album_id: album },
    );
    assert_eq!(target(&catalog).as_deref(), Some("a"));
    let favorite = prepare(&catalog, &["new"]);
    catalog
        .set_organization_favorite("favorite", &favorite, true)
        .unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("a"));
}

#[test]
fn late_positions_old_sessions_and_closed_sessions_cannot_replace_the_bookmark() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    let second = checkpoint(&mut catalog, &session, 2, "c");
    assert_eq!(
        catalog
            .checkpoint_organization_review(&second, &selected(&catalog, "c"))
            .unwrap(),
        second
    );
    let first = OrganizationReviewSession {
        position_sequence: 1,
        ..session.clone()
    };
    assert_eq!(
        catalog
            .checkpoint_organization_review(&first, &selected(&catalog, "b"))
            .unwrap_err()
            .code,
        "organization_review_changed"
    );
    assert!(
        catalog
            .checkpoint_organization_review(&second, &selected(&catalog, "b"))
            .is_err()
    );
    let next = catalog.begin_organization_review(&pending()).unwrap();
    catalog
        .end_organization_review(&session.session_id)
        .unwrap();
    checkpoint(&mut catalog, &next, 1, "b");
    let old = OrganizationReviewSession {
        position_sequence: 3,
        ..session
    };
    assert!(
        catalog
            .checkpoint_organization_review(&old, &selected(&catalog, "a"))
            .is_err()
    );
    catalog.end_organization_review(&next.session_id).unwrap();
    assert!(
        catalog
            .checkpoint_organization_review(&next, &selected(&catalog, "a"))
            .is_err()
    );
    assert_eq!(target(&catalog).as_deref(), Some("b"));
}

#[test]
fn failed_progress_save_rolls_back_the_decision_and_retry_does_not_rewind_later_work() {
    let (directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    let position = checkpoint(&mut catalog, &session, 1, "b");
    let prepared = prepare(&catalog, &["b"]);
    catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_review BEFORE UPDATE ON organization_review_bookmark
             BEGIN SELECT RAISE(ABORT, 'injected bookmark failure'); END;",
        )
        .unwrap();
    assert!(
        catalog
            .change_current_organization_review(
                "review-ignore",
                &position,
                &prepared,
                &OrganizationAction::Ignore
            )
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE disposition = 'ignored'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id = 'review-ignore'"
        ),
        0
    );
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_review")
        .unwrap();
    let receipt = catalog
        .change_current_organization_review(
            "review-ignore",
            &position,
            &prepared,
            &OrganizationAction::Ignore,
        )
        .unwrap();
    assert_eq!(receipt.session.position_sequence, 2);
    assert_eq!(target(&catalog).as_deref(), Some("a"));
    drop(catalog);
    let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("a"));
    let newer = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &newer, 1, "c");
    assert_eq!(
        catalog
            .change_current_organization_review(
                "review-ignore",
                &position,
                &prepared,
                &OrganizationAction::Ignore
            )
            .unwrap(),
        receipt
    );
    assert_eq!(target(&catalog).as_deref(), Some("c"));
    assert_eq!(
        catalog
            .change_current_organization_review(
                "review-ignore",
                &position,
                &prepared,
                &OrganizationAction::Defer
            )
            .unwrap_err()
            .code,
        "organization_operation_conflict"
    );
}

#[test]
fn stale_or_different_group_commands_do_not_advance_current_work() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    let first = checkpoint(&mut catalog, &session, 1, "c");
    let selected_c = prepare(&catalog, &["c"]);
    checkpoint(&mut catalog, &session, 2, "b");
    assert!(
        catalog
            .change_current_organization_review(
                "late",
                &first,
                &selected_c,
                &OrganizationAction::Ignore
            )
            .is_err()
    );
    let second = OrganizationReviewSession {
        position_sequence: 2,
        ..session
    };
    assert!(
        catalog
            .change_current_organization_review(
                "other",
                &second,
                &selected_c,
                &OrganizationAction::Ignore
            )
            .is_err()
    );
    assert_eq!(target(&catalog).as_deref(), Some("b"));
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_records WHERE disposition <> 'pending'"
        ),
        0
    );
}

#[test]
fn late_grouping_representatives_and_cache_cleanup_retain_the_logical_anchor() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "c");
    publish_batch(&mut catalog, &["a", "c"], 'a');
    assert_eq!(target(&catalog).as_deref(), Some("a"));
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
    assert_eq!(target(&catalog).as_deref(), Some("c"));
    catalog
        .connection
        .execute_batch(
            "DELETE FROM gallery_content_fingerprints; DELETE FROM content_fingerprints;",
        )
        .unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("c"));
}

#[test]
fn independent_changed_content_cannot_inherit_the_saved_anchor() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["c"], 'a');
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "c");
    invalidate(&mut catalog, "c");
    publish_batch(&mut catalog, &["c"], 'b');
    let result = catalog.load_organization_continuation(None).unwrap();
    assert_eq!(result.status, Status::AnchorUnavailable);
    assert_eq!(result.target.unwrap().location_id, "b");
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_review_bookmark"
        ),
        1
    );
}

#[test]
fn unassociated_work_is_retained_and_empty_scopes_differ_from_exhausted_suffixes() {
    let (_directory, mut catalog) = fixture();
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "c");
    catalog
        .connection
        .execute("DELETE FROM asset_locations WHERE location_id = 'c'", [])
        .unwrap();
    let result = catalog.load_organization_continuation(None).unwrap();
    assert_eq!(result.status, Status::AnchorUnavailable);
    assert_eq!(result.target.unwrap().location_id, "b");
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        1
    );
    checkpoint(&mut catalog, &session, 2, "a");
    change(
        &mut catalog,
        "ignore-oldest",
        &["a"],
        OrganizationAction::Ignore,
    );
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::EndOfQueue
    );
    change(
        &mut catalog,
        "ignore-remaining",
        &["b"],
        OrganizationAction::Ignore,
    );
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::EmptyScope
    );
}

#[test]
fn scoped_checkpoints_reject_hidden_items_without_materializing_or_advancing_them() {
    let (_directory, mut catalog) = fixture();
    let scoped = GalleryQuery {
        root_id: Some("root-c".into()),
        ..pending()
    };
    let session = catalog.begin_organization_review(&scoped).unwrap();
    let first = OrganizationReviewSession {
        position_sequence: 1,
        ..session.clone()
    };
    assert!(
        catalog
            .checkpoint_organization_review(&first, &selected(&catalog, "a"))
            .is_err()
    );
    assert_eq!(
        scalar(&catalog, "SELECT COUNT(*) FROM organization_records"),
        0
    );
    checkpoint(&mut catalog, &session, 1, "c");
    catalog.unregister_root("root-c").unwrap();
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::NoActivity
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT COUNT(*) FROM organization_review_scopes WHERE root_id = 'root-c'"
        ),
        0
    );
}

#[test]
fn removing_one_source_preserves_a_global_shared_group_bookmark() {
    let (_directory, mut catalog) = fixture();
    publish_batch(&mut catalog, &["a", "c"], 'a');
    let session = catalog.begin_organization_review(&pending()).unwrap();
    checkpoint(&mut catalog, &session, 1, "c");
    catalog.unregister_root("root-c").unwrap();
    assert_eq!(target(&catalog).as_deref(), Some("a"));
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap().status,
        Status::NoActivity
    );
}
