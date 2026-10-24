use std::fs;

use rusqlite::params;

use super::super::tests::{assert_no_scratch, fixture};
use super::super::{OrganizationArchiveRepository, SqliteCatalog};
use super::*;

mod cost;

const IDENTITY: &str = "0000000000000001:00000000000000000000000000000001";
const REVISION: &str = "windows-file-change-time-100ns-v1:0000000000000001";

fn native(catalog: &SqliteCatalog, identity: &str, revision: &str) {
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET file_identity_scheme = 'windows-file-id-128-v1',
         file_identity_value = ?1, source_revision_token = ?2",
            params![identity, revision],
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "UPDATE organization_bindings SET file_identity_scheme = 'windows-file-id-128-v1',
         file_identity_value = ?1, source_revision_token = ?2",
            params![identity, revision],
        )
        .unwrap();
}

fn content(catalog: &SqliteCatalog, digit: char, published: bool) {
    catalog
        .connection
        .execute_batch(
            "INSERT OR IGNORE INTO exact_analysis_runs
           VALUES ('archive-association', 'blake3-256-full-v1', 'fixture', '1', '{}', 1);
         DELETE FROM gallery_content_fingerprints;
         DELETE FROM content_fingerprints;",
        )
        .unwrap();
    catalog
        .connection
        .execute(
            "INSERT INTO content_fingerprints
         SELECT DISTINCT source_generation, 'blake3-256-full-v1', ?1, file_size,
           file_identity_scheme, file_identity_value, source_revision_token, 'archive-association'
         FROM asset_locations",
            [digit.to_string().repeat(64)],
        )
        .unwrap();
    catalog.connection.execute(
        "UPDATE organization_bindings SET fingerprint_scheme = 'blake3-256-full-v1', digest = ?1",
        [digit.to_string().repeat(64)],
    ).unwrap();
    if published {
        catalog.connection.execute_batch(
            "INSERT INTO gallery_content_fingerprints SELECT source_generation, scheme FROM content_fingerprints",
        ).unwrap();
    }
}

fn target(directory: &Path, locations: &[(&str, Option<&str>, i64)]) -> SqliteCatalog {
    let mut catalog = SqliteCatalog::open(directory.join("rebuilt.sqlite3")).unwrap();
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "rebuilt-scan",
        "rebuilt-root",
        "D:\\UnavailableRebuiltFixture",
        locations,
    );
    catalog
}

fn backup(directory: &Path, catalog: &SqliteCatalog) -> std::path::PathBuf {
    let archive = directory.join("records.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    archive
}

fn matched(catalog: &SqliteCatalog, archive: &Path) -> OrganizationArchiveAssociations {
    catalog
        .inspect_organization_archive(archive, &mut |_| true)
        .unwrap()
        .associations
}

#[test]
fn organization_archive_matching_never_uses_catalog_identifiers_or_path_as_proof() {
    let (directory, catalog) = fixture();
    let archive = backup(directory.path(), &catalog);
    let before = fs::read(&archive).unwrap();
    let changes = catalog.connection.total_changes();
    let result = matched(&catalog, &archive);
    assert_eq!((result.matched_groups, result.unmatched_groups), (0, 1));
    assert_eq!(result.unresolved_preferred_copies, 1);
    assert_eq!(catalog.connection.total_changes(), changes);
    assert_eq!(fs::read(archive).unwrap(), before);
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_matching_follows_native_versions_across_rebuild_and_rename() {
    let (directory, catalog) = fixture();
    native(&catalog, IDENTITY, REVISION);
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(directory.path(), &[("renamed", None, 5)]);
    native(&rebuilt, IDENTITY, REVISION);
    let result = matched(&rebuilt, &archive);
    assert_eq!((result.matched_groups, result.unmatched_groups), (1, 0));
    for change in [
        "UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'",
        "UPDATE asset_locations SET file_identity_value = '0000000000000002:00000000000000000000000000000001'",
        "UPDATE asset_locations SET file_identity_scheme = 'unrecognized-native-v1'",
        "UPDATE asset_locations SET source_revision_token = NULL",
        "UPDATE asset_locations SET file_size = 21",
    ] {
        native(&rebuilt, IDENTITY, REVISION);
        rebuilt.connection.execute_batch(change).unwrap();
        assert_eq!(matched(&rebuilt, &archive).unmatched_groups, 1, "{change}");
    }
}

#[test]
fn organization_archive_matching_uses_published_compatible_content_across_volumes() {
    let (directory, catalog) = fixture();
    content(&catalog, 'a', true);
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(
        directory.path(),
        &[("copied", None, 5), ("second", None, 6)],
    );
    native(&rebuilt, IDENTITY, REVISION);
    content(&rebuilt, 'a', true);
    assert_eq!(matched(&rebuilt, &archive).matched_groups, 1);
    for change in [
        "DELETE FROM gallery_content_fingerprints",
        "UPDATE content_fingerprints SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002'",
        "UPDATE content_fingerprints SET byte_length = 21",
        "DELETE FROM gallery_content_fingerprints;
         UPDATE content_fingerprints SET scheme = 'unrecognized-hash-v1';
         INSERT INTO gallery_content_fingerprints SELECT source_generation, scheme FROM content_fingerprints",
        "UPDATE library_change_root_state SET is_active = 0",
    ] {
        content(&rebuilt, 'a', true);
        rebuilt.connection.execute_batch(change).unwrap();
        assert_eq!(matched(&rebuilt, &archive).unmatched_groups, 1, "{change}");
    }
}

#[test]
fn organization_archive_matching_rejects_native_matches_with_contrary_content() {
    let (directory, catalog) = fixture();
    native(&catalog, IDENTITY, REVISION);
    content(&catalog, 'a', true);
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(directory.path(), &[("changed", None, 5)]);
    native(&rebuilt, IDENTITY, REVISION);
    content(&rebuilt, 'b', false);
    assert_eq!(matched(&rebuilt, &archive).unmatched_groups, 1);
    content(&rebuilt, 'a', false);
    assert_eq!(matched(&rebuilt, &archive).matched_groups, 1);
}

#[test]
fn organization_archive_matching_counts_a_group_once_when_aliases_and_copies_match() {
    let (directory, catalog) = fixture();
    content(&catalog, 'a', true);
    catalog.connection.execute_batch(
        "INSERT INTO organization_records VALUES ('alias', NULL, 'pending', 0, 0, 0, 1, 1, NULL);
         INSERT INTO organization_record_groups VALUES ('alias', 'record-a');
         INSERT INTO organization_bindings
           SELECT 'alias', 'old-copy', source_generation + 100, byte_length,
             NULL, NULL, NULL, fingerprint_scheme, digest FROM organization_bindings;",
    ).unwrap();
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(directory.path(), &[("one", None, 5), ("two", None, 6)]);
    content(&rebuilt, 'a', true);
    let result = matched(&rebuilt, &archive);
    assert_eq!((result.matched_groups, result.unmatched_groups), (1, 0));
}

#[test]
fn organization_archive_matching_distinguishes_preferred_copy_from_equal_bytes_and_hardlinks() {
    let (directory, catalog) = fixture();
    native(&catalog, IDENTITY, REVISION);
    content(&catalog, '0', true);
    catalog.connection.execute_batch(
        "UPDATE exact_duplicate_decisions SET preferred_asset_id = (SELECT asset_id FROM asset_locations LIMIT 1)",
    ).unwrap();
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(directory.path(), &[("one", None, 5), ("two", None, 6)]);
    content(&rebuilt, '0', true);
    assert_eq!(matched(&rebuilt, &archive).resolved_preferred_copies, 0);
    native(&rebuilt, IDENTITY, REVISION);
    content(&rebuilt, '0', true);
    let ambiguous = matched(&rebuilt, &archive);
    assert_eq!(ambiguous.matched_groups, 1);
    assert_eq!(ambiguous.unresolved_preferred_copies, 1);
    rebuilt.connection.execute(
        "UPDATE asset_locations SET file_identity_value = '0000000000000001:00000000000000000000000000000002'
         WHERE location_id = 'two'", [],
    ).unwrap();
    content(&rebuilt, '0', true);
    let unique = matched(&rebuilt, &archive);
    assert_eq!(
        (
            unique.resolved_preferred_copies,
            unique.unresolved_preferred_copies
        ),
        (1, 0)
    );
}

#[test]
fn organization_archive_matching_retains_one_catalog_snapshot_during_concurrent_publication() {
    let (directory, catalog) = fixture();
    native(&catalog, IDENTITY, REVISION);
    let archive = backup(directory.path(), &catalog);
    let live = Connection::open(&catalog.path).unwrap();
    let revision: u64 = live
        .query_row("SELECT revision FROM catalog_state", [], |row| {
            read_unsigned(row, 0)
        })
        .unwrap();
    let mut changed = false;
    let inspected = catalog.inspect_organization_archive(&archive, &mut |event| {
        if event.phase == OrganizationArchivePhase::Matching && !changed {
            live.execute_batch(
                "BEGIN IMMEDIATE;
                 UPDATE asset_locations SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000002';
                 UPDATE catalog_state SET revision = revision + 1;
                 UPDATE organization_state SET revision = revision + 1;
                 COMMIT;",
            ).unwrap();
            changed = true;
        }
        true
    }).unwrap();
    assert!(changed);
    assert_eq!(inspected.associations.catalog_revision, revision);
    assert_eq!(inspected.associations.organization_revision, 3);
    assert_eq!(inspected.associations.matched_groups, 1);
    let current = matched(&catalog, &archive);
    assert_eq!(current.catalog_revision, revision + 1);
    assert_eq!(current.organization_revision, 4);
    assert_eq!(current.unmatched_groups, 1);
}

#[test]
fn organization_archive_matching_preserves_preference_ambiguity_when_an_old_hardlink_disappears() {
    let (directory, mut catalog) = fixture();
    super::super::super::tests::publish_gallery_fixture(
        &mut catalog,
        "hardlink-scan",
        "hardlink-root",
        "D:\\UnavailableHardlinkFixture",
        &[("old-hardlink", None, 2)],
    );
    native(&catalog, IDENTITY, REVISION);
    content(&catalog, '0', true);
    catalog
        .connection
        .execute_batch(
            "UPDATE exact_duplicate_decisions SET preferred_asset_id = (
           SELECT asset_id FROM asset_locations WHERE location_id = 'a')",
        )
        .unwrap();
    let archive = backup(directory.path(), &catalog);
    let rebuilt = target(directory.path(), &[("surviving-hardlink", None, 5)]);
    native(&rebuilt, IDENTITY, REVISION);
    content(&rebuilt, '0', true);
    let result = matched(&rebuilt, &archive);
    assert_eq!(result.matched_groups, 1);
    assert_eq!(
        (
            result.resolved_preferred_copies,
            result.unresolved_preferred_copies
        ),
        (0, 1)
    );
}

#[test]
fn organization_archive_matching_cancels_between_bounded_windows_without_retained_state() {
    let (directory, catalog) = fixture();
    content(&catalog, 'a', true);
    let archive = backup(directory.path(), &catalog);
    let names = (0..600)
        .map(|value| format!("copy-{value:04}"))
        .collect::<Vec<_>>();
    let locations = names
        .iter()
        .map(|name| (name.as_str(), None, 1))
        .collect::<Vec<_>>();
    let rebuilt = target(directory.path(), &locations);
    content(&rebuilt, 'a', true);
    let changes = rebuilt.connection.total_changes();
    let failed = rebuilt
        .inspect_organization_archive(&archive, &mut |event| {
            event.phase != OrganizationArchivePhase::Matching || event.completed_units < 256
        })
        .unwrap_err();
    assert_eq!(failed.code, "organization_archive_cancelled");
    assert_eq!(rebuilt.connection.total_changes(), changes);
    assert_no_scratch(directory.path());
    assert_eq!(matched(&rebuilt, &archive).matched_groups, 1);
    assert_no_scratch(directory.path());
}
