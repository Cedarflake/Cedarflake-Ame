use std::collections::BTreeMap;
use std::fs::{self, File, FileTimes};
use std::path::Path;
use std::time::SystemTime;

use rusqlite::{Connection, OpenFlags, params};
use tempfile::{TempDir, tempdir};

use crate::adapters::SqliteCatalog;
use crate::application::StoragePaths;
use crate::application::preview::materialize_preview_with_storage;
use crate::domain::{
    AssetLocationView, CatalogSnapshot, GalleryQuery, LibraryChangeLane, LibraryChangeQueuePolicy,
    PreviewRequest, PreviewStatus, ScanError, ScanEvent, ScanIssue, ScanRequest,
};
use crate::media_fixtures::{
    ALL_MEDIA_FORMATS, MediaFixtureFormat, QUADRANT_COLORS, encode_rgb_quadrants,
    encode_rgb_quadrants_with_colors, truncated_header, truncated_pixels,
};
use crate::ports::{CatalogRepository, IncrementalCatalogRepository};

use super::{run_scan_with_storage, stable_id};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 96;
const REPLACEMENT_COLORS: [[u8; 3]; 4] = [
    QUADRANT_COLORS[3],
    QUADRANT_COLORS[2],
    QUADRANT_COLORS[1],
    QUADRANT_COLORS[0],
];

#[test]
fn mixed_media_scan_and_lazy_previews_use_content_and_isolate_invalid_inputs() {
    let mut fixture = MediaInputCatalog::new();
    let mut valid_names = Vec::new();
    for format in ALL_MEDIA_FORMATS {
        let bytes = encode_rgb_quadrants(format, WIDTH, HEIGHT).expect("encoded format fixture");
        let extension = format.extension();
        let wrong_extension = if format == MediaFixtureFormat::Png {
            "jpg"
        } else {
            "png"
        };
        for name in [
            format!("正确-{extension}.{extension}"),
            format!("冲突-{extension}.{wrong_extension}"),
            format!("未知-{extension}.data"),
            format!("无后缀-{extension}"),
        ] {
            fixture.write(&name, bytes.clone());
            valid_names.push(name);
        }
    }
    fixture.write("伪装.jpg", b"ordinary non-image content".to_vec());
    fixture.write("空文件.png", Vec::new());
    fixture.write("坏头.bmp", truncated_header(MediaFixtureFormat::Bmp));
    let bmp = encode_rgb_quadrants(MediaFixtureFormat::Bmp, WIDTH, HEIGHT).expect("BMP fixture");
    fixture.write(
        "坏像素.bmp",
        truncated_pixels(MediaFixtureFormat::Bmp, &bmp),
    );

    let scanned = fixture.scan();
    fixture.assert_path_queue_count(0);
    assert_eq!(valid_names.len(), 28);
    assert_eq!(scanned.snapshot.assets.len(), 29);
    assert_issues(
        &scanned.issues,
        &[
            ("伪装.jpg", "image_format_unsupported"),
            ("空文件.png", "image_format_unsupported"),
            ("坏头.bmp", "image_decode_invalid"),
        ],
    );
    for name in valid_names {
        let location = location_named(&scanned.snapshot, &name);
        assert!(matches!(location.preview_status, PreviewStatus::Pending));
        let ready = fixture
            .preview(&location, false)
            .expect("valid lazy preview");
        fixture.assert_pixels(&ready, QUADRANT_COLORS);
    }
    let corrupt = location_named(&scanned.snapshot, "坏像素.bmp");
    assert!(matches!(corrupt.preview_status, PreviewStatus::Pending));
    let failed = fixture
        .preview(&corrupt, false)
        .expect("isolated pixel failure");
    assert_failed_decode(&failed);
    let persisted = fixture.snapshot();
    assert_eq!(persisted.assets.len(), 29);
    assert_failed_decode(&location_named(&persisted, "坏像素.bmp"));
    assert_eq!(
        persisted
            .assets
            .iter()
            .filter(|asset| matches!(asset.preview_status, PreviewStatus::Ready))
            .count(),
        28
    );
    fixture.assert_sources_unchanged();
}

#[test]
fn same_path_non_image_replacement_retires_ready_preview_and_restores_new_pixels() {
    let valid = encode_rgb_quadrants(MediaFixtureFormat::Bmp, WIDTH, HEIGHT).expect("BMP fixture");
    assert_terminal_replacement(vec![b'x'; valid.len()], "image_format_unsupported");
}

#[test]
fn same_path_truncated_header_is_terminal_instead_of_retaining_old_ready_preview() {
    assert_terminal_replacement(
        truncated_header(MediaFixtureFormat::Bmp),
        "image_decode_invalid",
    );
}

#[test]
fn rejected_input_completed_in_issue_callback_hands_off_to_p0_without_a_second_scan() {
    for (has_baseline, file_name, broken, rejection_code) in [
        (
            false,
            "写入中.bmp",
            truncated_header(MediaFixtureFormat::Bmp),
            "image_decode_invalid",
        ),
        (
            true,
            "写入中.bmp",
            truncated_header(MediaFixtureFormat::Bmp),
            "image_decode_invalid",
        ),
        (
            true,
            "写入中.data",
            b"not image bytes".to_vec(),
            "media_type_unsupported",
        ),
        (
            true,
            "写入中",
            b"not image bytes".to_vec(),
            "media_type_unsupported",
        ),
    ] {
        let mut fixture = MediaInputCatalog::new();
        fixture.write("普通文档.data", b"ordinary document".to_vec());
        fixture.write(
            "保留.png",
            encode_rgb_quadrants(MediaFixtureFormat::Png, WIDTH, HEIGHT).expect("keeper"),
        );
        let old_ready = if has_baseline {
            fixture.write(
                file_name,
                encode_rgb_quadrants(MediaFixtureFormat::Bmp, WIDTH, HEIGHT).expect("old pixels"),
            );
            let initial = fixture.scan();
            let ready = fixture
                .preview(&location_named(&initial.snapshot, file_name), false)
                .expect("initial artifact");
            fixture.assert_pixels(&ready, QUADRANT_COLORS);
            Some(ready)
        } else {
            None
        };
        fixture.write(file_name, broken);
        let completed_bytes = replacement_bytes();
        let target = fixture.source.path().join(file_name);
        fixture
            .sources
            .insert(file_name.to_owned(), completed_bytes.clone());
        let mut rewritten = false;
        let observed = fixture.scan_with_events(|event| {
            if matches!(event, ScanEvent::Issue { issue, .. } if issue.code == rejection_code) {
                assert!(
                    !rewritten,
                    "one rejected observation, not an automatic scan restart"
                );
                fs::write(&target, &completed_bytes)
                    .expect("external writer completes controlled fixture");
                rewritten = true;
            }
        });
        assert!(
            rewritten,
            "mutation occurs after the actual terminal inspection callback"
        );
        assert_eq!(
            observed
                .issues
                .iter()
                .map(|issue| issue.code.as_str())
                .collect::<Vec<_>>(),
            [rejection_code, "source_changed_during_scan"]
        );
        fixture.assert_path_queue_count(1);
        let root_id = &observed.snapshot.roots[0].root_id;
        let mut catalog =
            SqliteCatalog::open(fixture.storage.catalog_path.clone()).expect("P0 catalog");
        let root = catalog
            .load_incremental_catalog_root(root_id)
            .expect("root authority")
            .expect("published root");
        let processed = crate::application::process_ready_library_changes_in_lane(
            &mut catalog,
            root_id,
            root.root_generation,
            LibraryChangeLane::Live,
            super::current_unix_ms()
                .expect("clock")
                .saturating_add(1_000),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("precise live path reinspection");
        assert_eq!(processed.retried_count, 0);
        assert_eq!(processed.completed_count, u32::from(!has_baseline));
        drop(catalog);
        let snapshot = fixture.snapshot();
        assert_eq!(snapshot.assets.len(), 2);
        let ready = fixture
            .preview(&location_named(&snapshot, file_name), false)
            .expect("new bytes preview");
        fixture.assert_pixels(&ready, REPLACEMENT_COLORS);
        if let Some(old) = old_ready {
            fixture.assert_old_preview_unowned(&old);
        }
        let evidence: (i64, i64) = Connection::open_with_flags(
            &fixture.storage.catalog_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("read-only convergence evidence")
        .query_row(
            "SELECT (SELECT COUNT(*) FROM scan_runs),
                    (SELECT COUNT(*) FROM library_change_queue AS change
                     JOIN library_change_queue_lanes AS lane ON lane.change_id = change.id
                     WHERE change.relative_path = ?1 AND change.scope = 'path'
                       AND change.status = 'completed' AND lane.lane = 'p0_live')",
            [file_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("exact scan and P0 ownership");
        assert_eq!(evidence, (if has_baseline { 2 } else { 1 }, 1));
        fixture.assert_sources_unchanged();
    }
}

#[test]
fn same_path_pixel_failure_retries_without_old_artifacts_and_recovers_after_update() {
    let mut fixture = MediaInputCatalog::new();
    let valid = encode_rgb_quadrants(MediaFixtureFormat::Bmp, WIDTH, HEIGHT).expect("BMP fixture");
    fixture.write("替换.png", valid.clone());
    let first = fixture.scan();
    assert!(first.issues.is_empty());
    let ready = fixture
        .preview(&location_named(&first.snapshot, "替换.png"), false)
        .expect("initial preview");
    fixture.assert_pixels(&ready, QUADRANT_COLORS);
    let original_modified = fixture.modified("替换.png");

    fixture.replace_preserving_modified(
        "替换.png",
        truncated_pixels(MediaFixtureFormat::Bmp, &valid),
        original_modified,
    );
    fixture.assert_superseded(&ready, false);
    fixture.assert_superseded(&ready, true);
    let damaged = fixture.scan();
    assert!(damaged.issues.is_empty());
    assert_eq!(damaged.snapshot.assets.len(), 1);
    let pending = location_named(&damaged.snapshot, "替换.png");
    assert_eq!(pending.location_id, ready.location_id);
    assert!(pending.source_generation > ready.source_generation);
    assert!(matches!(pending.preview_status, PreviewStatus::Pending));
    assert!(pending.preview_path.is_empty());
    fixture.assert_old_preview_unowned(&ready);
    let failed = fixture
        .preview(&pending, false)
        .expect("pixel failure is item-local");
    assert_failed_decode(&failed);
    assert_failed_decode(
        &fixture
            .preview(&failed, false)
            .expect("retained decode failure"),
    );
    assert_failed_decode(
        &fixture
            .preview(&failed, true)
            .expect("explicit failed retry"),
    );
    assert_failed_decode(&location_named(&fixture.snapshot(), "替换.png"));

    fixture.replace_preserving_modified("替换.png", replacement_bytes(), original_modified);
    fixture.assert_superseded(&failed, true);
    let restored = fixture.scan();
    assert!(restored.issues.is_empty());
    assert_eq!(restored.snapshot.assets.len(), 1);
    let restored_location = location_named(&restored.snapshot, "替换.png");
    assert!(restored_location.source_generation > failed.source_generation);
    assert!(matches!(
        restored_location.preview_status,
        PreviewStatus::Pending
    ));
    assert!(restored_location.preview_issue_code.is_none());
    assert!(restored_location.preview_issue_message.is_none());
    fixture.assert_superseded(&failed, true);
    let restored_ready = fixture
        .preview(&restored_location, false)
        .expect("new revision does not inherit failure");
    fixture.assert_pixels(&restored_ready, REPLACEMENT_COLORS);
    fixture.assert_old_preview_unowned(&ready);
}

fn assert_terminal_replacement(bad_bytes: Vec<u8>, expected_issue: &str) {
    let mut fixture = MediaInputCatalog::new();
    fixture.write(
        "替换.png",
        encode_rgb_quadrants(MediaFixtureFormat::Bmp, WIDTH, HEIGHT).expect("initial BMP"),
    );
    fixture.write(
        "保留.data",
        encode_rgb_quadrants(MediaFixtureFormat::Png, WIDTH, HEIGHT).expect("unaffected PNG"),
    );
    let first = fixture.scan();
    assert_eq!(first.snapshot.assets.len(), 2);
    assert!(first.issues.is_empty());
    let old_ready = fixture
        .preview(&location_named(&first.snapshot, "替换.png"), false)
        .expect("old ready preview");
    let untouched = fixture
        .preview(&location_named(&first.snapshot, "保留.data"), false)
        .expect("unaffected preview");
    fixture.assert_pixels(&old_ready, QUADRANT_COLORS);
    let original_modified = fixture.modified("替换.png");

    fixture.replace_preserving_modified("替换.png", bad_bytes, original_modified);
    fixture.assert_superseded(&old_ready, false);
    fixture.assert_superseded(&old_ready, true);
    let invalid = fixture.scan();
    assert_eq!(invalid.snapshot.assets.len(), 1);
    assert_issues(&invalid.issues, &[("替换.png", expected_issue)]);
    let unaffected = location_named(&invalid.snapshot, "保留.data");
    assert_eq!(unaffected.source_generation, untouched.source_generation);
    fixture.assert_pixels(&unaffected, QUADRANT_COLORS);
    fixture.assert_old_preview_unowned(&old_ready);
    assert_eq!(
        fixture
            .preview(&old_ready, true)
            .expect_err("removed location cannot retry the old artifact")
            .code,
        "preview_location_not_found"
    );

    fixture.replace_preserving_modified("替换.png", replacement_bytes(), original_modified);
    let restored = fixture.scan();
    assert!(restored.issues.is_empty());
    assert_eq!(restored.snapshot.assets.len(), 2);
    let pending = location_named(&restored.snapshot, "替换.png");
    assert_eq!(pending.location_id, old_ready.location_id);
    assert!(pending.source_generation > old_ready.source_generation);
    assert!(matches!(pending.preview_status, PreviewStatus::Pending));
    assert!(pending.preview_path.is_empty());
    assert!(pending.preview_issue_code.is_none());
    fixture.assert_superseded(&old_ready, true);
    let restored_ready = fixture
        .preview(&pending, false)
        .expect("restored source preview");
    fixture.assert_pixels(&restored_ready, REPLACEMENT_COLORS);
    let retried = fixture
        .preview(&restored_ready, true)
        .expect("explicit preview regeneration");
    fixture.assert_pixels(&retried, REPLACEMENT_COLORS);
    fixture.assert_old_preview_unowned(&old_ready);
}

fn replacement_bytes() -> Vec<u8> {
    encode_rgb_quadrants_with_colors(MediaFixtureFormat::Bmp, WIDTH, HEIGHT, REPLACEMENT_COLORS)
        .expect("changed BMP pixels")
}

fn location_named(snapshot: &CatalogSnapshot, name: &str) -> AssetLocationView {
    snapshot
        .assets
        .iter()
        .find(|location| location.relative_path == name)
        .expect("published fixture location")
        .clone()
}

fn assert_issues(issues: &[ScanIssue], expected: &[(&str, &str)]) {
    let actual = issues
        .iter()
        .map(|issue| {
            assert!(!issue.message.is_empty(), "actionable issue message");
            let name = Path::new(issue.path.as_deref().expect("issue source path"))
                .file_name()
                .expect("issue filename")
                .to_str()
                .expect("fixture Unicode");
            (name, issue.code.as_str())
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(issues.len(), expected.len(), "one issue per rejected input");
    assert_eq!(actual, expected.iter().copied().collect());
}

fn assert_failed_decode(location: &AssetLocationView) {
    assert!(matches!(location.preview_status, PreviewStatus::Failed));
    assert!(location.preview_path.is_empty());
    assert_eq!(
        location.preview_issue_code.as_deref(),
        Some("image_decode_failed")
    );
    assert!(
        location
            .preview_issue_message
            .as_ref()
            .is_some_and(|message| !message.is_empty())
    );
}

struct ScanObservation {
    snapshot: CatalogSnapshot,
    issues: Vec<ScanIssue>,
}

struct MediaInputCatalog {
    source: TempDir,
    _derived: TempDir,
    storage: StoragePaths,
    sources: BTreeMap<String, Vec<u8>>,
    next_scan: u32,
}

impl MediaInputCatalog {
    fn new() -> Self {
        let source = tempdir().expect("disposable media source");
        let derived = tempdir().expect("isolated derived storage");
        let storage = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        Self {
            source,
            _derived: derived,
            storage,
            sources: BTreeMap::new(),
            next_scan: 0,
        }
    }

    fn write(&mut self, name: &str, bytes: Vec<u8>) {
        fs::write(self.source.path().join(name), &bytes).expect("write owned source fixture");
        self.sources.insert(name.to_owned(), bytes);
    }

    fn modified(&self, name: &str) -> SystemTime {
        fs::metadata(self.source.path().join(name))
            .expect("source metadata")
            .modified()
            .expect("source modified time")
    }

    fn replace_preserving_modified(&mut self, name: &str, bytes: Vec<u8>, modified: SystemTime) {
        self.write(name, bytes);
        File::options()
            .write(true)
            .open(self.source.path().join(name))
            .expect("owned fixture timestamp handle")
            .set_times(FileTimes::new().set_modified(modified))
            .expect("restore fixture modified time");
        assert_eq!(self.modified(name), modified);
    }

    fn scan(&mut self) -> ScanObservation {
        self.scan_with_events(|_| {})
    }

    fn scan_with_events(&mut self, mut observe: impl FnMut(&ScanEvent)) -> ScanObservation {
        self.next_scan += 1;
        let identity = stable_id("media-input-fixture", &self.source.path().to_string_lossy());
        let scan_id = format!("media-input-{identity}-{}", self.next_scan);
        let mut events = Vec::new();
        run_scan_with_storage(
            ScanRequest {
                scan_id: scan_id.clone(),
                root_path: self.source.path().to_string_lossy().into_owned(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |event| {
                observe(&event);
                events.push(event);
                true
            },
            self.storage.clone(),
        )
        .expect("production scan completes with item-local media failures");
        let Some(ScanEvent::Completed {
            asset_count,
            issue_count,
            was_limited,
            ..
        }) = events.last()
        else {
            panic!(
                "media scan must publish completion, got {:?}",
                events.last()
            );
        };
        assert!(!was_limited);
        let issues = events
            .iter()
            .filter_map(|event| match event {
                ScanEvent::Issue { issue, .. } => Some(issue.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let snapshot = self.snapshot();
        assert_eq!(*asset_count, snapshot.assets.len() as u64);
        assert_eq!(*issue_count, issues.len() as u64);
        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(
            snapshot.roots[0].active_scan_id.as_deref(),
            Some(scan_id.as_str())
        );
        assert_eq!(snapshot.roots[0].issue_count, *issue_count);
        self.assert_sources_unchanged();
        ScanObservation { snapshot, issues }
    }

    fn snapshot(&self) -> CatalogSnapshot {
        let snapshot = SqliteCatalog::open(self.storage.catalog_path.clone())
            .expect("reopen authoritative catalog")
            .load_snapshot(
                128,
                &GalleryQuery::default(),
                "media-input-query",
                None,
                None,
                None,
            )
            .expect("bounded published snapshot");
        assert!(snapshot.next_cursor.is_none());
        snapshot
    }

    fn preview(
        &self,
        location: &AssetLocationView,
        retry_failed: bool,
    ) -> Result<AssetLocationView, ScanError> {
        let result = materialize_preview_with_storage(
            PreviewRequest {
                location_id: location.location_id.clone(),
                expected_root_id: location.root_id.clone(),
                expected_scan_id: location.scan_id.clone(),
                expected_source_revision: location.source_revision.clone(),
                expected_source_generation: location.source_generation,
                preview_edge: 128,
                retry_failed,
                protected_location_ids: Vec::new(),
            },
            self.storage.clone(),
        );
        self.assert_sources_unchanged();
        result
    }

    fn assert_superseded(&self, old: &AssetLocationView, retry: bool) {
        assert_eq!(
            self.preview(old, retry)
                .expect_err("changed source cannot reuse an old request")
                .code,
            "preview_request_superseded"
        );
    }

    fn assert_old_preview_unowned(&self, old: &AssetLocationView) {
        let owners: i64 = Connection::open_with_flags(
            &self.storage.catalog_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("read-only preview ownership query")
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations AS owner
                 JOIN preview_artifacts AS artifact ON artifact.artifact_key = owner.artifact_key
                 WHERE owner.location_id = ?1 AND artifact.artifact_path = ?2",
            params![old.location_id, old.preview_path],
            |row| row.get(0),
        )
        .expect("old preview ownership");
        assert_eq!(owners, 0);
    }

    fn assert_path_queue_count(&self, expected: i64) {
        let count: i64 = Connection::open_with_flags(
            &self.storage.catalog_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("read-only path queue")
        .query_row(
            "SELECT COUNT(*) FROM library_change_queue WHERE scope = 'path'",
            [],
            |row| row.get(0),
        )
        .expect("exact path queue count");
        assert_eq!(count, expected);
    }

    fn assert_pixels(&self, location: &AssetLocationView, colors: [[u8; 3]; 4]) {
        assert!(matches!(location.preview_status, PreviewStatus::Ready));
        assert!(location.preview_issue_code.is_none());
        assert!(location.preview_issue_message.is_none());
        assert_eq!((location.width, location.height), (WIDTH, HEIGHT));
        assert!(location.source_revision.is_some(), "ready source revision");
        let generation = i64::try_from(location.source_generation)
            .expect("ready generation fits the catalog integer range");
        let owners: i64 = Connection::open_with_flags(&self.storage.catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read-only ready ownership").query_row(
                "SELECT COUNT(*) FROM preview_artifact_locations AS owner
                 JOIN preview_artifacts AS artifact ON artifact.artifact_key = owner.artifact_key
                 JOIN asset_locations AS location ON location.location_id = owner.location_id
                 JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
                 WHERE owner.location_id = ?1 AND artifact.artifact_path = ?2
                   AND artifact.lifecycle_state = 'ready' AND artifact.source_generation = ?3
                   AND artifact.source_revision_token = location.source_revision_token AND location.scan_id = ?4
                   AND location.preview_status = 'ready' AND location.preview_path = artifact.artifact_path
                   AND location.source_generation = artifact.source_generation",
                params![location.location_id, location.preview_path, generation, location.scan_id], |row| row.get(0),
            ).expect("positive ready artifact and source ownership");
        assert_eq!(
            owners, 1,
            "ready evidence must have one live matching artifact owner before testing detachment"
        );
        let path = Path::new(&location.preview_path);
        assert!(path.starts_with(&self.storage.preview_root));
        assert!(!path.starts_with(self.source.path()));
        let pixels = image::open(path)
            .expect("actual preview artifact decodes")
            .to_rgb8();
        assert_eq!(pixels.dimensions(), (WIDTH, HEIGHT));
        for (index, (x, y)) in [
            (WIDTH / 4, HEIGHT / 4),
            (3 * WIDTH / 4, HEIGHT / 4),
            (WIDTH / 4, 3 * HEIGHT / 4),
            (3 * WIDTH / 4, 3 * HEIGHT / 4),
        ]
        .into_iter()
        .enumerate()
        {
            let actual = pixels.get_pixel(x, y).0;
            for (actual, expected) in actual.into_iter().zip(colors[index]) {
                assert!(
                    actual.abs_diff(expected) <= 20,
                    "preview quadrant {index}: actual {actual}, expected {expected}"
                );
            }
        }
    }

    fn assert_sources_unchanged(&self) {
        assert_eq!(
            fs::read_dir(self.source.path())
                .expect("fixture source entries")
                .count(),
            self.sources.len()
        );
        for (name, bytes) in &self.sources {
            assert_eq!(
                &fs::read(self.source.path().join(name)).expect("source bytes after operation"),
                bytes,
                "source changed: {name}"
            );
        }
    }
}
