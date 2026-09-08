use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;

use image::{Rgb, RgbImage};
use tempfile::{TempDir, tempdir};

use crate::adapters::{
    FileDiscovery, LocalMetadataInventory, PublicationGuardedFileDiscovery, SqliteCatalog,
    reset_source_enumeration_instrumentation, reset_source_root_entry_enumeration_count,
    set_before_metadata_inventory_spool_commit_hook, source_directory_open_count,
    source_entry_read_count, source_peak_staged_window, source_root_entry_enumeration_count,
    source_spool_open_count,
};
use crate::application::StoragePaths;
use crate::application::scan_library::{run_scan_with_storage, stable_id};
use crate::application::{
    process_ready_library_changes, process_ready_metadata_inventory_recovery_candidates_cancellable,
};
use crate::domain::{
    FileIdentityEvidence, IncrementalCatalogRoot, JournalFileReference, JournalIdentifier,
    JournalUsn, LeasedLibraryChange, LibraryChangeFailure, LibraryChangeId, LibraryChangeIntent,
    LibraryChangeIntentKind, LibraryChangeOrigin, LibraryChangeQueuePolicy, LibraryChangeScope,
    LibraryRecoveryAuthority, LibraryRecoveryAuthorityReason, LibraryRootGeneration,
    MetadataInventoryEntry, MetadataInventoryEntryKind, MetadataInventoryFrontierEntry,
    MetadataInventoryPage, MetadataInventoryPlaceholderState, MetadataInventoryRun,
    MetadataInventoryRunRequest, MetadataInventoryRunStatus, MetadataInventoryScope,
    MetadataInventoryStartRequest, PersistentJournalBaselineClosingBoundary,
    PersistentJournalBaselineStartRequest, PersistentJournalCapability,
    PersistentJournalCapabilityState, PersistentJournalContinuityState,
    PersistentJournalVolumeIdentity, PreviewStatus, ScanError, ScanRequest,
};
use crate::ports::{
    CatalogRepository, IncrementalCatalogRepository, LibraryChangeQueue,
    MetadataInventoryRepository, MetadataInventorySource, MetadataInventorySourcePreparation,
    PersistentJournalRepository,
};

use super::{
    MetadataInventoryProgressPhase, MetadataInventoryRecoveryExecution,
    MetadataInventoryWorkerControl, leased_change_requires_metadata_inventory,
    metadata_inventory_run_id, process_leased_metadata_inventory_change,
    process_leased_metadata_inventory_change_with_retained_source,
    recovery_reason_invalidates_unchanged_sources, retry_terminalization, run_metadata_inventory,
    set_before_metadata_inventory_finalization_hook,
};

mod lifecycle_admission;
mod spool_lifecycle;
mod terminal_media;

#[test]
fn recovery_reason_invalidation_matrix_covers_every_authority_reason() {
    let cases = [
        (LibraryRecoveryAuthorityReason::ExistingRootBaseline, false),
        (LibraryRecoveryAuthorityReason::FirstImportBoundary, false),
        (LibraryRecoveryAuthorityReason::JournalGap, true),
        (LibraryRecoveryAuthorityReason::JournalReset, true),
        (LibraryRecoveryAuthorityReason::JournalTrim, true),
        (
            LibraryRecoveryAuthorityReason::JournalReconstructionFailure,
            true,
        ),
        (LibraryRecoveryAuthorityReason::ContainmentFailure, true),
        (
            LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure,
            true,
        ),
        (LibraryRecoveryAuthorityReason::WatcherUncoveredGap, true),
    ];

    for (reason, expected) in cases {
        assert_eq!(
            recovery_reason_invalidates_unchanged_sources(reason),
            expected,
            "unexpected invalidation policy for {reason:?}"
        );
    }
}

#[test]
fn closed_process_metadata_changes_converge_through_bounded_inventory_pages() {
    let mut fixture = InventoryFixture::new(&[
        "keep.png",
        "modify.png",
        "delete.png",
        "rename.png",
        "album/moved.png",
    ]);
    write_png(
        &fixture.source.path().join("modify.png"),
        4,
        3,
        [90, 80, 70],
    );
    fs::remove_file(fixture.source.path().join("delete.png")).expect("delete fixture");
    fs::rename(
        fixture.source.path().join("rename.png"),
        fixture.source.path().join("renamed.png"),
    )
    .expect("rename fixture");
    fs::rename(
        fixture.source.path().join("album"),
        fixture.source.path().join("moved-album"),
    )
    .expect("move directory fixture");
    write_png(
        &fixture.source.path().join("新增图片.png"),
        3,
        2,
        [20, 40, 60],
    );
    let long_name = format!("{}.png", "长路径".repeat(30));
    write_png(&fixture.source.path().join(&long_name), 2, 3, [60, 40, 20]);

    let report = fixture.run_inventory(2, &AtomicBool::new(false));

    assert!(report.is_complete);
    assert!(report.staged_entry_count >= 7);
    assert!(report.candidate_count >= 6);
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process inventory candidates");
    assert!(incremental.completed_count >= 6);
    assert!(fixture.location("keep.png").is_some());
    assert_eq!(
        fixture
            .location("modify.png")
            .map(|location| (location.width, location.height)),
        Some((4, 3))
    );
    assert!(fixture.location("delete.png").is_none());
    assert!(fixture.location("rename.png").is_none());
    assert!(fixture.location("renamed.png").is_some());
    assert!(fixture.location("album/moved.png").is_none());
    assert!(fixture.location("moved-album/moved.png").is_some());
    assert!(fixture.location("新增图片.png").is_some());
    assert!(fixture.location(&long_name).is_some());
}

#[test]
fn durable_spool_reads_a_ten_thousand_entry_flat_directory_once_across_page_reopens() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..10_000_u32 {
        fs::write(
            fixture.source.path().join(format!("entry-{index:05}.jpg")),
            b"metadata",
        )
        .expect("flat source entry");
    }
    let leased = enqueue_authorized_recovery_control(&mut fixture, "spool-10k", 4_000);
    let request = MetadataInventoryStartRequest {
        run_id: "spool-10k".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 4_000,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin durable inventory");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    let mut raw_yields = 0_u32;
    loop {
        match source
            .prepare_next_page(4_095, &cancellation)
            .expect("prepare bounded raw batch")
        {
            MetadataInventorySourcePreparation::Ready => break,
            MetadataInventorySourcePreparation::Yielded => {
                raw_yields = raw_yields.saturating_add(1);
            }
        }
    }
    assert!(raw_yields > 1);
    assert_eq!(source_entry_read_count(&fixture.root_path), 10_000);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
    assert!(source_peak_staged_window(&fixture.root_path) <= 128);
    let page_plan = fixture
        .catalog
        .metadata_inventory_spool_page_query_plan("spool-10k", Some("entry-04094.jpg"), 4_095)
        .expect("indexed spool page plan");
    assert_eq!(page_plan.len(), 1);
    assert!(page_plan[0].contains("library_metadata_inventory_spool_entries_order"));
    assert!(page_plan[0].contains("run_id=? AND relative_path>?"));
    assert!(!page_plan[0].contains("SCAN"));

    let mut staged = 0_usize;
    let mut output_pages = 0_u32;
    loop {
        let page = source
            .next_page(4_095, &cancellation)
            .expect("load durable output page");
        staged = staged.saturating_add(page.entries.len());
        output_pages = output_pages.saturating_add(1);
        let is_complete = page.is_complete;
        let run = fixture
            .catalog
            .stage_metadata_inventory_page("spool-10k", &page, 5_000 + i64::from(output_pages))
            .expect("stage durable output page");
        if is_complete {
            assert!(run.enumeration_complete);
            break;
        }
        drop(source);
        source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    }

    assert_eq!(staged, 10_000);
    assert_eq!(output_pages, 3);
    assert_eq!(source_entry_read_count(&fixture.root_path), 10_000);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
}

#[test]
fn durable_spool_cancels_before_4095_and_replays_only_one_incomplete_directory_after_loss() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..1_000_u32 {
        fs::write(
            fixture.source.path().join(format!("entry-{index:04}.jpg")),
            b"metadata",
        )
        .expect("flat source entry");
    }
    let leased = enqueue_authorized_recovery_control(&mut fixture, "spool-crash", 6_000);
    let request = MetadataInventoryStartRequest {
        run_id: "spool-crash".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 6_000,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin crash-safe inventory");
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    assert_eq!(
        source
            .prepare_next_page(4_095, &cancellation)
            .expect("first raw batch"),
        MetadataInventorySourcePreparation::Yielded
    );
    cancellation.store(true, std::sync::atomic::Ordering::Release);
    let error = source
        .prepare_next_page(4_095, &cancellation)
        .expect_err("cancellation must stop before a full source page");
    assert_eq!(error.code, "metadata_inventory_cancelled");
    assert_eq!(source_entry_read_count(&fixture.root_path), 128);
    drop(source);

    let cancellation = AtomicBool::new(false);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("spool-crash")
        .expect("load durable run")
        .expect("active durable run");
    let mut resumed = open_inventory_source(&fixture, &request.scope, &run, &leased);
    while resumed
        .prepare_next_page(4_095, &cancellation)
        .expect("resume raw spool")
        == MetadataInventorySourcePreparation::Yielded
    {}

    assert!(source_entry_read_count(&fixture.root_path) <= 2_000);
    assert_eq!(source_spool_open_count(&fixture.root_path), 2);
    assert!(source_peak_staged_window(&fixture.root_path) <= 128);
}

#[cfg(windows)]
#[test]
fn first_p2_source_open_rejects_replaced_root_before_any_durable_namespace_read() {
    let mut fixture = InventoryFixture::new(&["trusted.png"]);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load proven root")
        .expect("proven root");
    let trusted_scan_id = root.active_scan_id.clone();
    let trusted_revision = root.catalog_revision;
    let leased =
        enqueue_authorized_recovery_control(&mut fixture, "first-p2-root-replacement", 6_500);
    let authority_before = fixture
        .catalog
        .load_metadata_inventory_recovery_authority(leased.change.id)
        .expect("load recovery authority before replacement")
        .expect("recovery authority before replacement");
    let original = fixture.source.path().to_path_buf();
    let retained = original.with_extension("first-p2-authorized-root");
    fs::rename(&original, &retained).expect("retain proven source root");
    fs::create_dir(&original).expect("create replacement source root");
    fs::write(original.join("replacement.jpg"), b"replacement namespace")
        .expect("replacement source entry");
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let rejected = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &leased,
        6_500,
        4_096,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("replacement is durably retried");

    fs::remove_dir_all(&original).expect("remove replacement source root");
    fs::rename(&retained, &original).expect("restore proven source root");
    assert_eq!(rejected.incremental.completed_count, 0);
    assert_eq!(rejected.incremental.applied_mutation_count, 0);
    assert_eq!(rejected.incremental.retried_count, 1);
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    assert_eq!(source_spool_open_count(&fixture.root_path), 0);
    assert!(fixture.location("trusted.png").is_some());
    assert!(fixture.location("replacement.jpg").is_none());
    let root_after = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root after replacement rejection")
        .expect("retained proven root");
    assert_eq!(root_after.active_scan_id, trusted_scan_id);
    assert_eq!(root_after.catalog_revision, trusted_revision);
    assert_eq!(
        root_after.publication_root_identity,
        root.publication_root_identity
    );
    let authority_after = fixture
        .catalog
        .load_metadata_inventory_recovery_authority(leased.change.id)
        .expect("load recovery authority after replacement")
        .expect("retained recovery authority");
    assert_eq!(authority_after, authority_before);
    let evidence = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open first-source rejection evidence");
    let namespace_writes: (i64, i64, i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_runs WHERE id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_entries WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners WHERE run_id = ?1)",
            [&authority_before.run_id],
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
        .expect("load first-source rejection evidence");
    assert_eq!(namespace_writes, (0, 0, 0, 0, 0));
}

#[cfg(windows)]
#[test]
fn first_p2_spool_commit_holds_the_publication_guard_until_commit_returns() {
    let mut fixture = InventoryFixture::new(&["trusted.png"]);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load commit-window root")
        .expect("commit-window root");
    let leased =
        enqueue_authorized_recovery_control(&mut fixture, "first-p2-spool-commit-guard", 6_600);
    let original = fixture.source.path().to_path_buf();
    let moved = original.with_extension("spool-commit-rename");
    let rename_error = Rc::new(RefCell::new(None));
    set_before_metadata_inventory_spool_commit_hook({
        let original = original.clone();
        let moved = moved.clone();
        let rename_error = Rc::clone(&rename_error);
        move || {
            let error = fs::rename(&original, &moved)
                .expect_err("the source namespace guard must deny rename before spool commit");
            *rename_error.borrow_mut() = error.raw_os_error();
        }
    });

    let page = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &leased,
        MetadataInventoryRecoveryExecution::new(
            6_600,
            4_096,
            queue_policy(),
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        None,
    )
    .expect("initialize guarded durable spool");

    assert!(page.retained_source.is_some());
    assert!(matches!(*rename_error.borrow(), Some(5 | 32)));
    let spool_identity = fixture
        .catalog
        .load_metadata_inventory_root_identity("first-p2-spool-commit-guard")
        .expect("load committed spool identity")
        .expect("committed spool identity");
    assert_eq!(
        spool_identity,
        root.publication_root_identity.expect("proof")
    );
    drop(page);
    fs::rename(&original, &moved).expect("rename succeeds after source guard release");
    fs::rename(&moved, &original).expect("restore disposable source root");
}

#[cfg(windows)]
#[test]
fn first_p2_baseline_without_publication_proof_establishes_a_guarded_spool_identity() {
    let mut fixture = InventoryFixture::new(&["baseline.png"]);
    remove_inventory_publication_namespace_proof(&fixture);
    fixture
        .catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 6_700,
        })
        .expect("save first-baseline capability");
    let baseline = fixture
        .catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "first-p2-unproven-baseline".to_owned(),
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                authority_reason:
                    crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "first-p2-unproven-baseline-volume".to_owned(),
                    volume_serial: 1,
                },
                root_file_reference: JournalFileReference::V3([1; 16]),
                journal_id: JournalIdentifier::new(1).expect("first-baseline journal ID"),
                opening_next_usn: JournalUsn::new(1).expect("first-baseline opening USN"),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: 6_700,
            },
            queue_policy(),
        )
        .expect("admit first-authority baseline");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load unproven baseline root")
        .expect("unproven baseline root");
    assert!(root.publication_root_identity.is_none());
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            6_700,
            queue_policy(),
        )
        .expect("lease first-authority baseline")
        .expect("first-authority baseline");
    assert_eq!(leased.change.id, baseline.change_id);

    let page = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &leased,
        MetadataInventoryRecoveryExecution::new(
            6_700,
            4_096,
            queue_policy(),
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        None,
    )
    .expect("open a guarded first-authority baseline");

    if page.report.incremental.retried_count != 0 {
        let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
            .expect("open first-baseline retry evidence");
        let failure: (Option<String>, Option<String>) = connection
            .query_row(
                "SELECT last_failure_code, last_failure_message
                 FROM library_change_queue WHERE id = ?1",
                [i64::try_from(leased.change.id.value()).expect("change ID")],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load first-baseline retry evidence");
        panic!("guarded first baseline retried unexpectedly: {failure:?}");
    }
    assert!(page.retained_source.is_some());
    let spool_identity = fixture
        .catalog
        .load_metadata_inventory_root_identity("first-p2-unproven-baseline")
        .expect("load first-baseline spool identity")
        .expect("first-baseline spool identity");
    let guarded_identity = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &fixture.root_path,
        None,
    )
    .expect("reopen guarded baseline source")
    .metadata_inventory_root_identity()
    .expect("load guarded baseline identity")
    .expect("guarded baseline identity");
    assert_eq!(spool_identity, guarded_identity);
    assert!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("reload baseline root")
            .expect("baseline root")
            .publication_root_identity
            .is_none(),
        "the spool identity is not publication proof before recovery completion"
    );
}

#[cfg(windows)]
#[test]
fn durable_spool_rebinds_the_configured_root_before_resuming_an_active_directory() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..256_u32 {
        fs::write(
            fixture.source.path().join(format!("entry-{index:03}.jpg")),
            b"metadata",
        )
        .expect("identity fixture entry");
    }
    let leased = enqueue_authorized_recovery_control(&mut fixture, "spool-identity", 7_000);
    let request = MetadataInventoryStartRequest {
        run_id: "spool-identity".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 7_000,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin identity inventory");
    let opening_identity = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load initial root")
        .expect("initial root")
        .publication_root_identity
        .expect("initial root identity");
    let execution = fixture
        .catalog
        .initialize_metadata_inventory_spool(
            &run,
            &leased,
            &opening_identity,
            None,
            Some(""),
            7_000,
        )
        .expect("capture initial spool execution");
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    assert_eq!(
        source
            .prepare_next_page(4_095, &cancellation)
            .expect("first identity batch"),
        MetadataInventorySourcePreparation::Yielded
    );
    assert_eq!(
        source
            .prepare_next_page(4_095, &cancellation)
            .expect("second identity batch"),
        MetadataInventorySourcePreparation::Yielded
    );
    let publication_identity = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load publication root")
        .expect("publication root")
        .publication_root_identity
        .expect("publication root identity");
    let source_identity = fixture
        .catalog
        .load_metadata_inventory_root_identity(&request.run_id)
        .expect("load spool root identity")
        .expect("spool root identity");
    let reads_before_replacement = source_entry_read_count(&fixture.root_path);
    drop(source);
    let original = fixture.source.path().to_path_buf();
    let replaced = original.with_extension("enumerated-before-replacement");
    fs::rename(&original, &replaced).expect("replace enumerated directory identity");
    fs::create_dir(&original).expect("replacement source directory");
    fs::write(
        original.join("replacement.jpg"),
        b"outside the authorized handle",
    )
    .expect("replacement source entry");

    let error = match fixture.catalog.open_metadata_inventory_source(
        &fixture.root_path,
        &request.scope,
        &run,
        &leased,
        Some(&publication_identity),
        &source_identity,
    ) {
        Ok(_) => panic!("a replacement configured root must invalidate source reconstruction"),
        Err(error) => error,
    };
    assert_eq!(error.code, "metadata_inventory_root_identity_changed");
    assert_eq!(
        source_entry_read_count(&fixture.root_path),
        reads_before_replacement,
        "replacement rejection must precede resumed source enumeration"
    );
    let paging_error = fixture
        .catalog
        .load_metadata_inventory_spool_page(&execution, 4_095)
        .expect_err("provisional rows must not become an output page");
    assert_eq!(paging_error.code, "metadata_inventory_spool_not_ready");
    fs::remove_dir_all(&original).expect("remove replacement source directory");
    fs::rename(&replaced, &original).expect("restore disposable source directory");
    let resumed = open_inventory_source(&fixture, &request.scope, &run, &leased);
    drop(resumed);
}

#[cfg(windows)]
#[test]
fn durable_spool_rejects_a_pending_frontier_replaced_by_an_external_junction() {
    use std::process::Command;

    let mut fixture = InventoryFixture::new(&["album/inside.png"]);
    let outside = tempdir().expect("outside directory");
    fs::write(outside.path().join("outside.png"), b"outside-only").expect("outside fixture");
    let active_scan_before = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root before adversarial inventory")
        .expect("registered root")
        .active_scan_id;
    let leased = enqueue_authorized_recovery_control(&mut fixture, "spool-junction", 7_500);
    let request = MetadataInventoryStartRequest {
        run_id: "spool-junction".to_owned(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 7_500,
    };
    let run = fixture
        .catalog
        .begin_next_metadata_inventory(&request)
        .expect("begin junction inventory");
    let cancellation = AtomicBool::new(false);
    let mut source = open_inventory_source(&fixture, &request.scope, &run, &leased);
    assert_eq!(
        source
            .prepare_next_page(4_095, &cancellation)
            .expect("persist root frontier"),
        MetadataInventorySourcePreparation::Yielded
    );

    let album = fixture.source.path().join("album");
    let retained_album = fixture.source.path().join("album-authorized");
    fs::rename(&album, &retained_album).expect("retain authorized directory");
    let junction = Command::new("cmd.exe")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&album)
        .arg(outside.path())
        .status()
        .expect("junction command");
    assert!(junction.success());

    let error = source
        .prepare_next_page(4_095, &cancellation)
        .expect_err("the external frontier must fail closed");
    fs::remove_dir(&album).expect("remove junction");
    assert_eq!(error.code, "source_path_outside_root");

    let run = fixture
        .catalog
        .load_metadata_inventory_run("spool-junction")
        .expect("load failed-closed run")
        .expect("retained failed-closed run");
    assert_eq!(run.status, MetadataInventoryRunStatus::Running);
    assert!(!run.enumeration_complete);
    assert!(!run.absence_authority);
    let active_scan_after = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root after adversarial inventory")
        .expect("registered root")
        .active_scan_id;
    assert_eq!(active_scan_after, active_scan_before);
    assert!(fixture.location("album/inside.png").is_some());
    assert!(fixture.location("outside.png").is_none());

    let connection = rusqlite::Connection::open(fixture._storage.path().join("catalog.sqlite3"))
        .expect("inspection connection");
    let outside_spool_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_spool_entries
             WHERE run_id = 'spool-junction' AND relative_path LIKE '%outside.png%'",
            [],
            |row| row.get(0),
        )
        .expect("outside spool count");
    let staged_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_entries
             WHERE run_id = 'spool-junction'",
            [],
            |row| row.get(0),
        )
        .expect("staged inventory count");
    let candidate_owner_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
             WHERE run_id = 'spool-junction'",
            [],
            |row| row.get(0),
        )
        .expect("candidate owner count");
    assert_eq!(outside_spool_count, 0);
    assert_eq!(staged_count, 0);
    assert_eq!(candidate_owner_count, 0);
}

#[cfg(windows)]
#[test]
fn candidate_drain_rejects_an_ancestor_junction_after_a_proven_spool_snapshot() {
    use std::process::Command;

    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::create_dir(fixture.source.path().join("album")).expect("candidate album");
    write_png(
        &fixture.source.path().join("album/new.png"),
        2,
        2,
        [10, 20, 30],
    );
    let outside = tempdir().expect("outside directory");
    write_png(&outside.path().join("new.png"), 7, 5, [200, 10, 10]);
    let active_scan_before = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root before cross-stage replacement")
        .expect("registered root")
        .active_scan_id;
    assert!(fixture.location("retained.png").is_some());
    fixture
        .catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 7_600,
        })
        .expect("cross-stage baseline capability");
    let baseline = fixture
        .catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "spool-candidate-junction".to_owned(),
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                authority_reason:
                    crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: PersistentJournalVolumeIdentity {
                    volume_guid: "cross-stage-volume".to_owned(),
                    volume_serial: 77,
                },
                root_file_reference: JournalFileReference::V3([8; 16]),
                journal_id: JournalIdentifier::new(11).expect("journal ID"),
                opening_next_usn: JournalUsn::new(12).expect("opening USN"),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: 7_600,
            },
            queue_policy(),
        )
        .expect("admit cross-stage baseline");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load cross-stage root")
        .expect("registered root");
    let mut waiting = None;
    for step in 0..12_i64 {
        let now = 7_600_i64.saturating_add(step);
        let leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now,
                queue_policy(),
            )
            .expect("lease cross-stage baseline")
            .expect("cross-stage baseline work");
        assert_eq!(leased.change.id, baseline.change_id);
        let current = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &leased,
            now,
            4_096,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("advance cross-stage inventory");
        if current.inventory.awaiting_closing_boundary {
            waiting = Some(current);
            break;
        }
    }
    let waiting = waiting.expect("inventory reaches the closing boundary");
    assert!(!waiting.inventory.is_complete);
    assert!(waiting.inventory.candidate_count >= 1);
    assert!(fixture.location("retained.png").is_some());

    let album = fixture.source.path().join("album");
    let retained_album = fixture.source.path().join("album-authorized");
    fs::rename(&album, &retained_album).expect("retain authorized album");
    let junction = Command::new("cmd.exe")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(&album)
        .arg(outside.path())
        .status()
        .expect("junction command");
    assert!(junction.success());

    let drained = process_ready_metadata_inventory_recovery_candidates_cancellable(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        7_700,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("candidate drain fails closed");
    fs::remove_dir(&album).expect("remove replacement junction");

    assert_eq!(drained.completed_count, 0);
    assert_eq!(drained.applied_mutation_count, 0);
    assert!(drained.retried_count >= 1);
    assert!(fixture.location("retained.png").is_some());
    assert!(fixture.location("album/new.png").is_none());
    assert!(fixture.location("new.png").is_none());
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load root after cross-stage replacement")
            .expect("registered root")
            .active_scan_id,
        active_scan_before
    );
    let run = fixture
        .catalog
        .load_metadata_inventory_run("spool-candidate-junction")
        .expect("load cross-stage run")
        .expect("retained cross-stage run");
    assert_eq!(run.status, MetadataInventoryRunStatus::Comparing);
    assert!(!run.absence_authority);

    let connection = rusqlite::Connection::open(fixture._storage.path().join("catalog.sqlite3"))
        .expect("cross-stage inspection connection");
    let evidence: (i64, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries
                WHERE run_id = 'spool-candidate-junction'
                  AND relative_path = 'outside/new.png'),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners
                WHERE run_id = 'spool-candidate-junction'),
               (SELECT COUNT(*) FROM library_change_queue
                WHERE root_id = ?1 AND relative_path = 'new.png'),
               (SELECT COUNT(*) FROM library_persistent_journal_root_state
                WHERE root_id = ?1 AND continuity_state = 'current'),
               (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                WHERE root_id = ?1 AND continuity_state = 'current')",
            [&fixture.root_id],
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
        .expect("cross-stage fail-closed evidence");
    assert_eq!(evidence.0, 0);
    assert!(evidence.1 >= 1);
    assert_eq!(evidence.2, 0);
    assert_eq!(evidence.3, 0);
    assert_eq!(evidence.4, 0);
}

#[test]
fn cancelled_inventory_keeps_absence_unproven_and_catalog_unchanged() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::remove_file(fixture.source.path().join("retained.png")).expect("delete fixture");
    let cancellation = AtomicBool::new(true);

    let report = fixture.run_inventory(1, &cancellation);

    assert!(report.is_cancelled);
    assert!(!report.is_complete);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("inventory-1")
        .expect("load inventory")
        .expect("inventory run");
    assert_eq!(run.status, MetadataInventoryRunStatus::Cancelled);
    assert!(!run.enumeration_complete);
    assert!(!run.absence_authority);
    assert!(fixture.location("retained.png").is_some());
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count + metrics.retry_wait_count, 0);
}

#[test]
fn placeholder_evidence_is_staged_and_enqueued_without_media_inspection() {
    let mut fixture = InventoryFixture::new(&[]);
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-only.bin".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(123),
                modified_unix_ms: 1_500,
                file_identity: None,
                source_revision: None,
                placeholder_state: MetadataInventoryPlaceholderState::Offline,
                is_reparse_point: false,
            }],
            cursor: Some("cloud-only.bin".to_owned()),
            is_complete: true,
            frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only placeholder inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 1);
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count, 1);
}

#[test]
fn unchanged_reparse_cloud_placeholder_preserves_location_without_retry() {
    let mut fixture = InventoryFixture::new(&["cloud-only.png"]);
    let prior = fixture
        .location("cloud-only.png")
        .expect("published placeholder location");
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-only.png".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(prior.file_size),
                modified_unix_ms: prior.modified_unix_ms,
                file_identity: None,
                source_revision: None,
                placeholder_state: MetadataInventoryPlaceholderState::Offline,
                is_reparse_point: true,
            }],
            cursor: Some("cloud-only.png".to_owned()),
            is_complete: true,
            frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only reparse placeholder inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 0);
    assert_eq!(report.absence_candidate_count, 0);
    assert!(fixture.location("cloud-only.png").is_some());
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            queue_policy(),
        )
        .expect("queue metrics");
    assert_eq!(metrics.pending_count + metrics.retry_wait_count, 0);
}

#[test]
fn hydrated_cloud_files_reparse_matches_the_existing_location() {
    let mut fixture = InventoryFixture::new(&["cloud-local.png"]);
    let prior = fixture
        .location("cloud-local.png")
        .expect("published local Cloud Files location");
    let mut source = FixedInventorySource {
        page: Some(MetadataInventoryPage {
            page_index: 1,
            entries: vec![MetadataInventoryEntry {
                relative_path: "cloud-local.png".to_owned(),
                kind: MetadataInventoryEntryKind::File,
                file_size: Some(prior.file_size),
                modified_unix_ms: prior.modified_unix_ms,
                file_identity: prior.file_identity.clone(),
                source_revision: prior.source_revision.clone(),
                placeholder_state: MetadataInventoryPlaceholderState::Available,
                is_reparse_point: true,
            }],
            cursor: Some("cloud-local.png".to_owned()),
            is_complete: true,
            frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
        }),
    };
    let request = fixture.request();

    let report = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("metadata-only hydrated Cloud Files inventory");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 0);
    assert_eq!(report.absence_candidate_count, 0);
    assert!(fixture.location("cloud-local.png").is_some());
}

#[test]
fn source_failure_terminates_the_durable_inventory_run() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    let mut source = FailingInventorySource;

    let error = run_metadata_inventory(
        &mut fixture.catalog,
        &mut source,
        &request,
        2_000,
        4,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect_err("inventory source failure");
    let run = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load failed inventory")
        .expect("failed inventory run");

    assert_eq!(error.code, "metadata_inventory_fixture_failure");
    assert_eq!(run.status, MetadataInventoryRunStatus::Failed);
    assert_eq!(
        run.last_issue_code.as_deref(),
        Some("metadata_inventory_fixture_failure")
    );
    assert!(!run.absence_authority);
}

#[test]
fn terminalization_retries_one_transient_catalog_contention_failure() {
    let mut attempts = 0;

    retry_terminalization(|| {
        attempts += 1;
        if attempts == 1 {
            Err(ScanError::new(
                "catalog_database_busy",
                "fixture writer still owns the catalog",
            ))
        } else {
            Ok(())
        }
    })
    .expect("second terminalization attempt");

    assert_eq!(attempts, 2);
}

#[test]
fn newer_epoch_supersedes_an_orphaned_active_run_and_cleanup_stays_bounded() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&first)
        .expect("begin first inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &first.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("orphaned.txt")],
                cursor: Some("orphaned.txt".to_owned()),
                is_complete: false,
                frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                    0,
                    "",
                    None,
                    Some("orphaned.txt".to_owned()),
                    1,
                )],
            },
            2_100,
        )
        .expect("stage orphaned entry");
    let second = MetadataInventoryRunRequest {
        run_id: "inventory-2".to_owned(),
        epoch: 2,
        started_unix_ms: 3_000,
        ..first.clone()
    };

    fixture
        .catalog
        .begin_metadata_inventory(&second)
        .expect("newer epoch");

    let superseded = fixture
        .catalog
        .load_metadata_inventory_run(&first.run_id)
        .expect("load first run")
        .expect("superseded run");
    assert_eq!(superseded.status, MetadataInventoryRunStatus::Superseded);
    let cleanup = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(4_000, 1, 1)
        .expect("bounded cleanup");
    assert_eq!(cleanup.removed_entry_count, 1);
    assert_eq!(cleanup.removed_run_count, 1);
    assert!(!cleanup.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&first.run_id)
            .expect("load cleaned run")
            .is_none()
    );
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&second.run_id)
            .expect("load active run")
            .expect("active run")
            .status,
        MetadataInventoryRunStatus::Running
    );
}

#[test]
fn newer_epoch_revokes_superseded_absence_authority_and_catalog_reopens() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&first)
        .expect("begin first inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &first.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("retained-authority.txt")],
                cursor: Some("retained-authority.txt".to_owned()),
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
            },
            2_100,
        )
        .expect("complete first enumeration");
    let authorized = fixture
        .catalog
        .authorize_metadata_inventory_absence(&first.run_id, 2_200)
        .expect("authorize first inventory absence");
    assert_eq!(authorized.status, MetadataInventoryRunStatus::Comparing);
    assert!(authorized.absence_authority);

    let second = MetadataInventoryRunRequest {
        run_id: "inventory-2".to_owned(),
        epoch: 2,
        started_unix_ms: 3_000,
        ..first.clone()
    };
    let _second_authority =
        enqueue_authorized_recovery_control(&mut fixture, &second.run_id, 3_000);
    fixture
        .catalog
        .begin_metadata_inventory(&second)
        .expect("begin newer inventory epoch");

    let superseded = fixture
        .catalog
        .load_metadata_inventory_run(&first.run_id)
        .expect("load first run")
        .expect("superseded run");
    assert_eq!(superseded.status, MetadataInventoryRunStatus::Superseded);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let reopened = SqliteCatalog::open(catalog_path);
    let reopen_error = reopened
        .as_ref()
        .err()
        .map(|error| format!("{}: {}", error.code, error.message));
    assert!(
        !superseded.absence_authority && reopened.is_ok(),
        "newer epoch retained absence_authority={} and reopen_error={reopen_error:?}",
        superseded.absence_authority,
    );
}

#[test]
fn terminalization_revokes_absence_authority_and_catalog_reopens() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &request.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![metadata_entry("retained-terminal.txt")],
                cursor: Some("retained-terminal.txt".to_owned()),
                is_complete: true,
                frontier: vec![MetadataInventoryFrontierEntry::completed("", None)],
            },
            2_100,
        )
        .expect("complete inventory enumeration");
    fixture
        .catalog
        .authorize_metadata_inventory_absence(&request.run_id, 2_200)
        .expect("authorize inventory absence");
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_300,
        )
        .expect("terminate authoritative inventory");

    let terminal = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load terminal inventory")
        .expect("terminal inventory");
    assert_eq!(terminal.status, MetadataInventoryRunStatus::Failed);
    assert_eq!(terminal.last_issue_code.as_deref(), Some("fixture_failure"));
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let reopened = SqliteCatalog::open(catalog_path);
    let reopen_error = reopened
        .as_ref()
        .err()
        .map(|error| format!("{}: {}", error.code, error.message));
    assert!(
        !terminal.absence_authority && reopened.is_ok(),
        "terminal inventory retained absence_authority={} and reopen_error={reopen_error:?}",
        terminal.absence_authority,
    );
}

#[test]
fn next_epoch_is_allocated_atomically_and_does_not_depend_on_wall_clock_order() {
    let mut fixture = InventoryFixture::new(&[]);
    let first = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "inventory-next-1".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 5_000,
        })
        .expect("begin first allocated epoch");
    let second = fixture
        .catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "inventory-next-2".to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            scope: MetadataInventoryScope::Subtree {
                relative_path: "album".to_owned(),
            },
            started_unix_ms: 4_000,
        })
        .expect("begin second allocated epoch");

    assert_eq!(first.request.epoch, 1);
    assert_eq!(second.request.epoch, 2);
    assert_eq!(second.request.started_unix_ms, 4_000);
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&first.request.run_id)
            .expect("load first allocated epoch")
            .expect("first allocated epoch")
            .status,
        MetadataInventoryRunStatus::Superseded
    );
}

#[test]
fn proven_consistency_gap_uses_inventory_and_completes_its_authoritative_lease() {
    let mut fixture = InventoryFixture::new(&["removed.png"]);
    fs::remove_file(fixture.source.path().join("removed.png")).expect("remove fixture");
    write_png(&fixture.source.path().join("added.png"), 3, 2, [40, 50, 60]);
    let policy = LibraryChangeQueuePolicy {
        max_unresolved_changes: 2,
        max_lease_batch: 1,
        ..queue_policy()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue consistency-gap authority");
    let mut report = None;
    let mut completed_candidates = 0_u32;
    let progress = RefCell::new(Vec::new());
    let page_yields = RefCell::new(Vec::new());
    let mut retained_source = None;
    for _ in 0..32 {
        let leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                2_000,
                policy,
            )
            .expect("lease consistency-gap authority")
            .expect("consistency-gap authority");
        assert!(leased_change_requires_metadata_inventory(&leased));
        let run_id = metadata_inventory_run_id(&leased);
        authorize_leased_containment_recovery(&mut fixture, &leased, &run_id, 2_000);
        let root = fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load root")
            .expect("root");
        let current = process_leased_metadata_inventory_change_with_retained_source(
            &mut fixture.catalog,
            &root,
            &leased,
            MetadataInventoryRecoveryExecution::new(
                2_000,
                1,
                policy,
                MetadataInventoryWorkerControl::with_progress_and_page_yield(
                    &AtomicBool::new(false),
                    &|phase| progress.borrow_mut().push(phase),
                    &|phase| page_yields.borrow_mut().push(phase),
                ),
            ),
            retained_source,
        )
        .expect("process consistency-gap inventory page");
        retained_source = current.retained_source;
        let current = current.report;
        completed_candidates = completed_candidates.saturating_add(
            process_ready_library_changes(
                &mut fixture.catalog,
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                2_000,
                policy,
            )
            .expect("publish inventory candidates")
            .completed_count,
        );
        let is_complete = current.inventory.is_complete;
        report = Some(current);
        if is_complete {
            break;
        }
    }
    let report = report.expect("inventory recovery report");
    assert!(
        progress
            .borrow()
            .contains(&MetadataInventoryProgressPhase::Enumeration)
    );
    assert!(
        progress
            .borrow()
            .contains(&MetadataInventoryProgressPhase::Comparison)
    );
    assert!(
        progress
            .borrow()
            .contains(&MetadataInventoryProgressPhase::QueuePublication)
    );
    assert!(
        page_yields
            .borrow()
            .contains(&MetadataInventoryProgressPhase::Enumeration)
    );

    assert!(report.inventory.is_complete);
    assert_eq!(report.inventory.staged_entry_count, 1);
    assert_eq!(report.inventory.candidate_count, 2);
    assert_eq!(report.incremental.leased_count, 1);
    assert_eq!(completed_candidates, 2);
    assert!(fixture.location("removed.png").is_none());
    assert!(fixture.location("added.png").is_some());
}

#[cfg(windows)]
#[test]
fn ordinary_startup_catch_up_never_authorizes_metadata_inventory() {
    let mut fixture = InventoryFixture::new(&[]);
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::StartupCatchUp,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue ordinary startup catch-up");
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease ordinary startup catch-up")
        .expect("startup catch-up work");

    assert!(!leased_change_requires_metadata_inventory(&leased));
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_recovery_authority(leased.change.id)
            .expect("load recovery authority")
            .is_none()
    );
    remove_inventory_publication_namespace_proof(&fixture);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load unproven root")
        .expect("unproven root");
    assert!(root.publication_root_identity.is_none());
    let metrics_before = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("load unauthorized queue metrics");
    crate::adapters::reset_configured_root_open_instrumentation(&fixture.root_path);
    reset_source_enumeration_instrumentation(&fixture.root_path);

    let error = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &leased,
        2_000,
        4_096,
        policy,
        &AtomicBool::new(false),
    )
    .expect_err("proofless ordinary catch-up must not enter inventory");

    assert_eq!(error.code, "metadata_inventory_recovery_not_authorized");
    assert_eq!(
        crate::adapters::configured_root_open_count(&fixture.root_path, true),
        0
    );
    assert_eq!(
        crate::adapters::configured_root_open_count(&fixture.root_path, false),
        0
    );
    assert_eq!(source_entry_read_count(&fixture.root_path), 0);
    assert_eq!(source_directory_open_count(&fixture.root_path), 0);
    assert_eq!(source_spool_open_count(&fixture.root_path), 0);
    assert_eq!(
        fixture
            .catalog
            .load_library_change_root_queue_metrics(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                2_000,
                policy,
            )
            .expect("reload unauthorized queue metrics"),
        metrics_before
    );
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("reload unproven root")
            .expect("unproven root remains")
            .catalog_revision,
        root.catalog_revision
    );
    let evidence = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open unauthorized inventory evidence");
    let writes: (i64, i64, i64, i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_runs),
               (SELECT COUNT(*) FROM library_metadata_inventory_entries),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier),
               (SELECT COUNT(*) FROM library_metadata_inventory_spools),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries),
               (SELECT COUNT(*) FROM library_metadata_inventory_candidate_owners)",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("load unauthorized inventory writes");
    assert_eq!(writes, (0, 0, 0, 0, 0, 0));
}

#[test]
fn retained_recovery_frontier_advances_without_replaying_prior_pages() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..4 {
        write_png(
            &fixture.source.path().join(format!("image-{index}.png")),
            2,
            2,
            [index, index.saturating_add(1), index.saturating_add(2)],
        );
    }
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue retained-frontier recovery");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let first_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease first page")
        .expect("first recovery page");
    let run_id = metadata_inventory_run_id(&first_lease);
    authorize_leased_containment_recovery(&mut fixture, &first_lease, &run_id, 2_000);
    let first = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &first_lease,
        MetadataInventoryRecoveryExecution::new(
            2_000,
            1,
            policy,
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        None,
    )
    .expect("process first retained page");
    assert!(!first.report.inventory.is_complete);
    let mut retained = first.retained_source;
    let mut next_unix_ms = 2_001_i64;
    let durable = loop {
        let durable = fixture
            .catalog
            .load_metadata_inventory_run(&run_id)
            .expect("load durable first page")
            .expect("durable first page");
        if durable.staged_entry_count == 1 {
            break durable;
        }
        let lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                next_unix_ms,
                policy,
            )
            .expect("lease retained source preparation")
            .expect("retained source preparation");
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut fixture.catalog,
            &root,
            &lease,
            MetadataInventoryRecoveryExecution::new(
                next_unix_ms,
                1,
                policy,
                MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
            ),
            retained,
        )
        .expect("prepare and stage first retained page");
        retained = page.retained_source;
        next_unix_ms = next_unix_ms.saturating_add(1);
        assert!(next_unix_ms < 2_020, "first retained page did not converge");
    };
    assert_eq!(durable.next_page_index, 2);
    assert_eq!(durable.staged_entry_count, 1);
    let first_path = durable.enumeration_cursor.expect("first durable cursor");
    fs::remove_file(fixture.source.path().join(first_path)).expect("remove already consumed entry");

    let second_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            next_unix_ms,
            policy,
        )
        .expect("lease second page")
        .expect("second recovery page");
    let second = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &second_lease,
        MetadataInventoryRecoveryExecution::new(
            next_unix_ms,
            1,
            policy,
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        retained,
    )
    .expect("process second retained page without replay");
    assert!(second.retained_source.is_some());
    let durable = fixture
        .catalog
        .load_metadata_inventory_run(&run_id)
        .expect("load durable second page")
        .expect("durable second page");
    assert_eq!(durable.next_page_index, 3);
    assert_eq!(durable.staged_entry_count, 2);
}

#[test]
fn crashed_recovery_resumes_a_completed_durable_spool_without_source_replay() {
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..4 {
        write_png(
            &fixture.source.path().join(format!("image-{index}.png")),
            2,
            2,
            [index, index.saturating_add(1), index.saturating_add(2)],
        );
    }
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue crash-resumable recovery");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let first_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease first page")
        .expect("first recovery page");
    let run_id = metadata_inventory_run_id(&first_lease);
    authorize_leased_containment_recovery(&mut fixture, &first_lease, &run_id, 2_000);
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let first = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &first_lease,
        MetadataInventoryRecoveryExecution::new(
            2_000,
            1,
            policy,
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        None,
    )
    .expect("process first page");
    let mut retained = first.retained_source;
    let mut next_unix_ms = 2_001_i64;
    let durable = loop {
        let durable = fixture
            .catalog
            .load_metadata_inventory_run(&run_id)
            .expect("load durable first page")
            .expect("durable first page");
        if durable.staged_entry_count == 1 {
            break durable;
        }
        let lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                next_unix_ms,
                policy,
            )
            .expect("lease durable spool preparation")
            .expect("durable spool preparation");
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut fixture.catalog,
            &root,
            &lease,
            MetadataInventoryRecoveryExecution::new(
                next_unix_ms,
                1,
                policy,
                MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
            ),
            retained,
        )
        .expect("prepare completed spool and stage first page");
        retained = page.retained_source;
        next_unix_ms = next_unix_ms.saturating_add(1);
        assert!(next_unix_ms < 2_020, "completed spool did not converge");
    };
    assert_eq!(durable.next_page_index, 2);
    assert_eq!(durable.staged_entry_count, 1);
    assert_eq!(source_entry_read_count(&fixture.root_path), 4);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
    drop(retained);

    let second_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            next_unix_ms,
            policy,
        )
        .expect("lease page after crash")
        .expect("crash-resumed recovery page");
    let second = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &second_lease,
        MetadataInventoryRecoveryExecution::new(
            next_unix_ms,
            1,
            policy,
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        None,
    )
    .expect("resume durable frontier without replay");
    assert!(second.retained_source.is_some());
    let durable = fixture
        .catalog
        .load_metadata_inventory_run(&run_id)
        .expect("load crash-resumed run")
        .expect("crash-resumed run");
    assert_eq!(durable.next_page_index, 3);
    assert_eq!(durable.staged_entry_count, 2);
    assert_eq!(source_entry_read_count(&fixture.root_path), 4);
    assert_eq!(source_spool_open_count(&fixture.root_path), 1);
}

#[test]
fn nested_durable_spool_orders_directories_and_never_replays_a_completed_parent() {
    let mut fixture = InventoryFixture::new(&[]);
    write_png(&fixture.source.path().join("root.png"), 2, 2, [1, 2, 3]);
    write_png(
        &fixture.source.path().join("album-a").join("a.png"),
        2,
        2,
        [4, 5, 6],
    );
    write_png(
        &fixture
            .source
            .path()
            .join("album-a")
            .join("nested")
            .join("deep.png"),
        2,
        2,
        [7, 8, 9],
    );
    write_png(
        &fixture.source.path().join("album-b").join("b.png"),
        2,
        2,
        [10, 11, 12],
    );
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue nested recovery");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load nested root")
        .expect("nested root");
    let mut lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease nested recovery")
        .expect("nested recovery");
    let run_id = metadata_inventory_run_id(&lease);
    authorize_leased_containment_recovery(&mut fixture, &lease, &run_id, 2_000);
    reset_source_enumeration_instrumentation(&fixture.root_path);
    let mut retained = None;

    for directory_index in 0..4_i64 {
        let observed_unix_ms = 2_000_i64.saturating_add(directory_index);
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut fixture.catalog,
            &root,
            &lease,
            MetadataInventoryRecoveryExecution::new(
                observed_unix_ms,
                4_095,
                policy,
                MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
            ),
            retained,
        )
        .expect("enumerate one nested source directory");
        assert!(!page.report.inventory.is_complete);
        retained = page.retained_source;
        let run = fixture
            .catalog
            .load_metadata_inventory_run(&run_id)
            .expect("load nested run")
            .expect("nested run");
        assert!(
            !run.absence_authority,
            "absence must remain untrusted while any source directory is unfinished"
        );
        if directory_index == 0 {
            assert_eq!(source_entry_read_count(&fixture.root_path), 3);
            assert_eq!(source_spool_open_count(&fixture.root_path), 1);
            drop(retained.take());
            let catalog_path = fixture.catalog.catalog_path().to_path_buf();
            fixture.catalog = SqliteCatalog::open(catalog_path)
                .expect("reopen current v27 catalog with completed parent spool");
        } else if directory_index == 1 {
            assert_eq!(
                source_entry_read_count(&fixture.root_path),
                5,
                "reopening after the parent completed must not reread its three entries"
            );
            assert_eq!(source_spool_open_count(&fixture.root_path), 2);
        }
        if directory_index < 3 {
            lease = fixture
                .catalog
                .lease_authoritative_library_change(
                    &fixture.root_id,
                    LibraryRootGeneration::initial(),
                    observed_unix_ms.saturating_add(1),
                    policy,
                )
                .expect("lease next nested directory")
                .expect("next nested directory");
        }
    }

    assert_eq!(source_entry_read_count(&fixture.root_path), 7);
    assert_eq!(source_spool_open_count(&fixture.root_path), 4);
    assert!(source_peak_staged_window(&fixture.root_path) <= 128);
    let evidence = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open nested spool evidence");
    let spool_state: String = evidence
        .query_row(
            "SELECT state FROM library_metadata_inventory_spools WHERE run_id = ?1",
            [&run_id],
            |row| row.get(0),
        )
        .expect("load nested spool state");
    assert_eq!(spool_state, "ready");
    let directories = {
        let mut statement = evidence
            .prepare(
                "SELECT relative_directory, ordinal, state, source_entry_count,
                        directory_identity_scheme IS NOT NULL,
                        directory_identity_value IS NOT NULL
                 FROM library_metadata_inventory_spool_directories
                 WHERE run_id = ?1 ORDER BY ordinal",
            )
            .expect("prepare nested directories");
        statement
            .query_map([&run_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, bool>(5)?,
                ))
            })
            .expect("query nested directories")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect nested directories")
    };
    assert_eq!(
        directories,
        vec![
            ("".to_owned(), 0, "completed".to_owned(), 3, true, true),
            (
                "album-a".to_owned(),
                1,
                "completed".to_owned(),
                2,
                true,
                true,
            ),
            (
                "album-b".to_owned(),
                2,
                "completed".to_owned(),
                1,
                true,
                true,
            ),
            (
                "album-a/nested".to_owned(),
                3,
                "completed".to_owned(),
                1,
                true,
                true,
            ),
        ]
    );
    drop(evidence);

    lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_004,
            policy,
        )
        .expect("lease ordered spool output")
        .expect("ordered spool output");
    let staged = process_leased_metadata_inventory_change_with_retained_source(
        &mut fixture.catalog,
        &root,
        &lease,
        MetadataInventoryRecoveryExecution::new(
            2_004,
            4_095,
            policy,
            MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
        ),
        retained,
    )
    .expect("stage complete nested scope");
    retained = staged.retained_source;
    let enumerated = fixture
        .catalog
        .load_metadata_inventory_run(&run_id)
        .expect("load staged nested run")
        .expect("staged nested run");
    assert!(enumerated.enumeration_complete);
    assert!(!enumerated.absence_authority);
    assert_eq!(enumerated.staged_entry_count, 7);

    let mut final_report = None;
    for step in 0..8_i64 {
        let observed_unix_ms = 2_005_i64.saturating_add(step);
        lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                observed_unix_ms,
                policy,
            )
            .expect("lease nested comparison")
            .expect("nested comparison");
        let page = process_leased_metadata_inventory_change_with_retained_source(
            &mut fixture.catalog,
            &root,
            &lease,
            MetadataInventoryRecoveryExecution::new(
                observed_unix_ms,
                4_095,
                policy,
                MetadataInventoryWorkerControl::without_progress(&AtomicBool::new(false)),
            ),
            retained,
        )
        .expect("publish nested comparison");
        retained = page.retained_source;
        let is_complete = page.report.inventory.is_complete;
        final_report = Some(page.report);
        if is_complete {
            break;
        }
    }
    let final_report = final_report.expect("nested final report");
    assert!(final_report.inventory.is_complete);
    let completed = fixture
        .catalog
        .load_metadata_inventory_run(&run_id)
        .expect("load completed nested run")
        .expect("completed nested run");
    assert!(completed.absence_authority);
    assert_eq!(source_entry_read_count(&fixture.root_path), 7);
    assert_eq!(source_spool_open_count(&fixture.root_path), 4);
}

#[test]
fn persisted_frontier_survives_every_page_reopen_without_replaying_metadata() {
    const DIRECTORY_COUNT: u64 = 9;
    let mut fixture = InventoryFixture::new(&[]);
    for index in 0..DIRECTORY_COUNT {
        write_png(
            &fixture
                .source
                .path()
                .join(format!("album-{index:02}"))
                .join("image.png"),
            2,
            2,
            [index as u8, 1, 2],
        );
    }
    let request = fixture.request_with_id("restart-safe-frontier", 1);
    let _authority = enqueue_authorized_recovery_control(&mut fixture, &request.run_id, 2_000);
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin restart-safe inventory");
    reset_source_root_entry_enumeration_count(&fixture.root_path);
    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    let mut expected_resume_opens = 0_u64;
    let mut page_count = 0_u64;

    loop {
        let run = fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load durable frontier")
            .expect("active durable frontier");
        let resume_frame_count = if run.next_page_index == 1 {
            1
        } else {
            u64::try_from(
                run.frontier
                    .iter()
                    .filter(|entry| {
                        entry.state == crate::domain::MetadataInventoryFrontierState::Enumerating
                    })
                    .count(),
            )
            .expect("bounded resume frame count")
        };
        expected_resume_opens = expected_resume_opens
            .checked_add(resume_frame_count.saturating_mul(2))
            .expect("bounded directory open count");
        let materialized_before = source_root_entry_enumeration_count(&fixture.root_path);
        let mut cancelled_source = LocalMetadataInventory::resume(
            &fixture.root_path,
            &request.scope,
            run.next_page_index,
            run.staged_entry_count,
            run.enumeration_cursor.as_deref(),
            &run.frontier,
        )
        .expect("resume cancelled source");
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            cancelled_source
                .next_page(3, &cancelled)
                .expect_err("cancel before page materialization")
                .code,
            "metadata_inventory_cancelled"
        );
        assert_eq!(
            source_root_entry_enumeration_count(&fixture.root_path),
            materialized_before
        );
        drop(cancelled_source);

        let mut source = LocalMetadataInventory::resume(
            &fixture.root_path,
            &request.scope,
            run.next_page_index,
            run.staged_entry_count,
            run.enumeration_cursor.as_deref(),
            &run.frontier,
        )
        .expect("resume production source");
        let page = source
            .next_page(3, &AtomicBool::new(false))
            .expect("next restart-safe page");
        let is_complete = page.is_complete;
        fixture
            .catalog
            .stage_metadata_inventory_page(
                &request.run_id,
                &page,
                3_000_i64
                    .checked_add(i64::try_from(page_count).expect("bounded page timestamp"))
                    .expect("bounded page timestamp"),
            )
            .expect("atomically stage page and frontier");
        page_count = page_count.checked_add(1).expect("bounded page count");
        drop(source);
        fixture.catalog = SqliteCatalog::open(catalog_path.clone()).expect("reopen catalog");
        if is_complete {
            break;
        }
    }

    let run = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load completed enumeration")
        .expect("completed enumeration");
    assert!(run.enumeration_complete);
    assert_eq!(run.staged_entry_count, DIRECTORY_COUNT.saturating_mul(2));
    assert_eq!(
        source_root_entry_enumeration_count(&fixture.root_path),
        DIRECTORY_COUNT.saturating_mul(2)
    );
    assert_eq!(
        source_directory_open_count(&fixture.root_path),
        expected_resume_opens.saturating_add(DIRECTORY_COUNT)
    );
    assert!(page_count > 1);
}

#[test]
fn page_frontier_failure_rolls_back_entries_cursor_and_frontier_together() {
    let mut fixture = InventoryFixture::new(&[]);
    write_png(&fixture.source.path().join("only.png"), 2, 2, [1, 2, 3]);
    let request = fixture.request_with_id("frontier-rollback", 1);
    let _authority = enqueue_authorized_recovery_control(&mut fixture, &request.run_id, 2_000);
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin rollback inventory");
    let mut source =
        LocalMetadataInventory::new(&fixture.root_path, &request.scope).expect("inventory source");
    let page = source
        .next_page(1, &AtomicBool::new(false))
        .expect("first inventory page");
    let mut invalid = page.clone();
    invalid.frontier[0].relative_directory = "a".repeat(32_768);

    fixture
        .catalog
        .stage_metadata_inventory_page(&request.run_id, &invalid, 2_001)
        .expect_err("frontier constraint failure");
    let rolled_back = fixture
        .catalog
        .load_metadata_inventory_run(&request.run_id)
        .expect("load rolled-back run")
        .expect("rolled-back run");
    assert_eq!(rolled_back.next_page_index, 1);
    assert_eq!(rolled_back.staged_entry_count, 0);
    assert!(rolled_back.enumeration_cursor.is_none());
    assert!(rolled_back.frontier.is_empty());

    let committed = fixture
        .catalog
        .stage_metadata_inventory_page(&request.run_id, &page, 2_002)
        .expect("retry exact page after rollback");
    assert_eq!(committed.next_page_index, 2);
    assert_eq!(committed.staged_entry_count, 1);
    assert_eq!(committed.frontier, page.frontier);
}

#[test]
fn newer_gap_supersedes_an_incomplete_inventory_epoch_and_its_staged_candidates() {
    let mut fixture = InventoryFixture::new(&[]);
    write_png(&fixture.source.path().join("first.png"), 2, 2, [10, 20, 30]);
    write_png(
        &fixture.source.path().join("second.png"),
        2,
        2,
        [40, 50, 60],
    );
    let policy = queue_policy();
    let first_intent = LibraryChangeIntent {
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        kind: LibraryChangeIntentKind::FreshnessUnknown,
        scope: LibraryChangeScope::Root,
        relative_path: String::new(),
        previous_relative_path: None,
        origin: LibraryChangeOrigin::ConsistencyAudit,
        first_observed_unix_ms: 2_000,
        most_recent_observed_unix_ms: 2_000,
        first_sequence: 1,
        most_recent_sequence: 1,
        coalesced_observation_count: 1,
    };
    fixture
        .catalog
        .enqueue_library_change_intents(std::slice::from_ref(&first_intent), 2_000, policy)
        .expect("enqueue first epoch");
    let first_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease first epoch")
        .expect("first epoch");
    let first_run_id = metadata_inventory_run_id(&first_lease);
    authorize_leased_containment_recovery(&mut fixture, &first_lease, &first_run_id, 2_000);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let first_report = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &first_lease,
        2_000,
        1,
        policy,
        &AtomicBool::new(false),
    )
    .expect("start first epoch");
    assert!(!first_report.inventory.is_complete);
    let first_spool = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open first epoch spool evidence");
    let first_spool_state: (i64, i64) = first_spool
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories
                WHERE run_id = ?1)",
            [&first_run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load first epoch spool evidence");
    assert_eq!(first_spool_state.0, 1);
    assert!(first_spool_state.1 > 0);
    drop(first_spool);

    let second_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            3_000,
            policy,
        )
        .expect("lease newer epoch")
        .expect("newer epoch");
    assert_eq!(second_lease.change.id, first_lease.change.id);
    assert!(second_lease.lease_generation > first_lease.lease_generation);
    let second_run_id = "same-authority-newer-inventory-run".to_owned();
    authorize_leased_containment_recovery(&mut fixture, &second_lease, &second_run_id, 3_000);
    process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &second_lease,
        3_000,
        1,
        policy,
        &AtomicBool::new(false),
    )
    .expect("start newer epoch");

    let first_run = fixture
        .catalog
        .load_metadata_inventory_run(&first_run_id)
        .expect("load first run")
        .expect("first run");
    let second_run = fixture
        .catalog
        .load_metadata_inventory_run(&second_run_id)
        .expect("load second run")
        .expect("second run");
    assert_eq!(first_run.status, MetadataInventoryRunStatus::Superseded);
    assert_eq!(second_run.request.epoch, first_run.request.epoch + 1);
    assert!(first_run.frontier.iter().all(|entry| {
        entry.state == crate::domain::MetadataInventoryFrontierState::Completed
            && entry.resume_after_relative_path.is_none()
            && entry.enumerated_entry_count == 0
    }));

    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let reopened =
        SqliteCatalog::open(catalog_path.clone()).expect("reopen superseded spool catalog");
    drop(reopened);
    let evidence =
        rusqlite::Connection::open(catalog_path).expect("open superseded spool evidence");
    let old_spool_state: (i64, i64, i64, i64) = evidence
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_metadata_inventory_spools WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_directories
                WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_spool_entries
                WHERE run_id = ?1),
               (SELECT COUNT(*) FROM library_metadata_inventory_frontier
                WHERE run_id = ?1 AND (
                  state <> 'completed' OR resume_after_relative_path IS NOT NULL
                  OR enumerated_entry_count <> 0
                ))",
            [&first_run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("load superseded spool evidence");
    assert_eq!(old_spool_state, (0, 0, 0, 0));
}

#[test]
fn active_p2_owner_keeps_exact_lease_affinity_until_completion() {
    let mut fixture =
        InventoryFixture::new(&["first.png", "second.png", "third.png", "fourth.png"]);
    let policy = queue_policy();
    let old_run_id = "active-affinity-old-run";
    let old_lease = enqueue_authorized_recovery_control_with_reason(
        &mut fixture,
        old_run_id,
        1_000,
        LibraryRecoveryAuthorityReason::WatcherUncoveredGap,
    );
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let first_page = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &old_lease,
        1_000,
        1,
        policy,
        &AtomicBool::new(false),
    )
    .expect("start exact older P2 owner");
    assert!(!first_page.inventory.is_complete);

    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: 3_000,
                most_recent_observed_unix_ms: 3_000,
                first_sequence: 2,
                most_recent_sequence: 2,
                coalesced_observation_count: 1,
            }],
            3_000,
            policy,
        )
        .expect("enqueue distinct newer live gap");
    let live_gap = fixture
        .catalog
        .lease_live_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            3_000,
            policy,
        )
        .expect("lease distinct newer live gap")
        .expect("distinct newer live gap");
    assert_ne!(live_gap.change.id, old_lease.change.id);
    fixture
        .catalog
        .promote_live_watcher_gap_to_metadata_inventory(
            live_gap.change.id,
            live_gap.lease_generation,
            &LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "Fixture watcher gap requires metadata inventory".to_owned(),
            },
            3_000,
            policy,
        )
        .expect("promote distinct newer live gap");

    let (new_change_id, new_run_id): (i64, String) =
        rusqlite::Connection::open(fixture.catalog.catalog_path())
            .expect("open distinct authority evidence")
            .query_row(
                "SELECT change_id, run_id
                 FROM library_recovery_authorities
                 WHERE root_id = ?1 AND root_generation = 1
                   AND reason = 'watcher_uncovered_gap'
                   AND retired_unix_ms IS NULL AND change_id <> ?2",
                rusqlite::params![
                    &fixture.root_id,
                    i64::try_from(old_lease.change.id.value()).expect("old change ID"),
                ],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load distinct newer authority");
    let new_change_id =
        LibraryChangeId::new(u64::try_from(new_change_id).expect("positive newer change ID"))
            .expect("nonzero newer change ID");
    let old_run = fixture
        .catalog
        .load_metadata_inventory_run(old_run_id)
        .expect("load active old run")
        .expect("active old run");
    let rejected_request = MetadataInventoryRunRequest {
        run_id: new_run_id.clone(),
        root_id: fixture.root_id.clone(),
        root_generation: LibraryRootGeneration::initial(),
        epoch: old_run.request.epoch + 1,
        scope: MetadataInventoryScope::Root,
        started_unix_ms: 3_000,
    };
    let state = |path: &Path| {
        rusqlite::Connection::open(path)
            .expect("open owner state")
            .query_row(
                "SELECT old_queue.status, old_queue.lease_generation,
                        old_authority.retired_unix_ms, old_run.status,
                        (SELECT COUNT(*) FROM library_metadata_inventory_spools
                         WHERE run_id = old_run.id),
                        (SELECT COUNT(*) FROM library_metadata_inventory_frontier
                         WHERE run_id = old_run.id AND state <> 'completed'),
                        new_queue.status, new_queue.lease_generation,
                        new_authority.retired_unix_ms,
                        (SELECT COUNT(*) FROM library_metadata_inventory_runs
                         WHERE id = new_authority.run_id)
                 FROM library_change_queue AS old_queue
                 JOIN library_recovery_authorities AS old_authority
                   ON old_authority.change_id = old_queue.id
                 JOIN library_metadata_inventory_runs AS old_run
                   ON old_run.id = old_authority.run_id
                 JOIN library_change_queue AS new_queue ON new_queue.id = ?2
                 JOIN library_recovery_authorities AS new_authority
                   ON new_authority.change_id = new_queue.id
                 WHERE old_queue.id = ?1",
                rusqlite::params![
                    i64::try_from(old_lease.change.id.value()).expect("old change ID"),
                    i64::try_from(new_change_id.value()).expect("new change ID"),
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, Option<i64>>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                },
            )
            .expect("load owner state")
    };
    let before_rejection = state(fixture.catalog.catalog_path());
    let error = fixture
        .catalog
        .begin_metadata_inventory(&rejected_request)
        .expect_err("distinct P2 authority must not supersede the active owner");
    assert_eq!(error.code, "metadata_inventory_active_authority_conflict");
    assert_eq!(state(fixture.catalog.catalog_path()), before_rejection);

    let mut old_completed = false;
    for now_unix_ms in 3_200..3_240 {
        let lease = fixture
            .catalog
            .lease_metadata_inventory_recovery(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms,
                policy,
            )
            .expect("lease exact active P2 owner")
            .expect("exact active P2 owner remains leaseable");
        assert_eq!(lease.change.id, old_lease.change.id);
        let new_state: (String, Option<i64>, i64) =
            rusqlite::Connection::open(fixture.catalog.catalog_path())
                .expect("open deferred newer owner evidence")
                .query_row(
                    "SELECT queue.status, authority.retired_unix_ms,
                            (SELECT COUNT(*) FROM library_metadata_inventory_runs
                             WHERE id = authority.run_id)
                     FROM library_change_queue AS queue
                     JOIN library_recovery_authorities AS authority
                       ON authority.change_id = queue.id
                     WHERE queue.id = ?1",
                    [i64::try_from(new_change_id.value()).expect("new change ID")],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("load deferred newer owner evidence");
        assert_eq!(new_state, ("pending".to_owned(), None, 0));
        let report = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &lease,
            now_unix_ms,
            128,
            policy,
            &AtomicBool::new(false),
        )
        .expect("continue exact active P2 owner");
        process_ready_library_changes(
            &mut fixture.catalog,
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            now_unix_ms,
            policy,
        )
        .expect("publish exact older owner candidates");
        if report.inventory.is_complete {
            old_completed = true;
            break;
        }
    }
    assert!(old_completed, "the exact old P2 owner did not complete");

    let new_lease = fixture
        .catalog
        .lease_metadata_inventory_recovery(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            4_000,
            policy,
        )
        .expect("lease newer owner after exact owner completion")
        .expect("newer owner becomes leaseable after exact owner completion");
    assert_eq!(new_lease.change.id, new_change_id);
    let mut next_lease = Some(new_lease);
    let mut new_completed = false;
    for now_unix_ms in 4_000..4_040 {
        let lease = next_lease.take().unwrap_or_else(|| {
            fixture
                .catalog
                .lease_metadata_inventory_recovery(
                    &fixture.root_id,
                    LibraryRootGeneration::initial(),
                    now_unix_ms,
                    policy,
                )
                .expect("lease newer owner continuation")
                .expect("newer owner remains leaseable")
        });
        assert_eq!(lease.change.id, new_change_id);
        let report = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &lease,
            now_unix_ms,
            128,
            policy,
            &AtomicBool::new(false),
        )
        .expect("process newer P2 owner");
        process_ready_library_changes(
            &mut fixture.catalog,
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            now_unix_ms,
            policy,
        )
        .expect("publish newer owner candidates");
        if report.inventory.is_complete {
            new_completed = true;
            break;
        }
    }
    assert!(new_completed, "the newer P2 owner did not converge");
    assert!(
        fixture
            .catalog
            .lease_metadata_inventory_recovery(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                5_000,
                policy,
            )
            .expect("verify no terminal recovery lease")
            .is_none()
    );

    let catalog_path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    drop(
        SqliteCatalog::open(catalog_path.clone()).expect("reopen converged owner-affinity catalog"),
    );
    let terminal: (
        String,
        Option<i64>,
        String,
        i64,
        String,
        Option<i64>,
        String,
    ) = rusqlite::Connection::open(catalog_path)
        .expect("open terminal owner evidence")
        .query_row(
            "SELECT old_queue.status, old_authority.retired_unix_ms, old_run.status,
                        (SELECT COUNT(*) FROM library_metadata_inventory_spools
                         WHERE run_id = old_run.id),
                        new_queue.status, new_authority.retired_unix_ms, new_run.status
                 FROM library_change_queue AS old_queue
                 JOIN library_recovery_authorities AS old_authority
                   ON old_authority.change_id = old_queue.id
                 JOIN library_metadata_inventory_runs AS old_run
                   ON old_run.id = old_authority.run_id
                 JOIN library_change_queue AS new_queue ON new_queue.id = ?2
                 JOIN library_recovery_authorities AS new_authority
                   ON new_authority.change_id = new_queue.id
                 JOIN library_metadata_inventory_runs AS new_run
                   ON new_run.id = new_authority.run_id
                 WHERE old_queue.id = ?1",
            rusqlite::params![
                i64::try_from(old_lease.change.id.value()).expect("old change ID"),
                i64::try_from(new_change_id.value()).expect("new change ID"),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .expect("load terminal owner evidence");
    assert_eq!(terminal.0, "completed");
    assert!(terminal.1.is_some());
    assert_eq!(terminal.2, "completed");
    assert_eq!(terminal.3, 0);
    assert_eq!(terminal.4, "completed");
    assert!(terminal.5.is_some());
    assert_eq!(terminal.6, "completed");
}
#[test]
fn inventory_failure_exhausts_durable_retry_without_starting_a_full_scan() {
    let mut fixture = InventoryFixture::new(&[]);
    fs::remove_dir(fixture.source.path()).expect("remove source fixture");
    let policy = LibraryChangeQueuePolicy {
        max_attempts: 2,
        retry_initial_delay_millis: 1,
        retry_maximum_delay_millis: 1,
        ..queue_policy()
    };
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue failing inventory");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    for now_unix_ms in [2_000, 2_001] {
        let leased = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now_unix_ms,
                policy,
            )
            .expect("lease failing inventory")
            .expect("failing inventory authority");
        let run_id = metadata_inventory_run_id(&leased);
        authorize_leased_containment_recovery(&mut fixture, &leased, &run_id, now_unix_ms);
        let report = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &leased,
            now_unix_ms,
            16,
            policy,
            &AtomicBool::new(false),
        )
        .expect("record inventory failure");
        assert_eq!(report.incremental.retried_count, 1);
    }
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_001,
            policy,
        )
        .expect("load exhausted inventory metrics");

    assert_eq!(metrics.exhausted_retry_count, 1);
    assert_eq!(metrics.ready_count, 0);
    assert_eq!(
        metrics.latest_exhausted_failure_code.as_deref(),
        Some("root_publication_namespace_unavailable")
    );
    assert!(
        fixture
            .catalog
            .load_recoverable_scan()
            .expect("recoverable scan")
            .is_none()
    );
}

#[test]
fn cancelled_automatic_inventory_defers_authority_and_preserves_absence() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    fs::remove_file(fixture.source.path().join("retained.png")).expect("remove fixture");
    let policy = queue_policy();
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: 2_000,
                most_recent_observed_unix_ms: 2_000,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            2_000,
            policy,
        )
        .expect("enqueue cancellable inventory");
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("lease cancellable inventory")
        .expect("cancellable inventory");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");
    let run_id = metadata_inventory_run_id(&leased);
    authorize_leased_containment_recovery(&mut fixture, &leased, &run_id, 2_000);

    let report = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &leased,
        2_000,
        1,
        policy,
        &AtomicBool::new(true),
    )
    .expect("cancel inventory");
    let metrics = fixture
        .catalog
        .load_library_change_root_queue_metrics(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy,
        )
        .expect("load cancelled inventory metrics");

    assert!(report.inventory.is_cancelled);
    assert_eq!(report.incremental.deferred_count, 1);
    assert_eq!(metrics.pending_count, 1);
    assert!(fixture.location("retained.png").is_some());
    assert_eq!(
        fixture
            .catalog
            .load_metadata_inventory_run(&run_id)
            .expect("load cancelled recovery frontier")
            .expect("cancelled recovery frontier")
            .status,
        MetadataInventoryRunStatus::Running
    );
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_recovery_authority(leased.change.id)
            .expect("load cancelled recovery authority")
            .is_some_and(|authority| authority.retired_unix_ms.is_none())
    );
}

#[test]
fn terminal_inventory_cleanup_deletes_staging_in_bounded_batches() {
    let mut fixture = InventoryFixture::new(&[]);
    let request = fixture.request();
    fixture
        .catalog
        .begin_metadata_inventory(&request)
        .expect("begin inventory");
    fixture
        .catalog
        .stage_metadata_inventory_page(
            &request.run_id,
            &MetadataInventoryPage {
                page_index: 1,
                entries: vec![
                    metadata_entry("a.txt"),
                    metadata_entry("b.txt"),
                    metadata_entry("c.txt"),
                ],
                cursor: Some("c.txt".to_owned()),
                is_complete: false,
                frontier: vec![MetadataInventoryFrontierEntry::enumerating(
                    0,
                    "",
                    None,
                    Some("c.txt".to_owned()),
                    3,
                )],
            },
            2_100,
        )
        .expect("stage entries");
    fixture
        .catalog
        .terminate_metadata_inventory(
            &request.run_id,
            MetadataInventoryRunStatus::Failed,
            Some(("fixture_failure", "fixture failure")),
            2_200,
        )
        .expect("terminate inventory");

    let first = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 2, 1)
        .expect("first cleanup batch");
    assert_eq!(first.removed_entry_count, 2);
    assert_eq!(first.removed_run_count, 0);
    assert!(first.has_more);
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path.clone())
        .expect("reopen partially cleaned terminal inventory");
    let second = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(0, 2, 1)
        .expect("second cleanup batch");
    assert_eq!(second.removed_entry_count, 1);
    assert_eq!(second.removed_run_count, 0);
    assert!(!second.has_more);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run(&request.run_id)
            .expect("load retained summary")
            .is_some()
    );
    drop(fixture.catalog);
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("reopen cleaned catalog");
    let expired = fixture
        .catalog
        .cleanup_terminal_metadata_inventories(3_000, 2, 1)
        .expect("expired summary cleanup");
    assert_eq!(expired.removed_entry_count, 0);
    assert_eq!(expired.removed_run_count, 1);
    assert!(!expired.has_more);
}

#[cfg(windows)]
#[test]
fn one_previous_path_is_not_reused_for_multiple_hard_link_candidates() {
    let mut fixture = InventoryFixture::new(&["old.png"]);
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new-a.png"),
    )
    .expect("rename fixture");
    fs::hard_link(
        fixture.source.path().join("new-a.png"),
        fixture.source.path().join("new-b.png"),
    )
    .expect("hard link fixture");

    let report = fixture.run_inventory(16, &AtomicBool::new(false));
    let incremental = process_ready_library_changes(
        &mut fixture.catalog,
        &fixture.root_id,
        LibraryRootGeneration::initial(),
        2_000,
        queue_policy(),
    )
    .expect("process hard link candidates");
    let first = fixture.location("new-a.png").expect("first hard link");
    let second = fixture.location("new-b.png").expect("second hard link");

    assert!(report.is_complete);
    assert_eq!(report.candidate_count, 2);
    assert_eq!(incremental.completed_count, 2);
    assert!(fixture.location("old.png").is_none());
    assert_eq!(first.asset_id, second.asset_id);
}

#[test]
fn metadata_identity_window_is_empty_safe_and_bounded() {
    let fixture = InventoryFixture::new(&[]);
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_previous_paths("missing-run", &[])
            .expect("load empty identity window")
            .is_empty()
    );
    let identities = vec![
        FileIdentityEvidence {
            scheme: "fixture".to_owned(),
            value: "identity".to_owned(),
        };
        4_097
    ];
    let error = fixture
        .catalog
        .load_metadata_inventory_previous_paths("missing-run", &identities)
        .expect_err("reject oversized identity window");
    assert_eq!(error.code, "metadata_inventory_identity_window_invalid");
}

#[test]
fn baseline_blocks_absence_until_closing_and_completes_atomically_after_restart() {
    let mut fixture = InventoryFixture::new(&[]);
    fixture
        .catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: 1_000,
        })
        .expect("baseline capability");
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: "fixture-volume".to_owned(),
        volume_serial: 42,
    };
    let baseline = fixture
        .catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: "migration-baseline-run".to_owned(),
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                authority_reason:
                    crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: volume.clone(),
                root_file_reference: JournalFileReference::V3([7; 16]),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                opening_next_usn: JournalUsn::new(10).expect("opening USN"),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: 1_000,
            },
            queue_policy(),
        )
        .expect("admit migration baseline");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load root")
        .expect("root");

    let first = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            1_000,
            queue_policy(),
        )
        .expect("lease enumeration")
        .expect("baseline enumeration");
    let enumeration = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &first,
        1_000,
        4_096,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("enumerate baseline page");
    assert!(!enumeration.inventory.awaiting_closing_boundary);

    let mut waiting = None;
    for step in 0..8_i64 {
        let observed_unix_ms = 1_100_i64.saturating_add(step);
        let lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                observed_unix_ms,
                queue_policy(),
            )
            .expect("lease comparison")
            .expect("baseline comparison");
        let current = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &lease,
            observed_unix_ms,
            4_096,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("reach closing barrier");
        if current.inventory.awaiting_closing_boundary {
            waiting = Some(current);
            break;
        }
    }
    let waiting = waiting.expect("bounded source preparation reaches closing barrier");
    assert!(waiting.inventory.awaiting_closing_boundary);
    assert!(!waiting.inventory.is_complete);
    let run = fixture
        .catalog
        .load_metadata_inventory_run("migration-baseline-run")
        .expect("load waiting inventory")
        .expect("waiting inventory");
    assert!(!run.absence_authority);

    fixture
        .catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &PersistentJournalBaselineClosingBoundary {
                change_id: baseline.change_id,
                volume,
                root_file_reference: JournalFileReference::V3([7; 16]),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                closing_next_usn: JournalUsn::new(10).expect("closing USN"),
                protocol_version: 5,
                captured_unix_ms: 1_200,
            },
        )
        .expect("capture closing boundary");
    let catalog_path = fixture._storage.path().join("catalog.sqlite3");
    fixture.catalog = SqliteCatalog::open(catalog_path).expect("restart baseline catalog");

    let final_lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            1_300,
            queue_policy(),
        )
        .expect("lease closing inventory")
        .expect("closing inventory");
    let completed = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &final_lease,
        1_300,
        4_096,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("complete baseline atomically");
    let authority = fixture
        .catalog
        .load_metadata_inventory_recovery_authority(baseline.change_id)
        .expect("load retired authority")
        .expect("retired authority");
    assert!(completed.inventory.is_complete);
    assert_eq!(completed.incremental.completed_count, 1);
    assert!(authority.retired_unix_ms.is_some());
    assert!(
        fixture
            .catalog
            .load_persistent_journal_baselines()
            .expect("active baselines after completion")
            .is_empty()
    );
}

#[cfg(windows)]
#[test]
fn replacement_before_absence_publication_preserves_the_last_trusted_catalog() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    let active_scan_id = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load initial root")
        .expect("initial root")
        .active_scan_id;
    fs::remove_file(fixture.source.path().join("retained.png"))
        .expect("remove source before baseline");
    let (change_id, root, volume) =
        prepare_baseline_closing_wait(&mut fixture, "absence-root-rebinding", 8_000);
    let original = fixture.source.path().to_path_buf();
    let moved = original.with_file_name("absence-root-rebinding-moved");
    fs::rename(&original, &moved).expect("rename configured root");
    fs::create_dir(&original).expect("create configured-path replacement");
    capture_baseline_closing_boundary(&mut fixture, change_id, volume, 8_100);

    let lease = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            8_200,
            queue_policy(),
        )
        .expect("lease absence publication")
        .expect("absence publication work");
    let rejected = process_leased_metadata_inventory_change(
        &mut fixture.catalog,
        &root,
        &lease,
        8_200,
        4_096,
        queue_policy(),
        &AtomicBool::new(false),
    )
    .expect("root rebinding failure is durably retried");

    assert_eq!(rejected.incremental.completed_count, 0);
    assert_eq!(rejected.incremental.applied_mutation_count, 0);
    assert_eq!(rejected.incremental.retried_count, 1);
    assert!(fixture.location("retained.png").is_some());
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load retained root")
            .expect("retained root")
            .active_scan_id,
        active_scan_id
    );
    let run = fixture
        .catalog
        .load_metadata_inventory_run("absence-root-rebinding")
        .expect("load retained run")
        .expect("retained run");
    assert!(!run.absence_authority);
    let authority = fixture
        .catalog
        .load_metadata_inventory_recovery_authority(change_id)
        .expect("load retained authority")
        .expect("retained authority");
    assert!(authority.retired_unix_ms.is_none());
    assert_no_current_journal_authority(&fixture);

    fs::remove_dir(&original).expect("remove replacement root");
    fs::rename(&moved, &original).expect("restore disposable source root");
}

#[cfg(windows)]
#[test]
fn replacement_before_finalizer_preserves_catalog_checkpoint_and_authority() {
    let mut fixture = InventoryFixture::new(&["retained.png"]);
    let active_scan_id = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load initial root")
        .expect("initial root")
        .active_scan_id;
    fs::remove_file(fixture.source.path().join("retained.png"))
        .expect("remove source before baseline");
    let (change_id, root, volume) =
        prepare_baseline_closing_wait(&mut fixture, "finalizer-root-rebinding", 9_000);
    capture_baseline_closing_boundary(&mut fixture, change_id, volume, 9_100);
    let original = fixture.source.path().to_path_buf();
    let moved = original.with_file_name("finalizer-root-rebinding-moved");
    set_before_metadata_inventory_finalization_hook({
        let original = original.clone();
        let moved = moved.clone();
        move || {
            fs::rename(&original, &moved).expect("rename root before finalizer pin");
            fs::create_dir(&original).expect("create finalizer-path replacement");
        }
    });

    let mut failure = None;
    for step in 0..8_i64 {
        let now = 9_200_i64.saturating_add(step);
        let lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now,
                queue_policy(),
            )
            .expect("lease finalization work")
            .expect("finalization work");
        match process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &lease,
            now,
            4_096,
            queue_policy(),
            &AtomicBool::new(false),
        ) {
            Ok(report) => {
                assert_eq!(report.incremental.completed_count, 0);
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    let failure = failure.expect("finalizer must rebind the configured root");
    assert!(
        matches!(
            failure.code.as_str(),
            "metadata_inventory_root_identity_changed" | "root_identity_unavailable"
        ),
        "unexpected finalizer rejection: {}: {}",
        failure.code,
        failure.message
    );
    assert!(fixture.location("retained.png").is_some());
    assert_eq!(
        fixture
            .catalog
            .load_incremental_catalog_root(&fixture.root_id)
            .expect("load retained root")
            .expect("retained root")
            .active_scan_id,
        active_scan_id
    );
    let authority = fixture
        .catalog
        .load_metadata_inventory_recovery_authority(change_id)
        .expect("load retained authority")
        .expect("retained authority");
    assert!(authority.retired_unix_ms.is_none());
    assert_no_current_journal_authority(&fixture);

    fs::remove_dir(&original).expect("remove replacement root");
    fs::rename(&moved, &original).expect("restore disposable source root");
}

fn prepare_baseline_closing_wait(
    fixture: &mut InventoryFixture,
    run_id: &str,
    started_unix_ms: i64,
) -> (
    LibraryChangeId,
    IncrementalCatalogRoot,
    PersistentJournalVolumeIdentity,
) {
    fixture
        .catalog
        .save_persistent_journal_capability(&PersistentJournalCapability {
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            protocol_version: 5,
            contract_version: 1,
            state: PersistentJournalCapabilityState::Supported,
            continuity: PersistentJournalContinuityState::BaselineRequired,
            failure: None,
            updated_unix_ms: started_unix_ms,
        })
        .expect("baseline capability");
    let volume = PersistentJournalVolumeIdentity {
        volume_guid: format!("{run_id}-volume"),
        volume_serial: 42,
    };
    let baseline = fixture
        .catalog
        .begin_persistent_journal_baseline(
            &PersistentJournalBaselineStartRequest {
                run_id: run_id.to_owned(),
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                authority_reason:
                    crate::domain::LibraryRecoveryAuthorityReason::ExistingRootBaseline,
                volume: volume.clone(),
                root_file_reference: JournalFileReference::V3([7; 16]),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                opening_next_usn: JournalUsn::new(10).expect("opening USN"),
                protocol_version: 5,
                contract_version: 1,
                authorized_unix_ms: started_unix_ms,
            },
            queue_policy(),
        )
        .expect("admit baseline");
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load baseline root")
        .expect("baseline root");
    let mut waiting = false;
    for step in 0..12_i64 {
        let now = started_unix_ms.saturating_add(step);
        let lease = fixture
            .catalog
            .lease_authoritative_library_change(
                &fixture.root_id,
                LibraryRootGeneration::initial(),
                now,
                queue_policy(),
            )
            .expect("lease baseline work")
            .expect("baseline work");
        let current = process_leased_metadata_inventory_change(
            &mut fixture.catalog,
            &root,
            &lease,
            now,
            4_096,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("advance baseline to closing wait");
        if current.inventory.awaiting_closing_boundary {
            waiting = true;
            break;
        }
    }
    assert!(waiting, "baseline reaches its closing boundary wait");
    let run = fixture
        .catalog
        .load_metadata_inventory_run(run_id)
        .expect("load waiting baseline")
        .expect("waiting baseline");
    assert!(!run.absence_authority);
    (baseline.change_id, root, volume)
}

fn capture_baseline_closing_boundary(
    fixture: &mut InventoryFixture,
    change_id: LibraryChangeId,
    volume: PersistentJournalVolumeIdentity,
    captured_unix_ms: i64,
) {
    fixture
        .catalog
        .capture_persistent_journal_baseline_closing_boundary(
            &PersistentJournalBaselineClosingBoundary {
                change_id,
                volume,
                root_file_reference: JournalFileReference::V3([7; 16]),
                journal_id: JournalIdentifier::new(9).expect("journal ID"),
                closing_next_usn: JournalUsn::new(10).expect("closing USN"),
                protocol_version: 5,
                captured_unix_ms,
            },
        )
        .expect("capture closing boundary");
}

fn assert_no_current_journal_authority(fixture: &InventoryFixture) {
    let connection = rusqlite::Connection::open(fixture._storage.path().join("catalog.sqlite3"))
        .expect("open journal authority evidence");
    let current: (i64, i64) = connection
        .query_row(
            "SELECT
               (SELECT COUNT(*) FROM library_persistent_journal_root_state
                WHERE root_id = ?1 AND continuity_state = 'current'),
               (SELECT COUNT(*) FROM library_persistent_journal_checkpoints
                WHERE root_id = ?1 AND continuity_state = 'current')",
            [&fixture.root_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("load journal authority evidence");
    assert_eq!(current, (0, 0));
}

struct FixedInventorySource {
    page: Option<MetadataInventoryPage>,
}

impl MetadataInventorySource for FixedInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        self.page.take().ok_or_else(|| {
            ScanError::new(
                "fixed_inventory_exhausted",
                "The fixed inventory page was already consumed",
            )
        })
    }
}

struct FailingInventorySource;

impl MetadataInventorySource for FailingInventorySource {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<MetadataInventoryPage, ScanError> {
        Err(ScanError::new(
            "metadata_inventory_fixture_failure",
            "The metadata inventory fixture failed",
        ))
    }
}

struct InventoryFixture {
    catalog: SqliteCatalog,
    source: TempDir,
    _storage: TempDir,
    root_id: String,
    root_path: String,
}

impl InventoryFixture {
    fn new(relative_paths: &[&str]) -> Self {
        let source = tempdir().expect("source directory");
        for relative_path in relative_paths {
            let path = source.path().join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent");
            }
            write_png(&path, 2, 2, [10, 20, 30]);
        }
        let storage = tempdir().expect("storage directory");
        let storage_paths = StoragePaths {
            catalog_path: storage.path().join("catalog.sqlite3"),
            preview_root: storage.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage.path().join("settings.sqlite3"),
        };
        let root_path = FileDiscovery::new(&source.path().to_string_lossy())
            .expect("source discovery")
            .canonical_root()
            .expect("canonical root")
            .to_string_lossy()
            .into_owned();
        let root_id = stable_id("library-root-v1", &root_path);
        let scan_id = stable_id("metadata-inventory-initial-scan-v1", &root_path);
        run_scan_with_storage(
            ScanRequest {
                scan_id,
                root_path: root_path.clone(),
                max_items: None,
                max_entries: None,
                preview_edge: 128,
            },
            |_| true,
            storage_paths.clone(),
        )
        .expect("initial scan");
        let catalog = SqliteCatalog::open(storage_paths.catalog_path.clone()).expect("catalog");
        Self {
            source,
            _storage: storage,
            catalog,
            root_id,
            root_path,
        }
    }

    fn request(&self) -> MetadataInventoryRunRequest {
        self.request_with_id("inventory-1", 1)
    }

    fn request_with_id(&self, run_id: &str, epoch: u64) -> MetadataInventoryRunRequest {
        MetadataInventoryRunRequest {
            run_id: run_id.to_owned(),
            root_id: self.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            epoch,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 2_000,
        }
    }

    fn run_inventory(
        &mut self,
        page_limit: u32,
        cancellation: &AtomicBool,
    ) -> crate::domain::MetadataInventoryReport {
        let request = self.request();
        let mut source = LocalMetadataInventory::new(&self.root_path, &request.scope)
            .expect("local metadata inventory");
        run_metadata_inventory(
            &mut self.catalog,
            &mut source,
            &request,
            2_000,
            page_limit,
            queue_policy(),
            cancellation,
        )
        .expect("run metadata inventory")
    }

    fn run_inventory_with_id(
        &mut self,
        run_id: &str,
        epoch: u64,
        page_limit: u32,
    ) -> crate::domain::MetadataInventoryReport {
        let request = self.request_with_id(run_id, epoch);
        let mut source = LocalMetadataInventory::new(&self.root_path, &request.scope)
            .expect("local metadata inventory");
        run_metadata_inventory(
            &mut self.catalog,
            &mut source,
            &request,
            3_000,
            page_limit,
            queue_policy(),
            &AtomicBool::new(false),
        )
        .expect("run metadata inventory")
    }

    fn location(&self, relative_path: &str) -> Option<crate::domain::AssetLocationView> {
        self.catalog
            .load_incremental_location_by_relative_path(&self.root_id, relative_path)
            .expect("load location")
    }
}

fn open_inventory_source(
    fixture: &InventoryFixture,
    scope: &MetadataInventoryScope,
    run: &MetadataInventoryRun,
    leased: &LeasedLibraryChange,
) -> Box<dyn MetadataInventorySource> {
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("load inventory source root")
        .expect("registered inventory source root");
    let publication_identity = root
        .publication_root_identity
        .expect("initial scan publication identity");
    let opening_guard = PublicationGuardedFileDiscovery::new_metadata_inventory_source_guard(
        &fixture.root_path,
        Some(&publication_identity),
    )
    .expect("open inventory source namespace guard");
    let source_identity = opening_guard
        .metadata_inventory_root_identity()
        .expect("load inventory source identity")
        .expect("inventory source root identity");
    fixture
        .catalog
        .open_metadata_inventory_source(
            &fixture.root_path,
            scope,
            run,
            leased,
            Some(&publication_identity),
            &source_identity,
        )
        .expect("open durable inventory source")
}

fn enqueue_authorized_recovery_control(
    fixture: &mut InventoryFixture,
    run_id: &str,
    observed_unix_ms: i64,
) -> LeasedLibraryChange {
    enqueue_authorized_recovery_control_with_reason(
        fixture,
        run_id,
        observed_unix_ms,
        LibraryRecoveryAuthorityReason::ContainmentFailure,
    )
}

fn enqueue_authorized_recovery_control_with_reason(
    fixture: &mut InventoryFixture,
    run_id: &str,
    observed_unix_ms: i64,
    reason: LibraryRecoveryAuthorityReason,
) -> LeasedLibraryChange {
    fixture
        .catalog
        .enqueue_library_change_intents(
            &[LibraryChangeIntent {
                root_id: fixture.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::FreshnessUnknown,
                scope: LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::ConsistencyAudit,
                first_observed_unix_ms: observed_unix_ms,
                most_recent_observed_unix_ms: observed_unix_ms,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            }],
            observed_unix_ms,
            queue_policy(),
        )
        .expect("enqueue authorized recovery control");
    let leased = fixture
        .catalog
        .lease_authoritative_library_change(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            observed_unix_ms,
            queue_policy(),
        )
        .expect("lease authorized recovery control")
        .expect("authorized recovery control");
    fixture
        .catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: leased.change.id,
            run_id: run_id.to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            reason,
            opening_boundary: None,
            authorized_unix_ms: observed_unix_ms,
            retired_unix_ms: None,
        })
        .expect("persist recovery authority");
    leased
}

fn authorize_leased_containment_recovery(
    fixture: &mut InventoryFixture,
    leased: &LeasedLibraryChange,
    run_id: &str,
    observed_unix_ms: i64,
) {
    fixture
        .catalog
        .authorize_metadata_inventory_recovery(&LibraryRecoveryAuthority {
            change_id: leased.change.id,
            run_id: run_id.to_owned(),
            root_id: fixture.root_id.clone(),
            root_generation: LibraryRootGeneration::initial(),
            reason: LibraryRecoveryAuthorityReason::ContainmentFailure,
            opening_boundary: None,
            authorized_unix_ms: observed_unix_ms,
            retired_unix_ms: None,
        })
        .expect("persist recovery authority");
}

fn remove_inventory_publication_namespace_proof(fixture: &InventoryFixture) {
    let connection = rusqlite::Connection::open(fixture.catalog.catalog_path())
        .expect("open publication proof fixture");
    assert_eq!(
        connection
            .execute(
                "DELETE FROM library_root_publication_namespaces WHERE root_id = ?1",
                [&fixture.root_id],
            )
            .expect("remove publication proof fixture"),
        1
    );
}

fn queue_policy() -> LibraryChangeQueuePolicy {
    LibraryChangeQueuePolicy {
        debounce_millis: 0,
        max_lease_batch: 128,
        ..LibraryChangeQueuePolicy::default()
    }
}

fn metadata_entry(relative_path: &str) -> MetadataInventoryEntry {
    MetadataInventoryEntry {
        relative_path: relative_path.to_owned(),
        kind: MetadataInventoryEntryKind::File,
        file_size: Some(1),
        modified_unix_ms: 1,
        file_identity: None,
        source_revision: None,
        placeholder_state: MetadataInventoryPlaceholderState::Available,
        is_reparse_point: false,
    }
}

fn write_png(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("PNG parent");
    }
    RgbImage::from_pixel(width, height, Rgb(color))
        .save(path)
        .expect("write PNG fixture");
}
