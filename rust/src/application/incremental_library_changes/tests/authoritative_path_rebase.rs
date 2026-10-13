use super::*;

#[test]
fn authoritative_path_rebases_an_unrelated_publication_without_retry_wait() {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    let target = fixture.source.path().join("visible.png");
    write_png(&target, 2, 2, [10, 20, 30]);
    let original = fs::read(&target).expect("source bytes");
    let mut change = intent(&fixture.root_id, "visible.png", None, 1);
    change.origin = LibraryChangeOrigin::ConsistencyAudit;
    fixture.enqueue(&[change]);
    let races = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let root = fixture
        .catalog
        .load_incremental_catalog_root(&fixture.root_id)
        .expect("root query")
        .expect("published root");
    let leased = fixture
        .catalog
        .lease_path_library_changes(&root.root_id, root.root_generation, 2_000, policy())
        .expect("lease visible path")
        .remove(0);
    let identity = root
        .publication_root_identity
        .as_ref()
        .expect("root identity");
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        identity,
    )
    .expect("held root namespace");
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, races);
    reset_source_content_open_instrumentation(&root.root_path);
    let counts = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = std::rc::Rc::clone(&counts);
    let root_path = root.root_path.clone();
    repository.on_publication = Some(Box::new(move |_, _| {
        observed
            .borrow_mut()
            .push(source_content_open_count(&root_path));
    }));

    let report = super::super::process_authoritative_path_set(
        &mut repository,
        super::super::AuthoritativePathSetContext {
            root_id: &root.root_id,
            root_generation: root.root_generation,
            expected_catalog_revision: root.catalog_revision,
            expected_root_identity: identity,
            leased: &leased,
            relative_paths: &["visible.png".to_owned()],
            now_unix_ms: 2_000,
            queue_policy: policy(),
            cancellation: &AtomicBool::new(false),
        },
        &discovery,
    )
    .expect("publish visible source despite unrelated catalog publication");

    assert_eq!(report.completed_count, 1, "{report:?}");
    assert_eq!(report.retried_count, 0, "{report:?}");
    assert_eq!(repository.publication_attempts, 2);
    assert!(counts.borrow()[0] > 0);
    assert_eq!(counts.borrow()[0], counts.borrow()[1]);
    let location = repository
        .catalog
        .load_incremental_location_by_relative_path(&root.root_id, "visible.png")
        .expect("published source")
        .expect("visible source is present");
    assert_eq!(location.width, 2);
    assert_eq!(
        fs::read(&target).expect("source after publication"),
        original
    );
    let pending = repository
        .catalog
        .load_library_change_root_queue_metrics(
            &root.root_id,
            root.root_generation,
            2_000,
            policy(),
        )
        .expect("queue metrics");
    assert_eq!(pending.retry_wait_count, 0);
}

#[test]
fn authoritative_path_rebase_keeps_its_two_attempt_limit_and_one_durable_owner() {
    for races in [2, 3] {
        let mut fixture = path_fixture();
        let competing =
            prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), races);
        let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);

        let report = run_path_set(
            &mut repository,
            &fixture.root_id,
            &["visible.png".to_owned()],
            &AtomicBool::new(false),
        );

        assert_eq!(repository.publication_attempts, 3);
        let connection = Connection::open(fixture._storage.path().join("catalog.sqlite3"))
            .expect("catalog queue");
        let (count, status, attempts): (i64, String, i64) = connection
            .query_row(
                "SELECT COUNT(*), status, attempt_count FROM library_change_queue
                 WHERE root_id = ?1",
                [&fixture.root_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("single original owner");
        assert_eq!(count, 1);
        assert_eq!(attempts, 1);
        if races == 2 {
            assert_eq!(report.completed_count, 1);
            assert_eq!(report.retried_count, 0);
            assert_eq!(status, "completed");
        } else {
            assert_eq!(report.completed_count, 0);
            assert_eq!(report.retried_count, 1);
            assert_eq!(status, "retry_wait");
        }
    }
}

#[test]
fn authoritative_path_rebase_rejects_changed_source_evidence() {
    let mut fixture = path_fixture();
    let competing = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);
    repository.rewrite_on_first_publication = Some(fixture.source.path().join("visible.png"));

    let report = run_path_set(
        &mut repository,
        &fixture.root_id,
        &["visible.png".to_owned()],
        &AtomicBool::new(false),
    );

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(report.retried_count, 1);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "visible.png")
            .expect("retained catalog")
            .is_none()
    );
}

#[test]
fn authoritative_path_rebase_cancellation_returns_the_same_lease_without_retry() {
    let mut fixture = path_fixture();
    let competing = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);
    let cancelled = Arc::new(AtomicBool::new(false));
    repository.cancel_after_first_publication = Some(Arc::clone(&cancelled));

    let report = run_path_set(
        &mut repository,
        &fixture.root_id,
        &["visible.png".to_owned()],
        &cancelled,
    );

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(report.retried_count, 0);
    assert_eq!(report.deferred_count, 1);
    let connection =
        Connection::open(fixture._storage.path().join("catalog.sqlite3")).expect("catalog queue");
    let (status, attempts): (String, i64) = connection
        .query_row(
            "SELECT status, attempt_count FROM library_change_queue",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("returned original lease");
    assert_eq!(status, "pending");
    assert_eq!(attempts, 0);
}

#[test]
fn authoritative_path_set_keeps_broader_scope_on_its_fixed_revision() {
    for scope in [LibraryChangeScope::Root, LibraryChangeScope::Subtree] {
        let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
        let album = fixture.source.path().join("album");
        fs::create_dir(&album).expect("owned subtree");
        write_png(&album.join("visible.png"), 2, 2, [10, 20, 30]);
        let relative_path = match scope {
            LibraryChangeScope::Root => "",
            LibraryChangeScope::Subtree => "album",
            LibraryChangeScope::Path => unreachable!(),
        };
        let mut change = intent(&fixture.root_id, relative_path, None, 1);
        change.scope = scope;
        change.kind = LibraryChangeIntentKind::Reconcile;
        change.origin = LibraryChangeOrigin::ConsistencyAudit;
        fixture.enqueue(&[change]);
        let competing = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
        let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);

        let report = run_path_set(
            &mut repository,
            &fixture.root_id,
            &["album/visible.png".to_owned()],
            &AtomicBool::new(false),
        );

        assert_eq!(repository.publication_attempts, 1);
        assert_eq!(report.completed_count, 0);
        assert_eq!(report.retried_count, 1);
    }
}

#[test]
fn authoritative_path_set_rejects_rebase_for_extra_or_mismatching_paths() {
    for paths in [vec!["visible.png", "other.png"], vec!["other.png"]] {
        let mut fixture = path_fixture();
        write_png(&fixture.source.path().join("other.png"), 3, 3, [50, 60, 70]);
        let competing = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
        let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);

        let report = run_path_set(
            &mut repository,
            &fixture.root_id,
            &paths.into_iter().map(str::to_owned).collect::<Vec<_>>(),
            &AtomicBool::new(false),
        );

        assert_eq!(repository.publication_attempts, 1);
        assert_eq!(report.completed_count, 0);
        assert_eq!(report.retried_count, 1);
    }
}

fn path_fixture() -> CatalogFixture {
    let mut fixture = seed_catalog(tempdir().expect("source"), &[]);
    write_png(
        &fixture.source.path().join("visible.png"),
        2,
        2,
        [10, 20, 30],
    );
    let mut change = intent(&fixture.root_id, "visible.png", None, 1);
    change.origin = LibraryChangeOrigin::ConsistencyAudit;
    fixture.enqueue(&[change]);
    fixture
}

#[test]
fn authoritative_path_rebase_retains_new_global_identity_evidence_for_retry() {
    let fixture = path_fixture();
    let aliases = tempdir().expect("alias source");
    fs::hard_link(
        fixture.source.path().join("visible.png"),
        aliases.path().join("alias.png"),
    )
    .expect("same-volume physical identity");
    let alias_path = aliases.path().to_path_buf();
    let mut repository = RevisionRacingCatalog::new(fixture.catalog, Vec::new());
    repository.on_publication = Some(Box::new(move |catalog, attempt| {
        if attempt == 1 {
            seed_root(
                catalog,
                "peer-root",
                "peer-scan",
                &alias_path,
                &["alias.png"],
            );
        }
    }));

    let report = run_path_set(
        &mut repository,
        &fixture.root_id,
        &["visible.png".to_owned()],
        &AtomicBool::new(false),
    );

    assert_eq!(repository.publication_attempts, 1);
    assert_eq!(report.completed_count, 0);
    assert_eq!(report.retried_count, 1);
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path(&fixture.root_id, "visible.png")
            .expect("old preparation was not published")
            .is_none()
    );
    assert!(
        repository
            .catalog
            .load_incremental_location_by_relative_path("peer-root", "alias.png")
            .expect("new identity remains published")
            .is_some()
    );
}

#[test]
fn authoritative_path_rebase_cannot_retry_or_defer_a_replacement_lease() {
    for cancel in [false, true] {
        let mut fixture = path_fixture();
        let competing = prepare_revision_races(&mut fixture.catalog, fixture._storage.path(), 1);
        let mut repository = RevisionRacingCatalog::new(fixture.catalog, competing);
        let replacement = std::rc::Rc::new(std::cell::RefCell::new(None));
        let observed = std::rc::Rc::clone(&replacement);
        let root_id = fixture.root_id.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        if cancel {
            repository.cancel_after_first_publication = Some(Arc::clone(&cancelled));
        }
        repository.on_publication = Some(Box::new(move |catalog, attempt| {
            let replaces_now = matches!((cancel, attempt), (true, 1) | (false, 2));
            if !replaces_now {
                return;
            }
            let mut newer = intent(&root_id, "visible.png", None, 2);
            newer.origin = LibraryChangeOrigin::ConsistencyAudit;
            catalog
                .enqueue_library_change_intents(&[newer], 2_001, policy())
                .expect("superseding path evidence");
            let mut leases = catalog
                .lease_path_library_changes(
                    &root_id,
                    LibraryRootGeneration::initial(),
                    2_001,
                    policy(),
                )
                .expect("replacement lease");
            assert_eq!(leases.len(), 1);
            *observed.borrow_mut() = Some(leases.remove(0));
        }));

        let report = run_path_set(
            &mut repository,
            &fixture.root_id,
            &["visible.png".to_owned()],
            &cancelled,
        );

        assert_eq!(report.completed_count, 0);
        assert_eq!(report.retried_count, 0);
        assert_eq!(report.deferred_count, 0);
        assert_eq!(report.superseded_count, 1);
        let replacement = replacement.borrow();
        let lease = replacement.as_ref().expect("new executor owns the path");
        let connection = Connection::open(fixture._storage.path().join("catalog.sqlite3"))
            .expect("catalog queue");
        let (status, generation, attempts): (String, i64, i64) = connection.query_row(
            "SELECT status, lease_generation, attempt_count FROM library_change_queue WHERE id = ?1",
            [i64::try_from(lease.change.id.value()).expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).expect("replacement authority after old completion");
        assert_eq!(status, "leased");
        assert_eq!(
            generation,
            i64::try_from(lease.lease_generation).expect("lease generation")
        );
        assert_eq!(attempts, i64::from(lease.change.attempt_count));
    }
}

fn run_path_set(
    repository: &mut RevisionRacingCatalog,
    root_id: &str,
    paths: &[String],
    cancelled: &AtomicBool,
) -> crate::domain::IncrementalLibraryChangeReport {
    let root = repository
        .catalog
        .load_incremental_catalog_root(root_id)
        .expect("root")
        .expect("published root");
    let mut leases = repository
        .catalog
        .lease_library_changes(root_id, root.root_generation, 2_000, policy())
        .expect("lease owned work");
    assert_eq!(leases.len(), 1);
    let leased = leases.remove(0);
    let identity = root
        .publication_root_identity
        .as_ref()
        .expect("root identity");
    let discovery = PublicationGuardedFileDiscovery::new_incremental_publication_guard(
        &root.root_path,
        identity,
    )
    .expect("held root namespace");
    super::super::process_authoritative_path_set(
        repository,
        super::super::AuthoritativePathSetContext {
            root_id,
            root_generation: root.root_generation,
            expected_catalog_revision: root.catalog_revision,
            expected_root_identity: identity,
            leased: &leased,
            relative_paths: paths,
            now_unix_ms: 2_000,
            queue_policy: policy(),
            cancellation: cancelled,
        },
        &discovery,
    )
    .expect("guarded path publication")
}
