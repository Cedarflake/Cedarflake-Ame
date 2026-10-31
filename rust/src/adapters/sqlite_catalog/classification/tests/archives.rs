use std::path::Path;

use crate::domain::organization_archive::OrganizationArchiveRestoreRequest;
use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::*;

pub(super) fn restore(
    catalog: &mut SqliteCatalog,
    path: &Path,
    operation: &str,
) -> Result<(), ScanError> {
    let inspection = catalog.inspect_organization_archive(path, &mut |_| true)?;
    catalog.restore_organization_archive(
        path,
        &OrganizationArchiveRestoreRequest {
            operation_id: operation.into(),
            archive_id: inspection.archive_id,
            checksum: inspection.checksum,
            expected_catalog_revision: inspection.associations.catalog_revision,
            expected_organization_revision: inspection.associations.organization_revision,
        },
        &mut |_| true,
    )?;
    Ok(())
}

#[test]
fn portable_classification_retains_human_choices_without_models_and_reexports() {
    let (directory, mut source) = fixture();
    publish(&mut source, "a", 'a');
    let run = setup(&mut source);
    predict(&mut source, &run, "a", "prediction", "drawing");
    source
        .change_classification(
            &id("confirm"),
            &prepare(&source, &["a"]),
            &ClassificationAction::Confirm,
        )
        .unwrap();
    source
        .change_classification(
            &id("defer-b"),
            &prepare(&source, &["b"]),
            &ClassificationAction::Defer,
        )
        .unwrap();
    let archive = directory.path().join("classification.ame-backup");
    source
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();

    let target_path = directory.path().join("restored.sqlite3");
    let mut target = SqliteCatalog::open(target_path.clone()).unwrap();
    restore(&mut target, &archive, "restore-classification").unwrap();
    assert_eq!(
        target.load_classification_state().unwrap().taxonomy,
        Some(taxonomy())
    );
    assert_eq!(
        scalar(&target, "SELECT count(*) FROM classification_runs"),
        0
    );
    assert_eq!(
        scalar(&target, "SELECT count(*) FROM classification_predictions"),
        0
    );
    assert_eq!(
        scalar(
            &target,
            "SELECT count(*) FROM classification_decisions WHERE category_id = 'drawing' AND choice_restored = 1"
        ),
        1
    );
    assert_eq!(
        scalar(
            &target,
            "SELECT count(*) FROM classification_decisions WHERE deferred = 1 AND review_restored = 1"
        ),
        1
    );
    assert!(
        target
            .undo_classification(&id("foreign-undo"), &id("confirm"))
            .is_err()
    );
    restore(&mut target, &archive, "repeat-restore").unwrap();
    assert_eq!(
        scalar(&target, "SELECT count(*) FROM classification_decisions"),
        2
    );
    drop(target);
    let target = SqliteCatalog::open(target_path).unwrap();
    let reexport = directory.path().join("reexport.ame-backup");
    target
        .export_organization_archive(&reexport, &mut |_| true)
        .unwrap();
    let mut third = SqliteCatalog::open(directory.path().join("third.sqlite3")).unwrap();
    restore(&mut third, &reexport, "restore-again").unwrap();
    assert_eq!(
        scalar(
            &third,
            "SELECT count(*) FROM classification_decisions WHERE category_id = 'drawing'"
        ),
        1
    );
    assert!(
        third
            .undo_classification(&id("foreign-undo"), &id("confirm"))
            .is_err()
    );
}

#[test]
fn restore_preserves_newer_local_choice_and_field_undo_authority() {
    let (directory, mut catalog) = fixture();
    setup(&mut catalog);
    catalog
        .change_classification(
            &id("first"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let archive = directory.path().join("older.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    catalog
        .change_classification(
            &id("newer"),
            &prepare(&catalog, &["a"]),
            &ClassificationAction::Correct(category("drawing")),
        )
        .unwrap();
    restore(&mut catalog, &archive, "restore-old").unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].human,
        HumanClassification::Chosen {
            category: category("drawing"),
            origin: HumanDecisionOrigin::Local,
        }
    );
    catalog
        .undo_classification(&id("undo-newer"), &id("newer"))
        .unwrap();
    assert_eq!(
        prepare(&catalog, &["a"]).groups[0].effective().category,
        Some(category("photo"))
    );
    let revision = catalog.load_classification_state().unwrap().revision;
    drop(catalog);
    let catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    assert_eq!(
        catalog.load_classification_state().unwrap().revision,
        revision
    );
}

#[test]
fn failed_classification_restore_rolls_back_r4_and_r5_together() {
    let (directory, mut source) = fixture();
    setup(&mut source);
    source
        .change_classification(
            &id("choose"),
            &prepare(&source, &["a"]),
            &ClassificationAction::Correct(category("photo")),
        )
        .unwrap();
    let archive = directory.path().join("failure.ame-backup");
    source
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let target_path = directory.path().join("target.sqlite3");
    let target = SqliteCatalog::open(target_path.clone()).unwrap();
    target
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_classification_restore
             BEFORE INSERT ON classification_decisions
             BEGIN SELECT RAISE(ABORT, 'injected restore failure'); END;",
        )
        .unwrap();
    drop(target);
    let mut target = SqliteCatalog::open(target_path.clone()).unwrap();
    let failure = restore(&mut target, &archive, "fail").unwrap_err();
    assert!(failure.message.contains("injected restore failure"));
    for table in [
        "classification_categories",
        "classification_operations",
        "classification_decisions",
        "organization_records",
        "organization_operations",
    ] {
        assert_eq!(
            scalar(&target, &format!("SELECT count(*) FROM {table}")),
            0,
            "{table}"
        );
    }
    assert_eq!(target.load_classification_state().unwrap().revision, 0);
    target
        .connection
        .execute_batch("DROP TRIGGER fail_classification_restore")
        .unwrap();
    drop(target);
    let mut target = SqliteCatalog::open(target_path).unwrap();
    restore(&mut target, &archive, "retry").unwrap();
    assert_eq!(
        scalar(&target, "SELECT count(*) FROM classification_decisions"),
        1
    );
}
