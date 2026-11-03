use crate::domain::classification_review::{ClassificationReviewFilter, ClassificationReviewScope};
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::{GalleryQuery, GallerySortDirection, GallerySortKey};
use crate::ports::classification_review::ClassificationReviewRepository;
use crate::ports::organization_archive::OrganizationArchiveRepository;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

#[test]
fn classification_position_survives_restart_without_overwriting_r4_continuation() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut r4 = catalog
        .begin_organization_review(&GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            ..Default::default()
        })
        .unwrap();
    r4.position_sequence = 1;
    catalog
        .checkpoint_organization_review(&r4, &selected(&catalog, "b"))
        .unwrap();
    let original = catalog.load_organization_continuation(None).unwrap();
    let scope = ClassificationReviewScope {
        gallery: GalleryQuery {
            sort_key: GallerySortKey::FileName,
            sort_direction: GallerySortDirection::Ascending,
            ..ClassificationReviewScope::default().gallery
        },
        ..Default::default()
    };
    let mut session = catalog.begin_classification_review(&scope).unwrap();
    session.position_sequence = 1;
    let selection = selected(&catalog, "a");
    catalog
        .checkpoint_classification_review(&session, &selection)
        .unwrap();
    catalog
        .checkpoint_classification_review(&session, &selection)
        .unwrap();
    assert!(
        catalog
            .checkpoint_classification_review(&session, &selected(&catalog, "b"))
            .is_err()
    );
    let bookmark = catalog.load_classification_bookmark().unwrap().unwrap();
    assert_eq!(bookmark.scope, scope);
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap(),
        original
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog.load_classification_bookmark().unwrap(),
        Some(bookmark)
    );
    assert_eq!(
        catalog.load_organization_continuation(None).unwrap(),
        original
    );
}

#[test]
fn review_confirmation_keeps_all_results_but_advances_a_pending_filter() {
    for filter in [
        ClassificationReviewFilter::All,
        ClassificationReviewFilter::Pending,
    ] {
        let (_directory, mut catalog) = fixture();
        publish(&mut catalog, "a", 'a');
        let run = setup(&mut catalog);
        predict(&mut catalog, &run, "a", "prediction", "drawing");
        let mut session = catalog
            .begin_classification_review(&ClassificationReviewScope {
                review: filter,
                ..Default::default()
            })
            .unwrap();
        session.position_sequence = 1;
        catalog
            .checkpoint_classification_review(&session, &selected(&catalog, "a"))
            .unwrap();
        let prepared = prepare(&catalog, &["a"]);
        let receipt = catalog
            .change_current_classification_review(
                &id("confirm"),
                &session,
                &prepared,
                &ClassificationAction::Confirm,
            )
            .unwrap();
        assert_eq!(receipt.session.position_sequence, 2);
        assert_eq!(
            catalog
                .load_classification_bookmark()
                .unwrap()
                .unwrap()
                .after_current,
            filter == ClassificationReviewFilter::Pending
        );
        assert_eq!(
            catalog
                .change_current_classification_review(
                    &id("confirm"),
                    &session,
                    &prepared,
                    &ClassificationAction::Confirm
                )
                .unwrap(),
            receipt
        );
        assert!(
            catalog
                .checkpoint_classification_review(&session, &selected(&catalog, "a"))
                .is_err()
        );
    }
}

#[test]
fn replaced_review_session_rejects_old_position_and_category_callbacks() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut first = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    first.position_sequence = 1;
    catalog
        .checkpoint_classification_review(&first, &selected(&catalog, "a"))
        .unwrap();
    let mut second = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    second.position_sequence = 1;
    catalog.end_classification_review(&first.id).unwrap();
    catalog
        .checkpoint_classification_review(&second, &selected(&catalog, "b"))
        .unwrap();
    let bookmark = catalog.load_classification_bookmark().unwrap();
    assert!(
        catalog
            .change_current_classification_review(
                &id("late"),
                &first,
                &prepare(&catalog, &["a"]),
                &ClassificationAction::Correct(category("photo"))
            )
            .is_err()
    );
    first.position_sequence = 2;
    assert!(
        catalog
            .checkpoint_classification_review(&first, &selected(&catalog, "a"))
            .is_err()
    );
    assert_eq!(catalog.load_classification_bookmark().unwrap(), bookmark);
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Unreviewed
    );
}

#[test]
fn portable_review_remaps_order_and_source_without_restoring_a_live_session() {
    let (directory, mut source) = fixture();
    setup(&mut source);
    let scope = ClassificationReviewScope {
        gallery: GalleryQuery {
            root_id: Some("root-a".into()),
            sort_key: GallerySortKey::LibraryAdmission,
            ..ClassificationReviewScope::default().gallery
        },
        ..Default::default()
    };
    let mut session = source.begin_classification_review(&scope).unwrap();
    session.position_sequence = 1;
    source
        .checkpoint_classification_review(&session, &selected(&source, "a"))
        .unwrap();
    let before = source.load_classification_bookmark().unwrap().unwrap();
    let archive = directory.path().join("position.ame-backup");
    source
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let target_path = directory.path().join("restored.sqlite3");
    let mut target = SqliteCatalog::open(target_path.clone()).unwrap();
    super::archives::restore(&mut target, &archive, "restore").unwrap();
    let restored = target.load_classification_bookmark().unwrap().unwrap();
    assert_eq!(restored.record_id, before.record_id);
    assert_eq!(
        restored.scope.gallery.sort_key,
        GallerySortKey::LibraryAdmission
    );
    assert_eq!(
        restored.position.primary_number,
        restored.position.admission_id as i64
    );
    assert_ne!(restored.scope.gallery.root_id, before.scope.gallery.root_id);
    assert_ne!(restored.position.location_id, before.position.location_id);
    assert_eq!(
        scalar(
            &target,
            "SELECT count(*) FROM classification_review_session"
        ),
        0
    );
    assert_eq!(
        scalar(&target, "SELECT count(*) FROM organization_review_bookmark"),
        0
    );
    drop(target);
    let target = SqliteCatalog::open(target_path).unwrap();
    assert_eq!(
        target.load_classification_bookmark().unwrap(),
        Some(restored)
    );
    target
        .export_organization_archive(
            &directory.path().join("position-again.ame-backup"),
            &mut |_| true,
        )
        .unwrap();
}

#[test]
fn catalog_validation_rejects_unknown_scope_fields_and_unpaired_undo_targets() {
    let (_directory, mut catalog) = fixture();
    setup(&mut catalog);
    let mut session = catalog
        .begin_classification_review(&ClassificationReviewScope::default())
        .unwrap();
    session.position_sequence = 1;
    catalog
        .checkpoint_classification_review(&session, &selected(&catalog, "a"))
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE classification_review_bookmark SET scope = json_set(scope, '$.unknown', 1)",
            [],
        )
        .unwrap();
    assert!(super::super::validation::catalog(&catalog.connection).is_err());
    catalog
        .connection
        .execute(
            "UPDATE classification_review_bookmark SET scope = json_remove(scope, '$.unknown')",
            [],
        )
        .unwrap();
    catalog
        .change_classification(
            &id("choice"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    super::super::validation::catalog(&catalog.connection).unwrap();
    catalog
        .connection
        .execute(
            "DELETE FROM classification_operation_targets WHERE operation_id = 'choice'",
            [],
        )
        .unwrap();
    assert!(super::super::validation::catalog(&catalog.connection).is_err());
}
