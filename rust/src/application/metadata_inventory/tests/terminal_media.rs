use super::*;
use crate::adapters::{LocalMediaInspector, revalidate_file_state};
use crate::domain::{ExpectedFileState, TerminalMediaEvidence};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants};
use crate::ports::MediaInspector;
use std::fs::FileTimes;

#[test]
fn unchanged_terminal_media_evidence_is_reused_after_catalog_reopen() {
    let mut fixture = InventoryFixture::new(&[]);
    fs::write(fixture.source.path().join("broken.jpg"), b"not a jpeg")
        .expect("write malformed media fixture");

    let first = fixture.run_inventory_with_id("inventory-terminal-1", 1, 4);
    assert!(first.is_complete);
    assert_eq!(first.candidate_count, 1);
    let processed = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        4_000,
        queue_policy(),
    )
    .expect("process terminal media candidate");
    assert_eq!(processed.completed_count, 1);
    assert_eq!(processed.retried_count, 0);
    assert_eq!(processed.applied_mutation_count, 1);
    let failed = fixture
        .location("broken.jpg")
        .expect("terminal media placeholder");
    assert!(matches!(failed.preview_status, PreviewStatus::Failed));
    assert!(failed.source_revision.is_some());
    assert!(failed.source_generation > 0);
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen catalog");

    let second = fixture.run_inventory_with_id("inventory-terminal-2", 2, 4);

    assert!(second.is_complete);
    assert_eq!(second.candidate_count, 0);
    assert!(second.unchanged_count >= 1);
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            3_000,
            queue_policy(),
        )
        .expect("queue metrics after evidence reuse");
    assert_eq!(
        metrics.pending_count + metrics.leased_count + metrics.retry_wait_count,
        0
    );
}

#[test]
fn mixed_non_media_and_malformed_batch_converges_without_retry_growth() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..96 {
        fs::write(
            fixture.source.path().join(format!("clip-{index:03}.mp4")),
            b"\0\0\0\x18ftypmp42\0\0\0\0",
        )
        .expect("write video fixture");
    }
    for index in 0..32 {
        fs::write(
            fixture.source.path().join(format!("broken-{index:03}.jpg")),
            b"not a jpeg",
        )
        .expect("write malformed image fixture");
    }

    let first = fixture.run_inventory_with_id("inventory-mixed-terminal-1", 1, 64);
    assert!(first.is_complete);
    assert_eq!(first.candidate_count, 128);
    let processed = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        4_000,
        queue_policy(),
    )
    .expect("process mixed terminal batch");

    assert_eq!(processed.leased_count, 128);
    assert_eq!(processed.completed_count, 128);
    assert_eq!(processed.retried_count, 0);
    assert_eq!(processed.applied_mutation_count, 32);
    let paths = (0..96)
        .map(|index| format!("clip-{index:03}.mp4"))
        .chain((0..32).map(|index| format!("broken-{index:03}.jpg")))
        .collect::<Vec<_>>();
    let locations = fixture
        .catalog
        .load_incremental_locations_by_relative_paths(&fixture.root_id, &paths)
        .expect("actual gallery locations");
    assert_eq!(
        locations.len(),
        32,
        "ordinary videos must not become gallery cards"
    );
    assert!(
        locations
            .iter()
            .all(|location| location.relative_path.starts_with("broken-")
                && matches!(location.preview_status, PreviewStatus::Failed))
    );
    let evidence = fixture
        .catalog
        .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
        .expect("independent negative observations");
    assert_eq!(evidence.len(), 128);
    for observation in &evidence {
        assert_current_negative(&fixture, observation);
    }
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            4_000,
            queue_policy(),
        )
        .expect("mixed terminal queue metrics");
    assert_eq!(metrics.retry_wait_count, 0);
    assert_eq!(metrics.exhausted_retry_count, 0);

    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("full schema reopen");
    assert_eq!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
            .expect("negative observations survive reopen"),
        evidence
    );
    let second = fixture.run_inventory_with_id("inventory-mixed-terminal-2", 2, 64);
    assert!(second.is_complete);
    assert_eq!(second.candidate_count, 0);
    assert_eq!(second.unchanged_count, 128);
    assert_eq!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
            .expect("unchanged observations are reused"),
        evidence
    );
    for name in &paths {
        let expected: &[u8] = if name.ends_with(".mp4") {
            b"\0\0\0\x18ftypmp42\0\0\0\0"
        } else {
            b"not a jpeg"
        };
        assert_eq!(
            fs::read(fixture.source.path().join(name)).expect("original bytes"),
            expected
        );
    }
    assert_eq!(
        fs::read_dir(fixture.source.path())
            .expect("source entries")
            .count(),
        128
    );
}

#[test]
fn non_media_evidence_revalidates_same_identity_size_and_restored_mtime_content_change() {
    let mut fixture = InventoryFixture::new(&[]);
    let name = "document.data";
    let path = fixture.source.path().join(name);
    let png = encode_rgb_quadrants(MediaFixtureFormat::Png, 24, 16).expect("real PNG pixels");
    let ordinary = vec![0_u8; png.len()];
    fs::write(&path, &ordinary).expect("ordinary non-image source");
    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let first = fixture.run_inventory_with_id("negative-revision-1", 1, 4);
    assert!(first.is_complete);
    assert_eq!(first.candidate_count, 1);
    let processed = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        4_000,
        queue_policy(),
    )
    .expect("negative publication");
    assert_eq!(processed.completed_count, 1);
    assert_eq!(processed.applied_mutation_count, 0);
    assert!(fixture.location(name).is_none());
    let paths = [name.to_owned()];
    let evidence = fixture
        .catalog
        .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
        .expect("negative evidence");
    assert_eq!(evidence.len(), 1);
    let previous = &evidence[0];
    assert_current_negative(&fixture, previous);
    assert_eq!(fs::read(&path).expect("ordinary bytes"), ordinary);
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone()).expect("negative reopen");
    let unchanged = fixture.run_inventory_with_id("negative-revision-2", 2, 4);
    assert!(unchanged.is_complete);
    assert_eq!(
        (unchanged.candidate_count, unchanged.unchanged_count),
        (0, 1)
    );

    fs::write(&path, &png).expect("same-file content replacement");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("owned attributes")
        .set_times(FileTimes::new().set_modified(modified))
        .expect("restore mtime");
    assert_eq!(
        fs::metadata(&path).expect("changed metadata").len(),
        previous.file_size
    );
    assert_eq!(
        fs::metadata(&path)
            .expect("changed metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    assert_eq!(
        crate::adapters::file_identity_evidence(&path).expect("same FileID"),
        previous.file_identity
    );
    assert!(
        revalidate_file_state(&negative_expected(&fixture, previous)).is_err(),
        "old source revision cannot certify changed bytes"
    );
    let changed = fixture.run_inventory_with_id("negative-revision-3", 3, 4);
    assert!(changed.is_complete);
    assert_eq!((changed.candidate_count, changed.unchanged_count), (1, 0));
    let processed = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        5_000,
        queue_policy(),
    )
    .expect("inspect changed image");
    assert_eq!(processed.completed_count, 1);
    assert_eq!(processed.retried_count, 0);
    assert_eq!(processed.applied_mutation_count, 1);
    let location = fixture.location(name).expect("actual image location");
    assert_eq!((location.width, location.height), (24, 16));
    assert_eq!(location.file_identity, previous.file_identity);
    assert_ne!(location.source_revision, previous.source_revision);
    assert!(location.source_generation > previous.source_generation);
    assert!(
        fixture
            .catalog
            .load_terminal_media_evidence_by_relative_paths(&fixture.root_id, &paths)
            .expect("successful image invalidates negative evidence")
            .is_empty()
    );
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("successful image reopen");
    let settled = fixture.run_inventory_with_id("negative-revision-4", 4, 4);
    assert!(settled.is_complete);
    assert_eq!((settled.candidate_count, settled.unchanged_count), (0, 1));
    assert_eq!(fs::read(&path).expect("new source bytes"), png);
    assert_eq!(
        fs::metadata(&path)
            .expect("final metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    assert_eq!(
        fs::read_dir(fixture.source.path())
            .expect("source entries")
            .count(),
        1
    );
}

fn assert_current_negative(fixture: &InventoryFixture, evidence: &TerminalMediaEvidence) {
    let inspector = LocalMediaInspector::new();
    assert!(evidence.file_identity.is_some());
    assert!(evidence.source_revision.is_some());
    assert!(evidence.source_generation > 0);
    assert_eq!(
        evidence.inspection_engine_id,
        inspector.inspection_engine_id()
    );
    assert_eq!(
        evidence.inspection_engine_version,
        inspector.inspection_engine_version()
    );
    revalidate_file_state(&negative_expected(fixture, evidence)).expect("current source version");
}

fn negative_expected(
    fixture: &InventoryFixture,
    evidence: &TerminalMediaEvidence,
) -> ExpectedFileState {
    ExpectedFileState {
        absolute_path: fixture
            .source
            .path()
            .join(&evidence.relative_path)
            .to_string_lossy()
            .into_owned(),
        file_size: evidence.file_size,
        modified_unix_ms: evidence.modified_unix_ms,
        file_identity: evidence.file_identity.clone(),
        source_revision: evidence.source_revision.clone(),
    }
}
