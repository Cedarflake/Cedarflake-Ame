use super::*;
use crate::domain::{LibraryChangeId, LibraryRecoveryAuthorityReason};

const ROOT: &str = "first-import-fallback";

fn published_import(path: std::path::PathBuf) -> (SqliteCatalog, PersistentJournalBaseline) {
    let mut catalog = SqliteCatalog::open(path).expect("catalog");
    let request = first_import_scan_request("first-import", ROOT);
    catalog
        .begin_scan(&request, ROOT, &request.root_path)
        .expect("begin import");
    let baseline = catalog
        .begin_persistent_journal_baseline(
            &first_import_baseline_request(&request, ROOT, 10),
            policy(),
        )
        .expect("opening boundary");
    catalog
        .publish_scan(&request.scan_id, ROOT, 0, 0)
        .expect("publish import");
    let baseline = catalog
        .capture_persistent_journal_baseline_closing_boundary(&closing_boundary(
            baseline.change_id,
            20,
        ))
        .expect("closing boundary");
    (catalog, baseline)
}

fn fallback_failure() -> PersistentJournalRootFailure {
    PersistentJournalRootFailure {
        root_id: ROOT.to_owned(),
        root_generation: LibraryRootGeneration::initial(),
        kind: PersistentJournalRootFailureKind::AutomaticMetadataFallback(
            LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure,
        ),
        failure: PersistentJournalFailure {
            code: "persistent_journal_read_stalled".to_owned(),
            message: "Journal replay did not advance".to_owned(),
        },
        opening_boundary: None,
    }
}

fn admit_fallback(catalog: &mut SqliteCatalog) -> LibraryChangeId {
    catalog
        .persist_persistent_journal_root_failure(&fallback_failure(), 3_000, policy())
        .expect("admit fallback")
        .expect("independent recovery")
}

fn import_control(catalog: &SqliteCatalog, change_id: LibraryChangeId) -> (String, Option<i64>) {
    catalog
        .connection
        .query_row(
            "SELECT control.status, authority.retired_unix_ms
             FROM library_change_queue AS control
             JOIN library_recovery_authorities AS authority ON authority.change_id = control.id
             WHERE control.id = ?1",
            [sqlite_integer(change_id.value(), "change ID").expect("change ID")],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("import execution state")
}

#[test]
fn fallback_retires_the_import_owner_without_claiming_journal_coverage() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut catalog, baseline) = published_import(path.clone());
    let recovery_id = admit_fallback(&mut catalog);
    assert_ne!(recovery_id, baseline.change_id);
    assert_eq!(
        import_control(&catalog, baseline.change_id),
        ("superseded".to_owned(), Some(3_000))
    );
    drop(catalog);

    let mut catalog = SqliteCatalog::open(path).expect("reopen retained failure evidence");
    let retained =
        super::super::baseline_state::load_baseline(&catalog.connection, baseline.change_id)
            .expect("retained window")
            .expect("failed window remains");
    assert_eq!(retained, baseline);
    assert!(
        catalog
            .load_persistent_journal_baselines()
            .expect("active baselines")
            .is_empty()
    );
    assert!(
        super::super::baseline_state::load_baseline_for_root(
            &catalog.connection,
            ROOT,
            LibraryRootGeneration::initial(),
        )
        .expect("active root baseline")
        .is_none()
    );
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(ROOT, LibraryRootGeneration::initial())
        .expect("checkpoint")
        .expect("retained cursor");
    assert_eq!(checkpoint.next_unread_usn, JournalUsn::new(10).unwrap());
    assert_eq!(
        checkpoint.continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    let authority = catalog
        .load_metadata_inventory_recovery_authority(recovery_id)
        .expect("recovery authority")
        .expect("independent recovery remains active");
    assert_eq!(
        authority.reason,
        LibraryRecoveryAuthorityReason::BrokerAfterCurrentFailure
    );
    catalog
        .begin_metadata_inventory(&MetadataInventoryRunRequest {
            run_id: authority.run_id,
            root_id: ROOT.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 3_001,
        })
        .expect("fallback starts without the superseded import authority conflict");
    assert!(
        !catalog
            .finalize_ready_first_import_journal_baseline(3_002)
            .expect("failed journal never completes the old import boundary")
    );
}

#[test]
fn restart_handoffs_a_retained_import_even_while_fallback_is_pending() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("catalog.sqlite3");
    let (mut catalog, baseline) = published_import(path.clone());
    let recovery_id = admit_fallback(&mut catalog);
    // Recreate the earlier persisted state: both execution owners remained active.
    catalog
        .connection
        .execute(
            "UPDATE library_recovery_authorities SET retired_unix_ms = NULL WHERE change_id = ?1",
            [sqlite_integer(baseline.change_id.value(), "change ID").unwrap()],
        )
        .expect("retain old import authority");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET status = 'retry_wait' WHERE id = ?1",
            [sqlite_integer(baseline.change_id.value(), "change ID").unwrap()],
        )
        .expect("retain old pending control");
    drop(catalog);
    let mut catalog = SqliteCatalog::open(path.clone()).expect("reopen previous-version state");
    assert!(
        !catalog
            .resume_journal_after_metadata_fallback(ROOT, LibraryRootGeneration::initial(), 4_000)
            .expect("fallback still owns this recovery")
    );
    drop(catalog);
    let catalog = SqliteCatalog::open(path).expect("reopen committed handoff");
    assert_eq!(
        import_control(&catalog, baseline.change_id),
        ("superseded".to_owned(), Some(4_000))
    );
    assert_eq!(
        import_control(&catalog, recovery_id),
        ("pending".to_owned(), None)
    );
}

#[test]
fn fallback_admission_rolls_back_when_import_retirement_fails() {
    let directory = tempdir().expect("directory");
    let (mut catalog, baseline) = published_import(directory.path().join("catalog.sqlite3"));
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER reject_import_retirement
         BEFORE UPDATE OF retired_unix_ms ON library_recovery_authorities
         WHEN OLD.reason = 'first_import_boundary' AND NEW.retired_unix_ms IS NOT NULL
         BEGIN SELECT RAISE(ABORT, 'controlled handoff failure'); END;",
        )
        .expect("inject failure");
    catalog
        .persist_persistent_journal_root_failure(&fallback_failure(), 3_000, policy())
        .expect_err("handoff failure aborts the complete admission");
    assert_eq!(
        import_control(&catalog, baseline.change_id),
        ("pending".to_owned(), None)
    );
    let counts: (i64, i64) = catalog
        .connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM library_metadata_fallbacks),
                (SELECT COUNT(*) FROM library_recovery_authorities)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("atomic rollback");
    assert_eq!(counts, (0, 1));
    let checkpoint = catalog
        .load_persistent_journal_checkpoint(ROOT, LibraryRootGeneration::initial())
        .expect("checkpoint")
        .expect("original cursor");
    assert_ne!(
        checkpoint.continuity,
        PersistentJournalContinuityState::LiveOnly
    );
    assert!(checkpoint.failure.is_none());
}

#[test]
fn an_unfinished_import_without_fallback_keeps_its_authority() {
    let directory = tempdir().expect("directory");
    let (mut catalog, baseline) = published_import(directory.path().join("catalog.sqlite3"));
    assert!(
        !catalog
            .resume_journal_after_metadata_fallback(ROOT, LibraryRootGeneration::initial(), 3_000)
            .expect("ordinary first-import replay")
    );
    assert_eq!(
        import_control(&catalog, baseline.change_id),
        ("pending".to_owned(), None)
    );
    assert_eq!(
        catalog
            .load_persistent_journal_baselines()
            .expect("active replay"),
        vec![baseline]
    );
}
