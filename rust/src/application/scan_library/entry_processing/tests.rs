use std::cell::Cell;

use tempfile::{TempDir, tempdir};

use crate::domain::{
    DiscoveredFile, FileIdentityEvidence, MediaInspection, MetadataInspection, ScanIssue,
    ScanRequest, SourceRevisionEvidence,
};
use crate::ports::{MediaInspectionFailure, MediaInspectionFailureKind};

use super::super::finalization::FinalizationMode;
use super::*;

#[test]
fn discovery_issue_detaches_before_inspection_or_staging() {
    let mut fixture = EntryFixture::new();
    let mut file = fixture.file("image.png");
    file.issues.push(issue("discovery-warning"));
    let inspector = RecordingInspector::success(Vec::new());
    let mut published = Vec::new();
    let outcome = fixture.apply(file, &inspector, &mut |event| {
        published.push(event);
        false
    });
    assert!(matches!(outcome, ScanEntryOutcome::Detached));
    assert_eq!(inspector.calls.get(), 0);
    fixture.assert_unstaged_issue(&published, "discovery-warning");
}

#[test]
fn metadata_issue_detaches_before_staging_and_preserves_the_checkpoint() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(vec![issue("metadata-warning")]);
    let mut published = Vec::new();
    let outcome = fixture.apply(fixture.file("image.png"), &inspector, &mut |event| {
        published.push(event);
        false
    });
    assert!(matches!(outcome, ScanEntryOutcome::Detached));
    assert_eq!(inspector.calls.get(), 1);
    fixture.assert_unstaged_issue(&published, "metadata-warning");
}

#[test]
fn applied_entry_stages_after_issue_delivery_without_advancing_traversal_checkpoint() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(vec![issue("metadata-warning")]);
    let mut published = Vec::new();
    let outcome = fixture.apply(fixture.file("image.png"), &inspector, &mut |event| {
        published.push(event);
        true
    });
    let ScanEntryOutcome::Applied {
        accepted_items,
        event: Some(ScanEvent::AssetDiscovered { asset, .. }),
    } = outcome
    else {
        panic!("expected one staged asset");
    };
    assert_eq!(accepted_items, 1);
    assert_eq!(asset.width, 37);
    assert_eq!(fixture.staged_count(), 1);
    assert_eq!(fixture.issue_count, 1);
    assert_eq!(published.len(), 1);
    assert_eq!(fixture.checkpoint.visited_entries, 0);
    assert_eq!(fixture.checkpoint.accepted_items, 0);
    assert_eq!(fixture.checkpoint.last_visited_relative_path, None);
}

#[test]
fn retryable_inspection_keeps_precise_retry_state_without_inventing_an_asset() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector {
        calls: Cell::new(0),
        result: Err(MediaInspectionFailure {
            kind: MediaInspectionFailureKind::Retryable,
            issue: issue("source-locked"),
        }),
    };
    let outcome = fixture.apply(fixture.file("image.png"), &inspector, &mut |_| true);
    let ScanEntryOutcome::Applied {
        accepted_items,
        event: Some(ScanEvent::Issue { issue, .. }),
    } = outcome
    else {
        panic!("expected a retained inspection issue");
    };
    assert_eq!(accepted_items, 0);
    assert_eq!(issue.code, "source-locked");
    assert_eq!(fixture.staged_count(), 0);
    assert_eq!(fixture.checkpoint.issue_count, 1);
    assert_eq!(fixture.checkpoint.accepted_items, 0);
    assert!(!fixture.checkpoint.requires_previous_snapshot);
    assert!(fixture.finalization.has_retained_rejection("image.png"));
}

#[test]
fn same_identity_at_another_path_reuses_metadata_and_source_generation() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    let prior = fixture.stage_prior(&inspector);
    let prepared = PreparedScanFile::load(
        &fixture.catalog,
        &inspector,
        "entry-scan",
        "entry-root",
        fixture.file("renamed.png"),
        false,
    )
    .expect("prepare same identity");
    assert_eq!(prepared.preservation_prior(), Some(&prior));
    let inspection = prepared.inspect(&inspector).expect("reuse metadata");
    assert_eq!(inspector.calls.get(), 1);
    let asset = prepared.into_asset(inspection);
    assert_eq!(asset.asset_id, prior.asset_id);
    assert_ne!(asset.location_id, prior.location_id);
    assert_eq!(asset.relative_path, "renamed.png");
    assert_eq!(asset.source_generation, prior.source_generation);
    assert!(asset.source_generation > 0);
}

#[test]
fn changed_source_preserves_identity_for_failure_without_reusing_metadata() {
    let mut fixture = EntryFixture::new();
    let inspector = RecordingInspector::success(Vec::new());
    let prior = fixture.stage_prior(&inspector);
    let mut file = fixture.file("renamed.png");
    file.source_revision
        .as_mut()
        .expect("source revision")
        .value = "0000000000000002".to_owned();
    let prepared = PreparedScanFile::load(
        &fixture.catalog,
        &inspector,
        "entry-scan",
        "entry-root",
        file,
        false,
    )
    .expect("prepare changed source");
    assert_eq!(prepared.preservation_prior(), Some(&prior));
    let inspection = prepared
        .inspect(&inspector)
        .expect("inspect changed source");
    assert_eq!(inspector.calls.get(), 2);
    let asset = prepared.into_asset(inspection);
    assert_eq!(asset.asset_id, prior.asset_id);
    assert_eq!(asset.source_generation, 0);
}

struct EntryFixture {
    _storage: TempDir,
    source: TempDir,
    catalog: SqliteCatalog,
    checkpoint: ScanCheckpoint,
    finalization: FinalizationPlan,
    issue_count: u64,
}

impl EntryFixture {
    fn new() -> Self {
        let storage = tempdir().expect("derived fixture storage");
        let source = tempdir().expect("owned source namespace");
        let mut catalog =
            SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("fixture catalog");
        let root_path = source.path().to_string_lossy().into_owned();
        let request = ScanRequest {
            scan_id: "entry-scan".to_owned(),
            root_path: root_path.clone(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let checkpoint = catalog
            .begin_scan(&request, "entry-root", &root_path)
            .expect("begin fixture");
        Self {
            _storage: storage,
            source,
            catalog,
            checkpoint,
            finalization: FinalizationPlan::new(FinalizationMode::Foreground),
            issue_count: 0,
        }
    }

    fn file(&self, name: &str) -> DiscoveredFile {
        DiscoveredFile {
            source_root_path: self.source.path().to_string_lossy().into_owned(),
            absolute_path: self.source.path().join(name).to_string_lossy().into_owned(),
            relative_path: name.to_owned(),
            file_size: 64,
            created_unix_ms: None,
            modified_unix_ms: 1234,
            file_identity: Some(FileIdentityEvidence {
                scheme: "windows-file-id-128-v1".to_owned(),
                value: "0000000000000001:00000000000000000000000000000001".to_owned(),
            }),
            source_revision: Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 0,
            issues: Vec::new(),
        }
    }

    fn apply(
        &mut self,
        file: DiscoveredFile,
        inspector: &RecordingInspector,
        publish: &mut impl FnMut(ScanEvent) -> bool,
    ) -> ScanEntryOutcome {
        let relative_path = file.relative_path.clone();
        apply_scan_entry(
            &mut self.catalog,
            &mut self.finalization,
            inspector,
            ScanEntryContext {
                scan_id: "entry-scan",
                root_id: "entry-root",
                relative_path: &relative_path,
                had_published_root: false,
                has_active_locations: false,
                accepted_items: 0,
                outcome: FileVisitOutcome::File(file),
            },
            &mut self.checkpoint,
            &mut self.issue_count,
            publish,
        )
        .expect("apply observed entry")
    }

    fn stage_prior(&mut self, inspector: &RecordingInspector) -> crate::domain::AssetLocationView {
        let ScanEntryOutcome::Applied {
            event: Some(ScanEvent::AssetDiscovered { asset, .. }),
            ..
        } = self.apply(self.file("image.png"), inspector, &mut |_| true)
        else {
            panic!("expected original asset");
        };
        self.catalog
            .prove_live_only_first_import_handoff_for_test("entry-scan")
            .expect("establish the existing fixture publication boundary");
        self.catalog
            .publish_scan("entry-scan", "entry-root", 1, 0)
            .expect("publish prior fixture");
        self.catalog
            .load_active_location(&asset.location_id)
            .expect("read published prior")
            .expect("published prior")
    }

    fn staged_count(&mut self) -> u64 {
        self.catalog
            .count_staged_file_states("entry-scan")
            .expect("flush and count staged entries")
    }

    fn assert_unstaged_issue(&mut self, published: &[ScanEvent], code: &str) {
        assert_eq!(self.staged_count(), 0);
        assert_eq!(self.issue_count, 1);
        assert_eq!(self.checkpoint.visited_entries, 0);
        assert_eq!(self.checkpoint.accepted_items, 0);
        assert_eq!(self.checkpoint.last_visited_relative_path, None);
        assert!(matches!(published, [ScanEvent::Issue { issue, .. }] if issue.code == code));
    }
}

struct RecordingInspector {
    calls: Cell<usize>,
    result: Result<MediaInspection, MediaInspectionFailure>,
}

impl RecordingInspector {
    fn success(issues: Vec<ScanIssue>) -> Self {
        Self {
            calls: Cell::new(0),
            result: Ok(MediaInspection {
                width: 37,
                height: 19,
                metadata: MetadataInspection {
                    engine_id: "fixture-metadata".to_owned(),
                    engine_version: "1".to_owned(),
                    capture_time: None,
                    issues,
                },
            }),
        }
    }
}

impl MediaInspector for RecordingInspector {
    fn metadata_engine_id(&self) -> &'static str {
        "fixture-metadata"
    }
    fn metadata_engine_version(&self) -> &'static str {
        "1"
    }
    fn inspection_engine_id(&self) -> &'static str {
        "fixture-inspection"
    }
    fn inspection_engine_version(&self) -> u32 {
        1
    }
    fn inspect(&self, _: &DiscoveredFile) -> Result<MediaInspection, MediaInspectionFailure> {
        self.calls.set(self.calls.get() + 1);
        self.result.clone()
    }
}

fn issue(code: &str) -> ScanIssue {
    ScanIssue {
        path: Some("image.png".to_owned()),
        code: code.to_owned(),
        message: "controlled entry observation".to_owned(),
    }
}
