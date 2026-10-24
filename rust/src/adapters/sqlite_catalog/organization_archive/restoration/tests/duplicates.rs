use crate::adapters::sqlite_catalog::{gallery, organization, tests::publish_gallery_fixture};
use crate::ports::CatalogRepository;
use crate::ports::organization_cleanup::OrganizationCleanupRepository;

use super::*;

fn native(catalog: &SqliteCatalog) {
    catalog
        .connection
        .execute_batch(
            "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
           file_identity_value = '0000000000000001:00000000000000000000000000000001',
           source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000007';",
        )
        .unwrap();
}

fn content(catalog: &SqliteCatalog, digit: char, published: bool) {
    catalog.connection.execute_batch(
        "INSERT OR IGNORE INTO exact_analysis_runs VALUES ('restore-content', 'blake3-256-full-v1', 'fixture', '1', '{}', 1);
         DELETE FROM gallery_content_fingerprints;
         DELETE FROM content_fingerprints;",
    ).unwrap();
    catalog.connection.execute(
        "INSERT INTO content_fingerprints
         SELECT DISTINCT source_generation, 'blake3-256-full-v1', ?1, file_size,
           file_identity_scheme, file_identity_value, source_revision_token, 'restore-content' FROM asset_locations",
        [digit.to_string().repeat(64)],
    ).unwrap();
    if published {
        catalog.connection.execute_batch(
            "INSERT INTO gallery_content_fingerprints SELECT source_generation, scheme FROM content_fingerprints",
        ).unwrap();
    }
}

fn source() -> (tempfile::TempDir, SqliteCatalog) {
    let (directory, catalog) = fixture();
    native(&catalog);
    content(&catalog, '0', true);
    catalog
        .connection
        .execute_batch(
            "UPDATE exact_duplicate_decisions SET preferred_asset_id = (
           SELECT asset_id FROM asset_locations WHERE location_id = 'a');",
        )
        .unwrap();
    (directory, catalog)
}

fn target(directory: &std::path::Path, aliases: bool) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(directory.join("target.sqlite3")).unwrap();
    let locations = if aliases {
        vec![("renamed", None, 1), ("alias", None, 2)]
    } else {
        vec![("renamed", None, 1)]
    };
    publish_gallery_fixture(
        &mut catalog,
        "current-scan",
        "current-root",
        "D:\\UnavailableRestoredCopies",
        &locations,
    );
    catalog
}

#[test]
fn organization_archive_duplicate_restores_a_proven_copy_and_survives_reexport() {
    let (directory, source) = source();
    let target = target(directory.path(), false);
    native(&target);
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    let choice: (String, String, bool) = target.connection.query_row(
        "SELECT preferred_location_id, preferred_asset_id, ignored FROM exact_duplicate_decisions", [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(
        choice,
        ("renamed".into(), "asset-current-scan-renamed".into(), true)
    );
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        0
    );
    queue::detach(&target, staged);
    content(&target, '0', true);
    let path = directory.path().join("roundtrip.ame-backup");
    target
        .export_organization_archive(&path, &mut |_| true)
        .unwrap();
    let inspection = target
        .inspect_organization_archive(&path, &mut |_| true)
        .unwrap();
    assert_eq!(inspection.associations.resolved_preferred_copies, 1);
    let reopened = staging::read(&path, directory.path(), &mut |_| true).unwrap();
    let staged_connection = Connection::open(reopened.path()).unwrap();
    assert_eq!(count(&staged_connection, "archive_version_evidence"), 2);
}

#[test]
fn organization_archive_duplicate_retains_ambiguous_aliases_and_contrary_versions() {
    for scenario in [
        "current-alias",
        "archived-alias",
        "changed-content",
        "changed-version",
    ] {
        let (directory, mut source) = source();
        if scenario == "archived-alias" {
            publish_gallery_fixture(
                &mut source,
                "alias-scan",
                "alias-root",
                "C:\\UnavailableArchivedAlias",
                &[("old-alias", None, 1)],
            );
            native(&source);
            content(&source, '0', true);
        }
        let target = target(directory.path(), scenario == "current-alias");
        native(&target);
        if scenario == "changed-content" {
            content(&target, '1', false);
        }
        if scenario == "changed-version" {
            target.connection.execute_batch(
                "UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000008'",
            ).unwrap();
        }
        let (archive, staged) = attach_archive(&target, &source, directory.path());
        apply(&target, &archive).unwrap();
        assert_eq!(
            count(&target.connection, "exact_duplicate_decisions"),
            0,
            "{scenario}"
        );
        assert_eq!(
            count(
                &target.connection,
                "organization_recovery_duplicate_decisions"
            ),
            1,
            "{scenario}"
        );
        queue::detach(&target, staged);
        let path = directory.path().join("unresolved.ame-backup");
        target
            .export_organization_archive(&path, &mut |_| true)
            .unwrap();
        assert_eq!(
            target
                .inspect_organization_archive(&path, &mut |_| true)
                .unwrap()
                .duplicate_choices,
            1
        );
    }
}

#[test]
fn organization_archive_duplicate_preserves_the_current_choice() {
    let (directory, source) = source();
    let target = target(directory.path(), false);
    native(&target);
    target
        .connection
        .execute_batch(
            "INSERT INTO exact_duplicate_decisions
         VALUES ('blake3-256-full-v1', printf('%064d', 0), 20, NULL, NULL, 0, 4);",
        )
        .unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert!(
        !target
            .connection
            .query_row("SELECT ignored FROM exact_duplicate_decisions", [], |row| {
                row.get::<_, bool>(0)
            },)
            .unwrap()
    );
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        0
    );
    queue::detach(&target, staged);
}

#[test]
fn organization_archive_duplicate_publishes_deferred_ignore_with_terminal_content_without_r4_records()
 {
    let (directory, source) = source();
    source.connection.execute_batch(
        "UPDATE exact_duplicate_decisions SET preferred_location_id = NULL, preferred_asset_id = NULL;",
    ).unwrap();
    let target = target(directory.path(), false);
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(count(&target.connection, "exact_duplicate_decisions"), 0);
    target
        .connection
        .execute_batch("DELETE FROM organization_records")
        .unwrap();
    content(&target, '0', false);
    let transaction = target.connection.unchecked_transaction().unwrap();
    gallery::duplicates::publish_completed_evidence(&transaction).unwrap();
    transaction.commit().unwrap();
    assert_eq!(count(&target.connection, "exact_duplicate_decisions"), 1);
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        0
    );
    queue::detach(&target, staged);
}

#[test]
fn organization_archive_duplicate_forgets_pending_choices_on_explicit_source_removal() {
    let (directory, source) = source();
    let mut target = target(directory.path(), true);
    native(&target);
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        1
    );
    queue::detach(&target, staged);
    target.unregister_root("current-root").unwrap();
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        0
    );
    publish_gallery_fixture(
        &mut target,
        "next-scan",
        "next-root",
        "D:\\UnavailableRestoredCopies",
        &[("renamed", None, 3)],
    );
    native(&target);
    let transaction = target.connection.unchecked_transaction().unwrap();
    organization::publication::root(&transaction, "next-root").unwrap();
    transaction.commit().unwrap();
    assert_eq!(count(&target.connection, "exact_duplicate_decisions"), 0);
}

#[test]
fn cleanup_can_retire_an_ambiguous_preferred_copy_despite_matching_content() {
    let (directory, source) = source();
    let mut target = target(directory.path(), true);
    native(&target);
    content(&target, '0', true);
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    queue::detach(&target, staged);
    let inspection = target
        .inspect_organization_cleanup(None, &mut |_| true)
        .unwrap();
    assert_eq!(inspection.duplicate_choices, 1);
    target
        .apply_organization_cleanup(
            &crate::domain::organization_cleanup::OrganizationCleanupRequest {
                operation_id: "cleanup-ambiguous-copy".into(),
                inspection,
            },
            &mut |_| true,
        )
        .unwrap();
    assert_eq!(
        count(
            &target.connection,
            "organization_recovery_duplicate_decisions"
        ),
        0
    );
    assert_eq!(count(&target.connection, "asset_locations"), 2);
    publish_gallery_fixture(
        &mut target,
        "unique-scan",
        "current-root",
        "D:\\UnavailableRestoredCopies",
        &[("renamed", None, 3)],
    );
    native(&target);
    let transaction = target.connection.unchecked_transaction().unwrap();
    organization::publication::root(&transaction, "current-root").unwrap();
    transaction.commit().unwrap();
    assert_eq!(count(&target.connection, "exact_duplicate_decisions"), 0);
}

#[test]
fn cleanup_preserves_resolved_choices_and_choices_owned_by_unavailable_sources() {
    for unavailable in [false, true] {
        let (directory, source) = source();
        let mut target = target(directory.path(), unavailable);
        native(&target);
        let (archive, staged) = attach_archive(&target, &source, directory.path());
        apply(&target, &archive).unwrap();
        queue::detach(&target, staged);
        if unavailable {
            target
                .connection
                .execute_batch("UPDATE library_change_root_state SET is_active = 0;")
                .unwrap();
        }
        let inspection = target
            .inspect_organization_cleanup(None, &mut |_| true)
            .unwrap();
        assert_eq!(inspection.duplicate_choices, 0);
        let retained_table = if unavailable {
            "organization_recovery_duplicate_decisions"
        } else {
            "exact_duplicate_decisions"
        };
        assert_eq!(count(&target.connection, retained_table), 1);
    }
}
