use super::*;

#[test]
fn root_authority_repair_restores_exact_legacy_removal_and_is_idempotent() {
    let mut fixture = Fixture::new(&["removed", "retained"]);
    let retained = fixture.authorize("retained", "retained-inventory", 2_000);
    fixture.advance("retained-inventory", "retained", InventoryPhase::Running);
    let retained_before = fixture.authority(retained.change.id);
    let removed_id = fixture.emulate_legacy_unregistration();
    let before = fixture.authority(removed_id);
    assert_eq!(eligible_repairs(&fixture), 1);

    fixture.reopen();

    let repaired = fixture.authority(removed_id);
    let retired_at = repaired.retired_unix_ms.expect("legacy authority retired");
    assert!(retired_at >= before.authorized_unix_ms);
    assert_eq!(
        repaired,
        LibraryRecoveryAuthority {
            retired_unix_ms: Some(retired_at),
            ..before
        }
    );
    assert_eq!(fixture.queue_status(removed_id), "superseded");
    assert_eq!(fixture.authority(retained.change.id), retained_before);
    assert_eq!(fixture.queue_status(retained.change.id), "leased");
    assert!(
        fixture
            .catalog
            .load_incremental_catalog_root("removed")
            .expect("removed root")
            .is_none()
    );
    assert!(
        fixture
            .catalog
            .load_metadata_inventory_run("legacy-removed-inventory")
            .expect("removed inventory")
            .is_none()
    );
    assert_eq!(eligible_repairs(&fixture), 0);
    fixture.reopen();
    assert_eq!(fixture.authority(removed_id), repaired);
    let version: i64 = fixture
        .catalog
        .connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("unchanged schema version");
    assert_eq!(
        version, 31,
        "compatibility repair does not conceal corruption with a version bump"
    );
}

#[test]
fn root_authority_repair_rejects_incomplete_retirement_evidence() {
    let counterexamples = [
        (
            "registered root",
            "INSERT INTO library_roots(id, path, created_unix_ms)
             VALUES ('removed', 'controlled-reintroduced-root', 1)",
        ),
        (
            "active generation",
            "UPDATE library_change_root_state SET is_active = 1
             WHERE root_id = 'removed'",
        ),
        (
            "different generation",
            "UPDATE library_change_root_state SET generation = 2
             WHERE root_id = 'removed'",
        ),
        (
            "completed instead of superseded queue",
            "UPDATE library_change_queue
             SET status = 'completed', catalog_revision_at_success = 0
             WHERE root_id = 'removed'",
        ),
        (
            "different queue root",
            "UPDATE library_change_queue SET root_id = 'retained'
             WHERE root_id = 'removed'",
        ),
        (
            "different queue generation",
            "UPDATE library_change_queue SET root_generation = 2
             WHERE root_id = 'removed'",
        ),
        (
            "non-P2 queue",
            "UPDATE library_change_queue SET origin = 'startup_catch_up'
             WHERE root_id = 'removed'",
        ),
        (
            "run identity still exists",
            "INSERT INTO library_metadata_inventory_runs(
             id, root_id, root_generation, epoch, scope_kind, scope_relative_path,
             status, next_page_index, started_unix_ms, updated_unix_ms
           ) VALUES ('legacy-removed-inventory', 'retained', 1, 1, 'root', '',
             'failed', 1, 2000, 2100)",
        ),
    ];
    for (description, mutation) in counterexamples {
        let mut fixture = Fixture::new(&["removed", "retained"]);
        let change_id = fixture.emulate_legacy_unregistration();
        fixture
            .catalog
            .connection
            .execute_batch(mutation)
            .expect(description);
        assert_eq!(
            eligible_repairs(&fixture),
            0,
            "missing proof: {description}"
        );

        let error = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
            .err()
            .expect("incomplete authority proof must still fail full startup validation");

        assert!(
            error.code.starts_with("catalog_") && error.code.contains("contract"),
            "{description}: unexpected non-contract failure: {error:?}"
        );
        assert_eq!(
            fixture.authority(change_id).retired_unix_ms,
            None,
            "compatibility repair cannot infer authority retirement from {description}"
        );
    }
}

#[test]
fn root_authority_repair_rejects_damaged_schema_before_interpreting_rows() {
    let mut fixture = Fixture::new(&["removed"]);
    let change_id = fixture.emulate_legacy_unregistration();
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER library_recovery_authority_update_guard;")
        .expect("damage the immutable authority identity guard");

    let error = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .err()
        .expect("damaged authority structure must fail closed");

    assert_eq!(
        error.code,
        "catalog_recovery_authority_contract_unverifiable"
    );
    assert_eq!(fixture.authority(change_id).retired_unix_ms, None);
}

#[test]
fn root_authority_repair_rolls_back_when_an_unrelated_row_contract_fails() {
    let mut fixture = Fixture::new(&["removed", "retained"]);
    let retained = fixture.authorize("retained", "retained-inventory", 2_000);
    fixture.advance("retained-inventory", "retained", InventoryPhase::Running);
    let retained_authority = fixture.authority(retained.change.id);
    let removed_id = fixture.emulate_legacy_unregistration();
    fixture
        .catalog
        .connection
        .execute(
            "UPDATE library_metadata_inventory_runs SET staged_entry_count = 1
         WHERE id = 'retained-inventory'",
            [],
        )
        .expect("inject unrelated derived count corruption");
    assert_eq!(eligible_repairs(&fixture), 1);

    let error = SqliteCatalog::open(fixture.catalog.catalog_path().to_path_buf())
        .err()
        .expect("full row validation must reject unrelated damage");

    assert_eq!(
        error.code,
        "catalog_metadata_inventory_contract_unverifiable"
    );
    assert_eq!(
        fixture.authority(removed_id).retired_unix_ms,
        None,
        "full-validator failure rolls back the otherwise eligible repair"
    );
    assert_eq!(fixture.authority(retained.change.id), retained_authority);
    fixture
        .catalog
        .connection
        .execute(
            "UPDATE library_metadata_inventory_runs SET staged_entry_count = 0
         WHERE id = 'retained-inventory'",
            [],
        )
        .expect("restore exactly the injected count");
    fixture.reopen();
    assert!(fixture.authority(removed_id).retired_unix_ms.is_some());
}

#[test]
fn root_authority_repair_sql_failure_rolls_back_all_eligible_retirements() {
    let mut fixture = Fixture::new(&["removed", "second"]);
    let first = fixture.authorize("removed", "first-inventory", 2_000);
    fixture.advance("first-inventory", "removed", InventoryPhase::Comparing);
    let second = fixture.authorize("second", "second-inventory", 2_000);
    fixture.advance("second-inventory", "second", InventoryPhase::Running);
    fixture.reopen();
    assert!(
        fixture
            .catalog
            .unregister_root("removed")
            .expect("remove first root")
    );
    assert!(
        fixture
            .catalog
            .unregister_root("second")
            .expect("remove second root")
    );
    fixture.reopen();
    let removed_id = first.change.id;
    fixture.catalog.connection.execute(
        "UPDATE library_recovery_authorities SET retired_unix_ms = NULL WHERE change_id IN (?1, ?2)",
        params![i64::try_from(removed_id.value()).expect("first SQLite change id"),
            i64::try_from(second.change.id.value()).expect("second SQLite change id")],
    ).expect("reconstruct both old omitted retirements after the valid removal workflows");
    assert_eq!(eligible_repairs(&fixture), 2);
    fixture
        .catalog
        .connection
        .execute_batch(
            "CREATE TEMP TABLE retirement_attempts(change_id INTEGER);
         CREATE TEMP TRIGGER abort_second_authority_repair
         BEFORE UPDATE OF retired_unix_ms ON library_recovery_authorities
         WHEN NEW.retired_unix_ms IS NOT NULL
         BEGIN
           INSERT INTO retirement_attempts VALUES (NEW.change_id);
           SELECT CASE WHEN (SELECT COUNT(*) FROM retirement_attempts) = 2
             THEN RAISE(ABORT, 'injected second authority repair failure') END;
         END;",
        )
        .expect("inject second update failure in actual migration transaction");

    let error = crate::adapters::sqlite_catalog::migrations::migrate_schema(
        &mut fixture.catalog.connection,
    )
    .expect_err("second retirement must abort the repair transaction");

    assert_eq!(error.code, "catalog_database_error");
    assert!(
        error
            .message
            .contains("injected second authority repair failure")
    );
    assert!(fixture.catalog.connection.is_autocommit());
    assert_eq!(fixture.authority(removed_id).retired_unix_ms, None);
    assert_eq!(fixture.authority(second.change.id).retired_unix_ms, None);
    fixture
        .catalog
        .connection
        .execute_batch(
            "DROP TRIGGER abort_second_authority_repair; DROP TABLE retirement_attempts;",
        )
        .expect("remove controlled failure injection");
    fixture.reopen();
    assert!(fixture.authority(removed_id).retired_unix_ms.is_some());
    assert!(
        fixture
            .authority(second.change.id)
            .retired_unix_ms
            .is_some()
    );
}

fn eligible_repairs(fixture: &Fixture) -> i64 {
    fixture
        .catalog
        .connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM ({})",
                change_queue::root_retirement::REMOVED_ROOT_AUTHORITY_IDS_SQL
            ),
            [],
            |row| row.get(0),
        )
        .expect("exact durable retirement proof")
}
