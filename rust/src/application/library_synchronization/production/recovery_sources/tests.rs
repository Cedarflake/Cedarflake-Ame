use super::super::runtime_test_support::*;
use super::super::*;
use super::*;

struct RetainedSourceProbe;

impl crate::ports::MetadataInventorySource for RetainedSourceProbe {
    fn next_page(
        &mut self,
        _max_entries: u32,
        _cancelled: &AtomicBool,
    ) -> Result<crate::domain::MetadataInventoryPage, ScanError> {
        Ok(crate::domain::MetadataInventoryPage {
            page_index: 0,
            entries: Vec::new(),
            cursor: None,
            is_complete: true,
            frontier: Vec::new(),
        })
    }
}

fn metadata_inventory_lease(change_id: LibraryChangeId) -> LeasedLibraryChange {
    let generation = LibraryRootGeneration::initial();
    let lease_generation = 1;
    let lease_expires_unix_ms = 10_000;
    LeasedLibraryChange {
        change: crate::domain::DurableLibraryChange {
            id: change_id,
            intent: crate::domain::LibraryChangeIntent {
                root_id: "root-a".to_owned(),
                root_generation: generation,
                kind: crate::domain::LibraryChangeIntentKind::FreshnessUnknown,
                scope: crate::domain::LibraryChangeScope::Root,
                relative_path: String::new(),
                previous_relative_path: None,
                origin: crate::domain::LibraryChangeOrigin::MetadataInventory,
                first_observed_unix_ms: 1,
                most_recent_observed_unix_ms: 1,
                first_sequence: 1,
                most_recent_sequence: 1,
                coalesced_observation_count: 1,
            },
            status: crate::domain::LibraryChangeQueueStatus::Leased,
            ready_unix_ms: 1,
            attempt_count: 1,
            next_retry_unix_ms: None,
            lease_generation,
            lease_expires_unix_ms: Some(lease_expires_unix_ms),
            last_failure: Some(crate::domain::LibraryChangeFailure {
                code: "metadata_inventory_required".to_owned(),
                message: "fixture metadata inventory authority".to_owned(),
            }),
            catalog_revision_at_enqueue: 0,
            catalog_revision_at_success: None,
            catch_up_source: None,
            catch_up_watermark: None,
            catch_up_lineage: Vec::new(),
            superseded_by_change_id: None,
        },
        lease_generation,
        lease_expires_unix_ms,
    }
}

fn retained_source_probe(run_id: &str) -> RetainedMetadataInventorySource {
    RetainedMetadataInventorySource::for_test(run_id, Box::new(RetainedSourceProbe))
}

#[test]
fn metadata_inventory_catalog_validation_failure_preserves_retained_change_owner() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
    drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    production.catalog_session = Some(Arc::new(
        SqliteCatalogSession::validate(catalog_path.clone()).expect("validated catalog session"),
    ));
    let change_id = LibraryChangeId::new(41).expect("fixture change ID");
    production.recovery_sources.publish(
        change_id,
        Some(retained_source_probe("retained-validation")),
    );

    let error = production
        .start_automatic_recovery(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            1,
            metadata_inventory_lease(change_id),
            1,
            directory.path().join("different-catalog.sqlite3"),
        )
        .expect_err("catalog switch must fail before source handoff");

    assert_eq!(error.code, "library_synchronization_catalog_changed");
    assert_eq!(
        production
            .recovery_sources
            .retained_for_test()
            .get(&change_id)
            .map(RetainedMetadataInventorySource::run_id),
        Some("retained-validation")
    );
    assert!(production.recovery.is_none());
}

#[test]
fn metadata_inventory_worker_spawn_failure_restores_retained_change_owner() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
    drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    let change_id = LibraryChangeId::new(42).expect("fixture change ID");
    production
        .recovery_sources
        .publish(change_id, Some(retained_source_probe("retained-spawn")));
    fail_next_recovery_worker_spawn();

    let error = production
        .start_automatic_recovery(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            1,
            metadata_inventory_lease(change_id),
            1,
            catalog_path,
        )
        .expect_err("injected spawn failure must restore source ownership");

    assert_eq!(error.code, "authoritative_recovery_worker_start_failed");
    assert_eq!(
        production
            .recovery_sources
            .retained_for_test()
            .get(&change_id)
            .map(RetainedMetadataInventorySource::run_id),
        Some("retained-spawn")
    );
    assert!(production.recovery.is_none());
}

#[test]
fn metadata_inventory_new_change_does_not_take_another_retained_owner() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
    drop(SqliteCatalog::open(catalog_path.clone()).expect("fixture catalog"));
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    let retained_change_id = LibraryChangeId::new(43).expect("retained change ID");
    let active_change_id = LibraryChangeId::new(44).expect("active change ID");
    production.recovery_sources.publish(
        retained_change_id,
        Some(retained_source_probe("retained-other-authority")),
    );

    production
        .start_automatic_recovery(
            "root-a".to_owned(),
            LibraryRootGeneration::initial(),
            1,
            metadata_inventory_lease(active_change_id),
            1,
            catalog_path,
        )
        .expect("start independent recovery owner");
    let deadline = Instant::now() + Duration::from_secs(5);
    while production.recovery.is_some() {
        production
            .poll_recovery(2)
            .expect("collect independent recovery result");
        assert!(
            Instant::now() < deadline,
            "independent recovery did not finish"
        );
        thread::yield_now();
    }

    assert_eq!(
        production
            .recovery_sources
            .retained_for_test()
            .get(&retained_change_id)
            .map(RetainedMetadataInventorySource::run_id),
        Some("retained-other-authority")
    );
    assert!(
        !production
            .recovery_sources
            .retained_for_test()
            .contains_key(&active_change_id)
    );
}

#[test]
fn metadata_inventory_source_pruning_discards_missing_authority() {
    let directory = tempfile::tempdir().expect("test directory");
    let catalog_path = directory.path().join("catalog").join("ame.sqlite3");
    let catalog = SqliteCatalog::open(catalog_path).expect("fixture catalog");
    let mut production = new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        test_live_only_connection(),
    );
    let change_id = LibraryChangeId::new(45).expect("stale change ID");
    production
        .recovery_sources
        .publish(change_id, Some(retained_source_probe("stale-authority")));

    production
        .recovery_sources
        .prune(&catalog)
        .expect("prune stale source owner");

    assert!(production.recovery_sources.retained_for_test().is_empty());
}
