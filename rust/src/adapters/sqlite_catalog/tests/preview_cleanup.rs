use crate::adapters::sqlite_catalog::tests::support::{publish_fixture, publish_preview_artifact};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn preview_cleanup_flushes_staging_and_resets_every_owner_without_changing_source_state() {
    for active_root in [None, Some("D:\\ActiveCache\\")] {
        let directory = tempdir().expect("temporary catalog");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        publish_cleanup_fixture(&mut catalog);
        let mut expected = stage_failed_preview(&mut catalog, "C:\\CleanupCache\\failed.jpg");
        assert_eq!(catalog.pending_locations.len(), 1);

        let reset = match active_root {
            Some(prefix) => catalog.reset_previews_outside_root(prefix),
            None => catalog.reset_all_previews_for_cleanup(),
        }
        .expect("reset previews");

        assert_eq!(reset, 1, "count only location records, not handoff owners");
        assert!(catalog.pending_locations.is_empty());
        expected.preview_path.clear();
        expected.preview_status = PreviewStatus::Pending;
        expected.preview_issue_code = None;
        expected.preview_issue_message = None;
        assert_eq!(
            catalog
                .load_active_location(&expected.location_id)
                .expect("location query"),
            Some(expected)
        );
        let owners = preview_owners(&catalog);
        assert_eq!(owners.len(), 3);
        for owner in owners {
            assert!(owner.path.is_empty());
            assert_eq!(owner.status, "pending");
            assert_eq!(owner.issue_code, None);
            assert_eq!(owner.issue_message, None);
        }
        assert!(
            !catalog
                .is_preview_artifact_path_indexed("C:\\CleanupCache\\thumb.jpg", None)
                .expect("artifact lookup")
        );
        let repeated = match active_root {
            Some(prefix) => catalog.reset_previews_outside_root(prefix),
            None => catalog.reset_all_previews_for_cleanup(),
        }
        .expect("repeat reset");
        assert_eq!(repeated, 0);
    }
}

#[test]
fn empty_preview_root_is_rejected_before_pending_locations_are_flushed() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_cleanup_fixture(&mut catalog);
    let before = preview_owners(&catalog);
    stage_failed_preview(&mut catalog, "C:\\CleanupCache\\failed.jpg");

    let error = catalog
        .reset_previews_outside_root("")
        .expect_err("empty root is invalid");

    assert_eq!(error.code, "preview_root_prefix_empty");
    assert_eq!(catalog.pending_locations.len(), 1);
    assert_eq!(preview_owners(&catalog), before);
    assert!(
        catalog
            .is_preview_artifact_path_indexed("C:\\CleanupCache\\thumb.jpg", None)
            .expect("artifact remains indexed")
    );
}

#[test]
fn preview_cleanup_rolls_back_all_owners_if_artifact_deletion_fails() {
    for active_root in [None, Some("D:\\ActiveCache\\")] {
        let directory = tempdir().expect("temporary catalog");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        publish_cleanup_fixture(&mut catalog);
        let before = preview_owners(&catalog);
        catalog
            .connection
            .execute_batch(
                "CREATE TEMP TRIGGER reject_preview_artifact_cleanup
                 BEFORE DELETE ON preview_artifacts
                 BEGIN SELECT RAISE(ABORT, 'fixture cleanup failure'); END;",
            )
            .expect("inject final-statement failure");

        let result = match active_root {
            Some(prefix) => catalog.reset_previews_outside_root(prefix),
            None => catalog.reset_all_previews_for_cleanup(),
        };

        let error = result.expect_err("artifact deletion fails");
        assert!(error.message.contains("fixture cleanup failure"));
        assert_eq!(preview_owners(&catalog), before);
        assert!(
            catalog
                .is_preview_artifact_path_indexed("C:\\CleanupCache\\thumb.jpg", None)
                .expect("artifact remains indexed")
        );
    }
}

#[test]
fn selective_cleanup_preserves_case_insensitive_root_and_pathless_failures() {
    let directory = tempdir().expect("temporary catalog");
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    publish_cleanup_fixture(&mut catalog);
    let before = preview_owners(&catalog);

    assert_eq!(
        catalog
            .reset_previews_outside_root("c:\\CLEANUPcache\\")
            .expect("case-insensitive active root"),
        0
    );
    assert_eq!(preview_owners(&catalog), before);
    let failed = stage_failed_preview(&mut catalog, "");

    assert_eq!(
        catalog
            .reset_previews_outside_root("D:\\ActiveCache\\")
            .expect("reset only paths outside the active root"),
        0
    );
    assert_eq!(
        catalog
            .load_active_location(&failed.location_id)
            .expect("failed location query"),
        Some(failed)
    );
    for owner in preview_owners(&catalog)
        .into_iter()
        .filter(|owner| owner.kind != "location")
    {
        assert_eq!(owner.status, "pending");
        assert!(owner.path.is_empty());
    }
    assert!(
        !catalog
            .is_preview_artifact_path_indexed("C:\\CleanupCache\\thumb.jpg", None)
            .expect("old artifact removed")
    );
}

#[test]
fn preview_root_activation_resets_only_artifacts_outside_the_new_root() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, preview_root) in [
        ("old", "C:\\OldPreviewRoot"),
        ("current", "D:\\CurrentPreviewRoot"),
    ] {
        let scan_id = format!("root-switch-{suffix}-scan");
        let root_id = format!("root-switch-{suffix}-root");
        let location_id = format!("root-switch-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\RootSwitchSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("{preview_root}\\{suffix}.jpg");
        location.preview_status = PreviewStatus::Ready;
        let artifact = PreviewArtifact {
            artifact_key: format!("root-switch-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 256,
            encoded_height: 192,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("publish preview artifact");
    }

    let reset = catalog
        .reset_previews_outside_root("D:\\CurrentPreviewRoot\\")
        .expect("reset old preview root");

    assert_eq!(reset, 1);
    let old = catalog
        .load_active_location("root-switch-old-location")
        .expect("old location query")
        .expect("old location");
    let current = catalog
        .load_active_location("root-switch-current-location")
        .expect("current location query")
        .expect("current location");
    assert!(matches!(old.preview_status, PreviewStatus::Pending));
    assert!(old.preview_path.is_empty());
    assert_eq!((old.width, old.height), (40, 50));
    assert!(matches!(current.preview_status, PreviewStatus::Ready));
    assert_eq!(current.preview_path, "D:\\CurrentPreviewRoot\\current.jpg");
    let artifact_count: i64 = catalog
        .connection
        .query_row("SELECT COUNT(*) FROM preview_artifacts", [], |row| {
            row.get(0)
        })
        .expect("artifact count");
    assert_eq!(artifact_count, 1);
    assert!(
        catalog
            .is_preview_artifact_path_indexed(
                "C:\\EquivalentPreviewAlias\\current.jpg",
                Some("root-switch-current"),
            )
            .expect("artifact-key ownership lookup")
    );
}

fn publish_cleanup_fixture(catalog: &mut SqliteCatalog) {
    publish_fixture(
        catalog,
        "cleanup-scan",
        "cleanup-root",
        "C:\\CleanupSource",
        "cleanup-location",
    );
    publish_preview_artifact(
        catalog,
        "cleanup-location",
        "cleanup-artifact",
        "C:\\CleanupCache\\thumb.jpg",
    );
    catalog
        .connection
        .execute_batch(
            "INSERT INTO library_change_catch_up_handoffs(
               catch_up_source, catch_up_watermark,
               file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               updated_unix_ms
             )
             SELECT 'windows_usn_v1', 'cleanup-watermark',
                    'windows-file-id-128-v1', 'cleanup-identity',
                    asset_id, location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, 'failed', 'decode_failed', 'Fixture decode failed',
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value, 1
             FROM asset_locations WHERE location_id = 'cleanup-location';
             INSERT INTO library_change_scan_handoff_batches(id, source_root_id, updated_unix_ms)
             VALUES ('cleanup-batch', 'cleanup-root', 1);
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value
             )
             SELECT 'cleanup-batch', file_identity_scheme, file_identity_value,
                    asset_id, source_location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code, preview_issue_message,
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value
             FROM library_change_catch_up_handoffs;",
        )
        .expect("publish both handoff preview owners");
}

fn stage_failed_preview(catalog: &mut SqliteCatalog, path: &str) -> AssetLocationView {
    let mut location = catalog
        .load_active_location("cleanup-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = path.to_owned();
    location.preview_status = PreviewStatus::Failed;
    location.preview_issue_code = Some("decode_failed".to_owned());
    location.preview_issue_message = Some("Fixture decode failed".to_owned());
    catalog
        .stage_location(&location.scan_id, &location.root_id, &location)
        .expect("stage pending location");
    location
}

#[derive(Debug, PartialEq, Eq)]
struct PreviewOwnerState {
    kind: String,
    path: String,
    status: String,
    issue_code: Option<String>,
    issue_message: Option<String>,
}

fn preview_owners(catalog: &SqliteCatalog) -> Vec<PreviewOwnerState> {
    catalog
        .connection
        .prepare(
            "SELECT 'location' AS kind, preview_path, preview_status,
                    preview_issue_code, preview_issue_message FROM asset_locations
             UNION ALL
             SELECT 'legacy_handoff', preview_path, preview_status,
                    preview_issue_code, preview_issue_message FROM library_change_catch_up_handoffs
             UNION ALL
             SELECT 'scan_handoff', preview_path, preview_status,
                    preview_issue_code, preview_issue_message FROM library_change_scan_handoff_items
             ORDER BY kind",
        )
        .expect("preview owner query")
        .query_map([], |row| {
            Ok(PreviewOwnerState {
                kind: row.get(0)?,
                path: row.get(1)?,
                status: row.get(2)?,
                issue_code: row.get(3)?,
                issue_message: row.get(4)?,
            })
        })
        .expect("preview owners")
        .collect::<Result<_, _>>()
        .expect("preview owner rows")
}
