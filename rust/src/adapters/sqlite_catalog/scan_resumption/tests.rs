use super::*;
use crate::domain::{
    AssetLocationView, JournalFileReference, JournalIdentifier, JournalUsn,
    LibraryChangeQueuePolicy, LibraryRecoveryAuthorityReason,
    PersistentJournalBaselineStartRequest, PersistentJournalVolumeIdentity, PreviewStatus,
    SourceRevisionEvidence,
};
use crate::ports::{CatalogRepository, PersistentJournalRepository};

struct Fixture {
    _directory: tempfile::TempDir,
    catalog: SqliteCatalog,
    request: ScanRequest,
    identity: FileIdentityEvidence,
    baseline: PersistentJournalBaseline,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("derived fixture");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        let request = ScanRequest {
            scan_id: "resume-coverage".to_owned(),
            root_path: "C:\\ResumeCoverage".to_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let identity = FileIdentityEvidence {
            scheme: "windows-file-id-128-v1".to_owned(),
            value: "0000000000000007:00000000000000000000000000000001".to_owned(),
        };
        catalog
            .begin_scan_with_publication_namespace(&request, "root", &request.root_path, &identity)
            .expect("bound import");
        let baseline = catalog
            .begin_persistent_journal_baseline(
                &PersistentJournalBaselineStartRequest {
                    run_id: request.scan_id.clone(),
                    root_id: "root".to_owned(),
                    root_generation: LibraryRootGeneration::initial(),
                    authority_reason: LibraryRecoveryAuthorityReason::FirstImportBoundary,
                    volume: PersistentJournalVolumeIdentity {
                        volume_guid: "volume-a".to_owned(),
                        volume_serial: 7,
                    },
                    root_file_reference: JournalFileReference::V3(1_u128.to_le_bytes()),
                    journal_id: JournalIdentifier::new(9).expect("journal"),
                    opening_next_usn: JournalUsn::new(20).expect("opening"),
                    protocol_version: 1,
                    contract_version: 1,
                    authorized_unix_ms: 1_000,
                },
                LibraryChangeQueuePolicy::default(),
            )
            .expect("opening boundary");
        catalog
            .claim_next_directory(&request.scan_id)
            .expect("root");
        catalog
            .stage_directory_entries(
                &request.scan_id,
                "",
                &["a.png".to_owned(), "b.png".to_owned()],
            )
            .expect("directory entries");
        catalog
            .complete_directory_with_identity(&request.scan_id, "", Some(&identity))
            .expect("complete enumeration");
        let asset = AssetLocationView {
            asset_id: "asset-a".to_owned(),
            location_id: "location-a".to_owned(),
            root_id: "root".to_owned(),
            scan_id: request.scan_id.clone(),
            absolute_path: format!("{}\\a.png", request.root_path),
            display_path: format!("{}\\a.png", request.root_path),
            relative_path: "a.png".to_owned(),
            preview_path: String::new(),
            file_size: 20,
            created_unix_ms: None,
            modified_unix_ms: 30,
            file_identity: Some(FileIdentityEvidence {
                scheme: identity.scheme.clone(),
                value: "0000000000000007:00000000000000000000000000000002".to_owned(),
            }),
            source_revision: Some(SourceRevisionEvidence {
                scheme: "windows-file-change-time-100ns-v1".to_owned(),
                value: "0000000000000001".to_owned(),
            }),
            source_generation: 0,
            width: 40,
            height: 50,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture-metadata".to_owned(),
            metadata_engine_version: "1".to_owned(),
            camera: Default::default(),
            capture_time: None,
        };
        catalog
            .stage_location(&request.scan_id, "root", &asset)
            .expect("stage inspected image");
        catalog
            .pause_scan(
                &request.scan_id,
                &ScanCheckpoint {
                    last_visited_relative_path: Some("a.png".to_owned()),
                    visited_entries: 1,
                    accepted_items: 1,
                    ..ScanCheckpoint::default()
                },
            )
            .expect("durable pause");
        Self {
            _directory: directory,
            catalog,
            request,
            identity,
            baseline,
        }
    }

    fn coverage(&self) -> ScanResumeCoverage {
        ScanResumeCoverage::verify(
            self.baseline.clone(),
            &self.baseline.volume,
            &self.baseline.root_file_reference,
            self.baseline.journal_id,
            JournalUsn::new(0).expect("first"),
            JournalUsn::new(50).expect("next"),
        )
        .expect("proof")
        .expect("covered interval")
    }

    fn resume(
        &mut self,
        coverage: Option<&ScanResumeCoverage>,
    ) -> Result<ScanCheckpoint, ScanError> {
        self.catalog.resume_scan_with_publication_namespace(
            &self.request,
            "root",
            &self.request.root_path,
            &self.identity,
            coverage,
        )
    }
}

#[test]
fn covered_resume_keeps_counts_directory_position_and_original_opening() {
    let mut fixture = Fixture::new();
    let coverage = fixture.coverage();
    let checkpoint = fixture
        .resume(Some(&coverage))
        .expect("resume covered import");
    assert_eq!(
        (checkpoint.visited_entries, checkpoint.accepted_items),
        (1, 1)
    );
    assert_eq!(
        checkpoint.last_visited_relative_path.as_deref(),
        Some("a.png")
    );
    assert!(
        fixture
            .catalog
            .is_current_directory_enumerated(&fixture.request.scan_id, "")
            .expect("enumerated")
    );
    assert_eq!(
        fixture
            .catalog
            .load_directory_entry_window(
                &fixture.request.scan_id,
                "",
                checkpoint.last_visited_relative_path.as_deref(),
                16,
            )
            .expect("remaining entries"),
        vec!["b.png"]
    );
    assert_eq!(
        fixture
            .catalog
            .load_first_import_resume_baseline(&fixture.request.scan_id, "root")
            .expect("opening"),
        Some(fixture.baseline)
    );
    fixture
        .catalog
        .pause_scan(&fixture.request.scan_id, &checkpoint)
        .expect("pause again");
    let path = fixture.catalog.catalog_path().to_owned();
    drop(fixture.catalog);
    let mut reopened = SqliteCatalog::open(path).expect("complete catalog validation");
    let retained = reopened
        .load_single_paused_foreground_scan()
        .expect("checkpoint")
        .expect("paused");
    assert_eq!((retained.visited_entries, retained.accepted_items), (1, 1));
}

#[test]
fn stale_coverage_rolls_back_resume_and_preserves_paused_work() {
    let mut fixture = Fixture::new();
    let coverage = fixture.coverage();
    fixture
        .catalog
        .connection
        .execute(
            "UPDATE library_persistent_journal_baselines SET updated_unix_ms = updated_unix_ms + 1",
            [],
        )
        .expect("concurrent boundary change");
    assert_eq!(
        fixture
            .resume(Some(&coverage))
            .expect_err("stale proof")
            .code,
        "catalog_scan_resume_coverage_stale"
    );
    let retained = fixture
        .catalog
        .load_single_paused_foreground_scan()
        .expect("checkpoint")
        .expect("still paused");
    assert_eq!((retained.visited_entries, retained.accepted_items), (1, 1));
    assert_eq!(
        fixture
            .catalog
            .count_staged_file_states(&fixture.request.scan_id)
            .expect("retained image"),
        1
    );
}

#[test]
fn uncovered_resume_reconciles_membership_while_retaining_inspected_images() {
    let mut fixture = Fixture::new();
    let checkpoint = fixture
        .resume(None)
        .expect("unverified namespace reconciliation");
    assert_eq!(
        (checkpoint.visited_entries, checkpoint.accepted_items),
        (0, 0)
    );
    assert_eq!(
        fixture
            .catalog
            .count_staged_file_states(&fixture.request.scan_id)
            .expect("retained image"),
        1
    );
    assert!(
        fixture
            .catalog
            .load_first_import_resume_baseline(&fixture.request.scan_id, "root")
            .expect("retired boundary")
            .is_none()
    );
    assert_eq!(
        fixture
            .catalog
            .claim_next_directory(&fixture.request.scan_id)
            .expect("fresh frontier"),
        Some(String::new())
    );
    assert!(
        !fixture
            .catalog
            .is_current_directory_enumerated(&fixture.request.scan_id, "")
            .expect("fresh enumeration")
    );
}
