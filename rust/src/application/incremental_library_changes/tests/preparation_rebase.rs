use std::cell::RefCell;
use std::rc::Rc;

use super::super::{PreparedChange, preparation_catalog::PreparationCatalog, revalidate_change};
use super::*;

#[test]
fn p1_revision_rebase_reuses_preparation_after_same_root_unrelated_live_publication() {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    let target = fixture.source.path().join("journal.png");
    write_png(&target, 2, 2, [10, 20, 30]);
    let original = fs::read(&target).expect("source bytes");
    fixture.enqueue(&[catch_up_intent(&fixture.root_id, "journal.png", 1)]);
    reset_source_content_open_instrumentation(&fixture.root_path);
    let counts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&counts);
    let root_id = fixture.root_id.clone();
    let root_path = fixture.root_path.clone();
    let live_path = fixture.source.path().join("live.png");
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, Vec::new());
    repository.on_publication = Some(Box::new(move |catalog, attempt| {
        observed
            .borrow_mut()
            .push(source_content_open_count(&root_path));
        if attempt == 1 {
            write_png(&live_path, 3, 2, [40, 50, 60]);
            publish_live(catalog, &root_id, "live.png");
            observed
                .borrow_mut()
                .push(source_content_open_count(&root_path));
        }
    }));

    let report = run_journal(&mut repository, &fixture.root_id);

    assert_eq!(repository.publication_attempts, 2);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    let counts = counts.borrow();
    assert_eq!(counts.len(), 3);
    assert!(
        counts[0] > 0,
        "the initial preparation must inspect actual media"
    );
    assert!(
        counts[1] > counts[0],
        "the competing P0 must inspect its own source"
    );
    assert_eq!(
        counts[2], counts[1],
        "rebase must not reopen unchanged media content"
    );
    let locations = repository
        .catalog
        .load_incremental_locations_in_subtree(&fixture.root_id, "", 3)
        .expect("published root locations");
    assert_eq!(locations.len(), 2);
    assert_eq!(
        fs::read(target).expect("source after publication"),
        original
    );
}

#[test]
fn p1_revision_rebase_reprepares_for_global_identity_appearance() {
    let parent = tempdir().expect("same-volume controlled parent");
    let source = tempdir_in(parent.path()).expect("source root");
    let alias_root = tempdir_in(parent.path()).expect("alias root");
    let mut fixture = seed_catalog(source, &[]);
    let target = fixture.source.path().join("journal.png");
    write_png(&target, 2, 2, [10, 20, 30]);
    fs::hard_link(&target, alias_root.path().join("alias.png")).expect("generated hardlink");
    seed_root(
        &mut fixture.catalog,
        "alias-root",
        "alias-baseline",
        alias_root.path(),
        &[],
    );
    let original = fs::read(&target).expect("source bytes");
    fixture.enqueue(&[catch_up_intent(&fixture.root_id, "journal.png", 1)]);
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, Vec::new());
    repository.on_publication = Some(Box::new(|catalog, attempt| {
        if attempt == 1 {
            publish_live(catalog, "alias-root", "alias.png");
        }
    }));

    let report = run_journal(&mut repository, &fixture.root_id);

    assert_eq!(repository.publication_attempts, 2);
    assert_eq!(report.completed_count, 1);
    let location = repository
        .catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "journal.png")
        .expect("source query")
        .expect("source location");
    let alias = repository
        .catalog
        .load_incremental_location_by_relative_path("alias-root", "alias.png")
        .expect("alias query")
        .expect("alias location");
    assert_eq!(location.asset_id, alias.asset_id);
    assert_eq!(location.file_identity, alias.file_identity);
    assert_eq!(fs::read(target).expect("source unchanged"), original);
}

#[test]
fn p1_revision_rebase_revalidates_same_identity_size_and_restored_mtime() {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    let path = fixture.source.path().join("same.bmp");
    write_bmp(&path, 4, 3, [10, 20, 30]);
    let initial = fixture.discovered("same.bmp");
    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let before = fs::read(&path).expect("initial bytes");
    fixture.enqueue(&[catch_up_intent(&fixture.root_id, "same.bmp", 1)]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let target = path.clone();
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, races);
    repository.on_publication = Some(Box::new(move |_, attempt| {
        if attempt == 1 {
            write_bmp(&target, 4, 3, [30, 20, 10]);
            fs::OpenOptions::new()
                .write(true)
                .open(&target)
                .expect("timestamp handle")
                .set_times(fs::FileTimes::new().set_modified(modified))
                .expect("restore mtime");
        }
    }));

    let report = run_journal(&mut repository, &fixture.root_id);

    assert_eq!(report.completed_count, 1);
    assert_eq!(repository.publication_attempts, 2);
    let after = repository
        .catalog
        .load_incremental_location_by_relative_path(&fixture.root_id, "same.bmp")
        .expect("location query")
        .expect("rewritten location");
    let current = FileDiscovery::new(&fixture.root_path).expect("discovery");
    let current = match current.visit_relative_path("same.bmp").outcome {
        FileVisitOutcome::File(file) => file,
        _ => panic!("rewritten image remains readable"),
    };
    assert_eq!(after.file_identity, initial.file_identity);
    assert_eq!(after.file_size, initial.file_size);
    assert_eq!(after.modified_unix_ms, initial.modified_unix_ms);
    assert_ne!(after.source_revision, initial.source_revision);
    assert_eq!(after.source_revision, current.source_revision);
    let after_bytes = fs::read(path).expect("rewritten bytes");
    assert_eq!(after_bytes.len(), before.len());
    assert_ne!(after_bytes, before);
}

#[test]
fn p1_revision_rebase_reconciles_an_unchanged_rename_source_removed_after_preparation() {
    let source = tempdir().expect("source");
    write_png(&source.path().join("old.png"), 2, 2, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let mut change = intent(&fixture.root_id, "missing.png", Some("old.png"), 1);
    change.origin = LibraryChangeOrigin::StartupCatchUp;
    fixture.enqueue(&[change]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let old = fixture.source.path().join("old.png");
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, races);
    repository.on_publication = Some(Box::new(move |_, attempt| {
        if attempt == 1 {
            fs::remove_file(&old).expect("remove generated old endpoint");
        }
    }));

    let report = run_journal(&mut repository, &fixture.root_id);

    assert_eq!(repository.publication_attempts, 2);
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.applied_mutation_count, 1);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "old.png")
            .expect("old endpoint query")
            .is_none()
    );
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "missing.png")
            .expect("new endpoint query")
            .is_none()
    );
}

#[test]
fn p1_revision_rebase_reconciles_absent_path_that_becomes_terminal_media() {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    fixture.enqueue(&[catch_up_intent(&fixture.root_id, "broken.jpg", 1)]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let path = fixture.source.path().join("broken.jpg");
    let target = path.clone();
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, races);
    repository.on_publication = Some(Box::new(move |_, attempt| {
        if attempt == 1 {
            fs::write(&target, b"not-a-jpeg").expect("generated malformed source");
        }
    }));

    let report = run_journal(&mut repository, &fixture.root_id);

    assert_eq!(repository.publication_attempts, 2);
    assert_eq!(report.completed_count, 1);
    let evidence = repository
        .catalog
        .load_terminal_media_evidence_by_relative_paths(
            &fixture.root_id,
            &["broken.jpg".to_owned()],
        )
        .expect("terminal evidence");
    assert_eq!(evidence.len(), 1);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "broken.jpg")
            .expect("terminal location query")
            .is_some()
    );
    assert_eq!(fs::read(path).expect("source preserved"), b"not-a-jpeg");
}

#[test]
fn first_publication_rejects_unchanged_source_rewritten_after_preparation() {
    let source = tempdir().expect("source");
    let path = source.path().join("same.bmp");
    write_bmp(&path, 4, 3, [10, 20, 30]);
    let modified = fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let mut fixture = seed_catalog(source, &["same.bmp"]);
    let (prepared, discovery) = prepare_one(&mut fixture, "same.bmp", None);
    assert!(
        prepared.mutations.is_empty(),
        "exercise the unchanged branch"
    );
    let revision = fixture.revision();

    write_bmp(&path, 4, 3, [30, 20, 10]);
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("timestamp handle")
        .set_times(fs::FileTimes::new().set_modified(modified))
        .expect("restore mtime");

    assert!(revalidate_change(&discovery, &prepared).is_err());
    assert_eq!(
        fixture.revision(),
        revision,
        "no stale-revision retry is involved"
    );
}

#[test]
fn first_publication_rejects_unchanged_rename_endpoint_removed_after_preparation() {
    let source = tempdir().expect("source");
    let old = source.path().join("old.png");
    write_png(&old, 2, 2, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let (prepared, discovery) = prepare_one(&mut fixture, "missing.png", Some("old.png"));
    assert!(
        prepared.mutations.is_empty(),
        "both observed endpoints initially require no mutation"
    );
    let revision = fixture.revision();

    fs::remove_file(old).expect("remove generated endpoint");

    assert!(revalidate_change(&discovery, &prepared).is_err());
    assert_eq!(fixture.revision(), revision);
    assert!(
        fixture.location("old.png").is_some(),
        "unpublished work preserves the baseline"
    );
}

#[test]
fn first_publication_rejects_absent_path_that_acquires_terminal_evidence() {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    let (prepared, discovery) = prepare_one(&mut fixture, "broken.jpg", None);
    let revision = fixture.revision();
    fs::write(fixture.source.path().join("broken.jpg"), b"not-a-jpeg")
        .expect("generated malformed source");

    let issue = revalidate_change(&discovery, &prepared).expect_err("absence proof changed");

    assert_eq!(issue.code, "incremental_source_changed_before_publication");
    assert_eq!(fixture.revision(), revision);
}

#[test]
fn preparation_read_set_preserves_none_and_inconsistent_repeated_path_reads() {
    let fixture = seed_catalog(tempdir().expect("source"), &[]);
    write_png(
        &fixture.source.path().join("incoming.png"),
        2,
        2,
        [10, 20, 30],
    );
    let mut recording = PreparationCatalog::new(&fixture.catalog);
    assert!(
        recording
            .load_incremental_location_by_relative_path(&fixture.root_id, "incoming.png")
            .expect("original absent prior")
            .is_none()
    );
    let mut writer = SqliteCatalog::open(fixture._storage.path().join("catalog.sqlite3"))
        .expect("independent writer");
    publish_live(&mut writer, &fixture.root_id, "incoming.png");
    assert!(
        recording
            .load_incremental_location_by_relative_path(&fixture.root_id, "incoming.png")
            .expect("later present prior")
            .is_some()
    );

    assert!(!recording.finish().still_matches(&fixture.catalog, None));
}

#[test]
fn preparation_read_set_compares_full_preview_evidence_for_path_and_identity() {
    let source = tempdir().expect("source");
    write_png(&source.path().join("image.png"), 2, 2, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["image.png"]);
    let mut location = fixture.location("image.png").expect("location");
    let identity = location.file_identity.clone().expect("identity");
    let mut path_reads = PreparationCatalog::new(&fixture.catalog);
    path_reads
        .load_incremental_location_by_relative_path(&fixture.root_id, "image.png")
        .expect("record path");
    let path_reads = path_reads.finish();
    let mut identity_reads = PreparationCatalog::new(&fixture.catalog);
    identity_reads
        .load_incremental_location_by_file_identity(&identity, &[])
        .expect("record global identity");
    let identity_reads = identity_reads.finish();
    assert!(path_reads.still_matches(&fixture.catalog, None));
    assert!(identity_reads.still_matches(&fixture.catalog, None));
    location.preview_status = PreviewStatus::Failed;
    location.preview_issue_code = Some("generated_preview_failure".to_owned());
    location.preview_issue_message = Some("fixture failure".to_owned());
    fixture
        .catalog
        .update_active_preview(&location, None, None)
        .expect("preview-only write");

    assert!(!path_reads.still_matches(&fixture.catalog, None));
    assert!(!identity_reads.still_matches(&fixture.catalog, None));
}

#[test]
fn preparation_read_set_budget_exhaustion_disables_reuse_without_failing_reads() {
    let fixture = seed_catalog(tempdir().expect("source"), &[]);
    let mut reads = PreparationCatalog::new(&fixture.catalog);
    for _ in 0..=LibraryChangeQueuePolicy::MAX_LEASE_BATCH * 5 {
        assert!(
            reads
                .load_incremental_location_by_relative_path(&fixture.root_id, "absent.png")
                .expect("bounded original lookup")
                .is_none()
        );
    }
    assert!(!reads.finish().still_matches(&fixture.catalog, None));
    let mut reads = PreparationCatalog::new(&fixture.catalog);
    let oversized_key = "x".repeat(8 * 1_024 * 1_024);
    assert!(
        reads
            .load_incremental_location_by_relative_path(&fixture.root_id, &oversized_key)
            .expect("lookup result survives optional-proof byte limit")
            .is_none()
    );
    assert!(!reads.finish().still_matches(&fixture.catalog, None));
}

#[test]
fn preparation_read_set_replays_global_identity_and_ordered_catch_up_lineage() {
    let fixture = seed_catalog(tempdir().expect("source"), &[]);
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, Vec::new());
    let observed = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&observed);
    repository.on_identity_read = Some(Box::new(move |identity, lineage| {
        captured
            .borrow_mut()
            .push((identity.clone(), lineage.to_vec()));
    }));
    let identity = FileIdentityEvidence {
        scheme: "fixture".to_owned(),
        value: "absent".to_owned(),
    };
    let lineage = [
        LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: "newer".to_owned(),
        },
        LibraryChangeCatchUpEvidence {
            source: "windows_usn_v1".to_owned(),
            watermark: "older".to_owned(),
        },
    ];
    let mut reads = PreparationCatalog::new(&repository);
    assert!(
        reads
            .load_incremental_location_by_file_identity(&identity, &lineage)
            .expect("original global lookup")
            .is_none()
    );

    assert!(reads.finish().still_matches(&repository, None));

    assert_eq!(
        *observed.borrow(),
        vec![
            (identity.clone(), lineage.to_vec()),
            (identity, lineage.to_vec())
        ]
    );
}

fn publish_live(catalog: &mut SqliteCatalog, root_id: &str, relative_path: &str) {
    catalog
        .enqueue_library_change_intents(
            &[intent(root_id, relative_path, None, 20)],
            1_000,
            policy(),
        )
        .expect("enqueue actual competing live intent");
    let report = process_ready_library_changes_in_lane(
        catalog,
        root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Live,
        2_000,
        policy(),
    )
    .expect("publish actual competing live delta");
    assert_eq!(report.completed_count, 1, "{report:?}");
}

fn run_journal(
    repository: &mut RevisionRacingCatalog,
    root_id: &str,
) -> crate::domain::IncrementalLibraryChangeReport {
    process_ready_library_changes_in_lane_cancellable(
        repository,
        root_id,
        LibraryRootGeneration::initial(),
        LibraryChangeLane::Journal,
        2_000,
        policy(),
        &AtomicBool::new(false),
    )
    .expect("process journal with revision conflict")
}

fn prepare_one(
    fixture: &mut CatalogFixture,
    relative_path: &str,
    previous: Option<&str>,
) -> (PreparedChange, PublicationGuardedFileDiscovery) {
    let mut change = intent(&fixture.root_id, relative_path, previous, 1);
    change.origin = LibraryChangeOrigin::ConsistencyAudit;
    fixture.enqueue(&[change]);
    let leased = fixture
        .catalog
        .lease_path_library_changes(
            &fixture.root_id,
            LibraryRootGeneration::initial(),
            2_000,
            policy(),
        )
        .expect("lease source proof work")
        .pop()
        .expect("source proof lease");
    let identity = FileDiscovery::new(&fixture.root_path)
        .expect("discovery")
        .metadata_inventory_root_identity()
        .expect("root identity")
        .expect("stable identity");
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &fixture.root_path,
        &identity,
    )
    .expect("publication namespace");
    let prepared = prepare_change(
        &mut PreparationCatalog::new(&fixture.catalog),
        &discovery,
        &LocalMediaInspector::new(),
        &leased,
    )
    .expect("prepare actual source state");
    (prepared, discovery)
}
