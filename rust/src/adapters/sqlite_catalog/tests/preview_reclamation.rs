use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, preview_lifecycle_state, preview_reference_count, publish_fixture,
    publish_gallery_fixture, publish_preview_artifact,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn preview_usage_touches_are_coarsened_to_page_publication_intervals() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "preview-touch-scan",
        "preview-touch-root",
        "C:\\PreviewTouchSource",
        "preview-touch-location",
    );
    let mut location = catalog
        .load_active_location("preview-touch-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = "C:\\AmeCache\\preview-touch.jpg".to_owned();
    let artifact = PreviewArtifact {
        artifact_key: "preview-touch-artifact".to_owned(),
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
    catalog
        .connection
        .execute(
            "UPDATE preview_artifacts SET last_used_unix_ms = 0 WHERE artifact_key = ?1",
            [&artifact.artifact_key],
        )
        .expect("age preview artifact");
    let visible = vec![(location.location_id, location.preview_path)];

    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("first usage touch"),
        1
    );
    assert_eq!(
        catalog
            .touch_preview_artifacts(&visible)
            .expect("coarsened usage touch"),
        0
    );
}

#[test]
fn preview_reclamation_orders_stale_before_lru_and_preserves_protected_locations() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    for (suffix, last_used) in [("stale", 30_i64), ("lru", 10_i64), ("protected", 1_i64)] {
        let scan_id = format!("reclaim-{suffix}-scan");
        let root_id = format!("reclaim-{suffix}-root");
        let location_id = format!("reclaim-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\ReclaimSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = format!("C:\\AmeCache\\{suffix}.jpg");
        let artifact = PreviewArtifact {
            artifact_key: format!("artifact-{suffix}"),
            algorithm_id: "ame-jpeg-thumbnail".to_owned(),
            algorithm_version: 2,
            orientation_contract: "exif-display-v1".to_owned(),
            size_bucket: 256,
            path: location.preview_path.clone(),
            byte_size: 1_024,
            encoded_width: 40,
            encoded_height: 50,
            width: location.width,
            height: location.height,
        };
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("publish preview artifact");
        catalog
            .connection
            .execute(
                "UPDATE preview_artifacts
                 SET lifecycle_state = ?2, last_used_unix_ms = ?3
                 WHERE artifact_key = ?1",
                params![
                    artifact.artifact_key,
                    if suffix == "stale" { "stale" } else { "ready" },
                    last_used,
                ],
            )
            .expect("set reclamation evidence");
    }

    let protected = vec!["reclaim-protected-location".to_owned()];
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &protected,
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("reclamation candidates");

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.artifact_key.as_str())
            .collect::<Vec<_>>(),
        ["artifact-stale", "artifact-lru"],
    );
    assert!(
        catalog
            .remove_reclaimed_preview(&candidates[0])
            .expect("remove reclaimed preview")
    );
    let reclaimed = catalog
        .load_active_location("reclaim-stale-location")
        .expect("reclaimed location query")
        .expect("reclaimed location");
    assert!(matches!(reclaimed.preview_status, PreviewStatus::Pending));
    assert!(reclaimed.preview_path.is_empty());
    assert_eq!((reclaimed.width, reclaimed.height), (40, 50));
}

#[test]
fn handoff_preview_owners_survive_staling_and_reclamation_until_explicit_cleanup() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    publish_fixture(
        &mut catalog,
        "handoff-owner-scan",
        "handoff-owner-root",
        "C:\\HandoffOwnerSource",
        "handoff-owner-location",
    );
    let mut location = catalog
        .load_active_location("handoff-owner-location")
        .expect("active location query")
        .expect("active location");
    location.preview_path = "C:\\AmeCache\\handoff-owner.jpg".to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: "handoff-owner-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), None)
        .expect("publish handoff-owned preview");
    let identity = FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "handoff-owner-volume:file".to_owned(),
    };
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET file_identity_scheme = ?2, file_identity_value = ?3
             WHERE location_id = ?1",
            params![location.location_id, identity.scheme, identity.value],
        )
        .expect("assign handoff identity");
    location.file_identity = Some(identity);
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
             SELECT 'windows_usn_v1', 'legacy-owner',
                    file_identity_scheme, file_identity_value,
                    asset_id, location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code, preview_issue_message,
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value, 1
             FROM asset_locations
             WHERE scan_id = 'handoff-owner-scan' AND location_id = 'handoff-owner-location';
             INSERT INTO library_change_scan_handoff_batches(id, source_root_id, updated_unix_ms)
             VALUES ('handoff-owner-batch', 'handoff-owner-root', 1);
             INSERT INTO library_change_scan_handoff_items(
               batch_id, file_identity_scheme, file_identity_value,
               asset_id, source_location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               width, height, preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value
             )
             SELECT 'handoff-owner-batch', file_identity_scheme, file_identity_value,
                    asset_id, location_id, root_id, absolute_path, relative_path,
                    preview_path, file_size, created_unix_ms, modified_unix_ms,
                    width, height, preview_status, preview_issue_code, preview_issue_message,
                    metadata_engine_id, metadata_engine_version, capture_local_time,
                    capture_offset_minutes, capture_time_source, capture_raw_value
             FROM asset_locations
             WHERE scan_id = 'handoff-owner-scan' AND location_id = 'handoff-owner-location';",
        )
        .expect("handoff preview owners");

    location.preview_path.clear();
    location.preview_status = PreviewStatus::Pending;
    catalog
        .update_active_preview(&location, None, None)
        .expect("detach active preview owner");
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("handoff-protected reclamation candidates");
    assert!(candidates.is_empty());
    assert!(
        !catalog
            .remove_reclaimed_preview(&PreviewReclamationCandidate {
                artifact_key: artifact.artifact_key.clone(),
                path: artifact.path.clone(),
            })
            .expect("defensive handoff reclamation guard")
    );

    catalog
        .reset_all_previews_for_cleanup()
        .expect("explicit handoff preview cleanup");
    let state: (i64, String, String, String, String) = catalog
        .connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM preview_artifacts WHERE artifact_key = ?1),
               (SELECT preview_status FROM library_change_catch_up_handoffs
                WHERE catch_up_watermark = 'legacy-owner'),
               (SELECT preview_path FROM library_change_catch_up_handoffs
                WHERE catch_up_watermark = 'legacy-owner'),
               (SELECT preview_status FROM library_change_scan_handoff_items
                WHERE batch_id = 'handoff-owner-batch'),
               (SELECT preview_path FROM library_change_scan_handoff_items
                WHERE batch_id = 'handoff-owner-batch')",
            [&artifact.artifact_key],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("cleaned handoff preview state");
    assert_eq!(
        state,
        (
            0,
            "pending".to_owned(),
            String::new(),
            "pending".to_owned(),
            String::new()
        )
    );
}

#[test]
fn shared_preview_is_protected_and_reset_through_every_active_location() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let artifact = PreviewArtifact {
        artifact_key: "shared-artifact".to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: "C:\\AmeCache\\shared.jpg".to_owned(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: 40,
        height: 50,
    };
    for suffix in ["first", "second"] {
        let scan_id = format!("shared-{suffix}-scan");
        let root_id = format!("shared-{suffix}-root");
        let location_id = format!("shared-{suffix}-location");
        publish_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\SharedSource\\{suffix}"),
            &location_id,
        );
        let mut location = catalog
            .load_active_location(&location_id)
            .expect("active location query")
            .expect("active location");
        location.preview_path = artifact.path.clone();
        location.preview_status = PreviewStatus::Ready;
        catalog
            .update_active_preview(&location, Some(&artifact), None)
            .expect("share preview artifact");
    }
    let reference_count: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations
             WHERE artifact_key = ?1",
            [&artifact.artifact_key],
            |row| row.get(0),
        )
        .expect("shared references");
    assert_eq!(reference_count, 2);

    let protected = vec!["shared-first-location".to_owned()];
    let protected_candidates = catalog
        .load_preview_reclamation_candidates(
            &protected,
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("protected candidates");
    assert!(protected_candidates.is_empty());

    let candidates = catalog
        .load_preview_reclamation_candidates(
            &[],
            "ame-jpeg-thumbnail",
            2,
            "exif-display-v1",
            "c:\\amecache\\",
            8,
        )
        .expect("unprotected candidates");
    assert_eq!(candidates.len(), 1);
    assert!(
        catalog
            .remove_reclaimed_preview(&candidates[0])
            .expect("remove shared preview")
    );
    for location_id in ["shared-first-location", "shared-second-location"] {
        let location = catalog
            .load_active_location(location_id)
            .expect("reclaimed location query")
            .expect("reclaimed location");
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        assert!(location.preview_path.is_empty());
    }
}

#[test]
fn publishing_replacement_scans_stales_artifact_only_after_its_last_location_leaves() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let artifact_key = "removed-shared-artifact";
    let artifact_path = "C:\\AmeCache\\removed-shared.jpg";
    for suffix in ["first", "second"] {
        let scan_id = format!("removed-shared-{suffix}-scan");
        let root_id = format!("removed-shared-{suffix}-root");
        let location_id = format!("removed-shared-{suffix}-location");
        publish_gallery_fixture(
            &mut catalog,
            &scan_id,
            &root_id,
            &format!("C:\\RemovedSharedSource\\{suffix}"),
            &[(location_id.as_str(), None, 30)],
        );
        publish_preview_artifact(&mut catalog, &location_id, artifact_key, artifact_path);
    }
    assert_eq!(preview_reference_count(&catalog, artifact_key), 2);

    publish_empty_replacement_scan(
        &mut catalog,
        "removed-shared-first-replacement",
        "removed-shared-first-root",
        "C:\\RemovedSharedSource\\first",
    );
    assert_eq!(preview_reference_count(&catalog, artifact_key), 1);
    assert_eq!(preview_lifecycle_state(&catalog, artifact_key), "ready");

    publish_empty_replacement_scan(
        &mut catalog,
        "removed-shared-second-replacement",
        "removed-shared-second-root",
        "C:\\RemovedSharedSource\\second",
    );
    assert_eq!(preview_reference_count(&catalog, artifact_key), 0);
    assert_eq!(preview_lifecycle_state(&catalog, artifact_key), "stale");
}

#[test]
fn abandoning_staged_scan_preserves_active_preview_reference() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let root_id = "abandoned-preview-root";
    let location_id = "abandoned-preview-location";
    publish_gallery_fixture(
        &mut catalog,
        "abandoned-preview-active-scan",
        root_id,
        "C:\\AbandonedPreviewSource",
        &[(location_id, None, 30)],
    );
    let artifact = publish_preview_artifact(
        &mut catalog,
        location_id,
        "abandoned-preview-artifact",
        "C:\\AmeCache\\abandoned-preview.jpg",
    );
    let staged_scan_id = "abandoned-preview-staged-scan";
    let request = fixture_request(staged_scan_id, "C:\\AbandonedPreviewSource");
    let active = catalog
        .load_active_location(location_id)
        .expect("active location query")
        .expect("active location");
    catalog
        .begin_scan(&request, root_id, &request.root_path)
        .expect("begin staged scan");
    catalog
        .stage_location(staged_scan_id, root_id, &active)
        .expect("stage matching location");
    assert_eq!(
        catalog
            .count_staged_file_states(staged_scan_id)
            .expect("flush staged location"),
        1
    );

    catalog
        .abandon_scan(staged_scan_id, "cancelled", 0)
        .expect("abandon staged scan");

    assert_eq!(preview_reference_count(&catalog, &artifact.artifact_key), 1);
    assert_eq!(
        preview_lifecycle_state(&catalog, &artifact.artifact_key),
        "ready"
    );
    let retained = catalog
        .load_active_location(location_id)
        .expect("retained active location query")
        .expect("retained active location");
    assert_eq!(retained.preview_path, artifact.path);
    assert!(matches!(retained.preview_status, PreviewStatus::Ready));
}

fn publish_empty_replacement_scan(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin empty replacement scan");
    catalog
        .publish_scan(scan_id, root_id, 0, 0)
        .expect("publish empty replacement scan");
}
