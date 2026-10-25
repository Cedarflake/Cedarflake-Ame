use crate::adapters::sqlite_catalog::{
    exact_duplicates::read_unsigned, organization, tests::publish_gallery_fixture,
};
use crate::domain::organization::{OrganizationAction, OrganizationSelection};
use crate::domain::organization_queue::OrganizationQueue;
use crate::domain::{GalleryQuery, GallerySortKey};
use crate::ports::CatalogRepository;
use crate::ports::organization::OrganizationRepository;
use crate::ports::organization_review::OrganizationReviewRepository;

use super::*;

fn source() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, mut catalog) = fixture();
    publish_gallery_fixture(
        &mut catalog,
        "scan-b",
        "root-b",
        "C:\\UnavailableQueueArchive",
        &[("b", None, 2), ("c", None, 3)],
    );
    for (name, native) in [("a", 1), ("b", 2), ("c", 3)] {
        identity(&catalog, name, native);
    }
    catalog.connection.execute_batch(
        "UPDATE organization_bindings SET file_identity_scheme = 'windows-file-id-128-v1',
         file_identity_value = printf('%016x:%032x', 1, 1), source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000007';",
    ).unwrap();
    (directory, catalog)
}

fn identity(catalog: &SqliteCatalog, location: &str, native: i64) {
    catalog.connection.execute(
        "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
           file_identity_value = printf('%016x:%032x', 1, ?2), source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000007'
         WHERE location_id = ?1", params![location, native],
    ).unwrap();
}

fn import(catalog: &mut SqliteCatalog, name: &str, native: i64) {
    let location = format!("current-{name}");
    let root = format!("current-root-{name}");
    publish_gallery_fixture(
        catalog,
        &format!("current-scan-{name}"),
        &root,
        &format!("C:\\UnavailableCurrentQueue\\{name}"),
        &[(&location, None, native)],
    );
    identity(catalog, &location, native);
    let transaction = catalog.connection.unchecked_transaction().unwrap();
    organization::publication::root(&transaction, &root).unwrap();
    transaction.commit().unwrap();
}

fn pending(catalog: &mut SqliteCatalog) -> Vec<String> {
    catalog
        .load_snapshot(
            20,
            &GalleryQuery {
                organization_queue: Some(OrganizationQueue::Pending),
                sort_key: GallerySortKey::LibraryAdmission,
                ..GalleryQuery::default()
            },
            "restored-queue",
            None,
            None,
            None,
        )
        .unwrap()
        .assets
        .into_iter()
        .map(|asset| asset.location_id)
        .collect()
}

#[test]
fn organization_archive_restores_selected_sort_and_missing_anchor_boundary() {
    let (directory, source) = source();
    source.connection.execute_batch(
        "UPDATE organization_review_sort SET sort_key = 'file_name', sort_direction = 'ascending';
         UPDATE organization_review_cursor SET primary_text = 'retained-name', primary_number = 0;",
    ).unwrap();
    let target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    let restored: (String, String, String, i64) = target
        .connection
        .query_row(
            "SELECT sort_key, sort_direction, primary_text, primary_number
         FROM organization_review_sort, organization_review_cursor WHERE scope_slot = 'bookmark'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        restored,
        (
            "file_name".into(),
            "ascending".into(),
            "retained-name".into(),
            0
        )
    );
    detach(&target, staged);
    drop(target);
    SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
}

pub(super) fn detach(catalog: &SqliteCatalog, staged: staging::StagedArchive) {
    catalog
        .connection
        .execute_batch("DETACH DATABASE recovery")
        .unwrap();
    drop(staged);
}

#[test]
fn organization_archive_queue_recovers_order_in_a_populated_catalog_and_keeps_new_arrivals_first() {
    let (directory, source) = source();
    let path = directory.path().join("target.sqlite3");
    let mut target = SqliteCatalog::open(path.clone()).unwrap();
    import(&mut target, "c", 3);
    import(&mut target, "b", 2);
    assert_eq!(pending(&mut target), ["current-b", "current-c"]);
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(pending(&mut target), ["current-c", "current-b"]);
    assert_eq!(count(&target.connection, "organization_review_session"), 0);
    let sequence = target
        .connection
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    apply(&target, &archive).unwrap();
    assert_eq!(
        sequence,
        target
            .connection
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'organization_admissions'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap()
    );
    import(&mut target, "d", 4);
    assert_eq!(
        pending(&mut target),
        ["current-d", "current-c", "current-b"]
    );
    detach(&target, staged);
    drop(target);
    let mut reopened = SqliteCatalog::open(path).unwrap();
    assert_eq!(
        pending(&mut reopened),
        ["current-d", "current-c", "current-b"]
    );
}

#[test]
fn organization_archive_queue_retains_unmatched_order_through_reexport_and_later_import() {
    let (directory, source) = source();
    let mut target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(
        count(&target.connection, "organization_recovery_admissions"),
        3
    );
    assert_eq!(
        count(&target.connection, "organization_recovery_admission_links"),
        0
    );
    detach(&target, staged);
    let exported = directory.path().join("roundtrip.ame-backup");
    target
        .export_organization_archive(&exported, &mut |_| true)
        .unwrap();
    let inspected = target
        .inspect_organization_archive(&exported, &mut |_| true)
        .unwrap();
    assert!(inspected.has_continuation);
    let roundtrip = staging::read(&exported, directory.path(), &mut |_| true).unwrap();
    let connection = Connection::open(roundtrip.path()).unwrap();
    assert_eq!(count(&connection, "archive_admission"), 3);
    assert_eq!(count(&connection, "archive_version_evidence"), 3);
    drop(connection);
    drop(roundtrip);
    import(&mut target, "c", 3);
    import(&mut target, "b", 2);
    assert_eq!(pending(&mut target), ["current-c", "current-b"]);
}

#[test]
fn organization_archive_queue_preserves_a_current_bookmark_and_active_session() {
    let (directory, source) = source();
    let (_target_directory, target) = fixture();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    let bookmark: (String, String, String, i64) = target.connection.query_row(
        "SELECT record_id, root_id, location_id, admission_id FROM organization_review_bookmark", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).unwrap();
    assert_eq!(
        bookmark,
        ("record-a".into(), "root-a".into(), "a".into(), 4)
    );
    assert_eq!(count(&target.connection, "organization_review_session"), 1);
    apply(&target, &archive).unwrap();
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT admission_id FROM organization_review_bookmark",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        4
    );
    detach(&target, staged);
}

#[test]
fn organization_archive_queue_continues_after_the_same_group_when_restoration_reorders_it() {
    let (directory, source) = source();
    source.connection.execute_batch(
        "DELETE FROM organization_conflicts; UPDATE organization_records SET disposition = 'pending';",
    ).unwrap();
    let path = directory.path().join("target.sqlite3");
    let mut target = SqliteCatalog::open(path.clone()).unwrap();
    import(&mut target, "c", 3);
    import(&mut target, "a", 1);
    import(&mut target, "b", 2);
    let selected = target.connection.query_row(
        "SELECT asset_id, location_id, source_generation FROM asset_locations WHERE location_id = 'current-b'", [],
        |row| Ok(OrganizationSelection { asset_id: row.get(0)?, location_id: row.get(1)?, source_generation: read_unsigned(row, 2)? }),
    ).unwrap();
    let mut session = target
        .begin_organization_review(&GalleryQuery {
            organization_queue: Some(OrganizationQueue::Pending),
            sort_key: GallerySortKey::LibraryAdmission,
            ..GalleryQuery::default()
        })
        .unwrap();
    session.position_sequence = 1;
    session = target
        .checkpoint_organization_review(&session, &selected)
        .unwrap();
    let prepared = target.prepare_organization_selection(&[selected]).unwrap();
    target
        .change_current_organization_review(
            "ignore-bookmarked",
            &session,
            &prepared,
            &OrganizationAction::Ignore,
        )
        .unwrap();
    assert_eq!(
        target
            .load_organization_continuation(None)
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "current-a"
    );
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(pending(&mut target), ["current-c", "current-a"]);
    assert_eq!(
        target
            .load_organization_continuation(None)
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "current-a"
    );
    detach(&target, staged);
    drop(target);
    let reopened = SqliteCatalog::open(path).unwrap();
    assert_eq!(
        reopened
            .load_organization_continuation(None)
            .unwrap()
            .target
            .unwrap()
            .location_id,
        "current-a"
    );
}

#[test]
fn organization_archive_queue_does_not_resurrect_removed_sources_until_explicit_restore() {
    let (directory, source) = source();
    let mut target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    import(&mut target, "c", 3);
    import(&mut target, "b", 2);
    assert!(target.unregister_root("current-root-b").unwrap());
    assert_eq!(
        count(&target.connection, "organization_recovery_admissions"),
        2
    );
    import(&mut target, "new-b", 2);
    assert_eq!(pending(&mut target), ["current-new-b", "current-c"]);
    apply(&target, &archive).unwrap();
    assert_eq!(pending(&mut target), ["current-c", "current-new-b"]);
    detach(&target, staged);
}

#[test]
fn organization_archive_queue_shift_failure_rolls_back_existing_order_and_bookmark() {
    let (directory, source) = source();
    let (_target_directory, target) = fixture();
    let (archive, _staged) = attach_archive(&target, &source, directory.path());
    target
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_queue BEFORE INSERT ON organization_recovery_admission_sources
         BEGIN SELECT RAISE(ABORT, 'injected queue failure'); END;",
        )
        .unwrap();
    assert!(apply(&target, &archive).is_err());
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT admission_id FROM organization_review_bookmark",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        target
            .connection
            .query_row(
                "SELECT admission_id FROM organization_admissions",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        count(&target.connection, "organization_recovery_admissions"),
        0
    );
    assert_eq!(
        count(&target.connection, "organization_recovery_versions"),
        0
    );
}
