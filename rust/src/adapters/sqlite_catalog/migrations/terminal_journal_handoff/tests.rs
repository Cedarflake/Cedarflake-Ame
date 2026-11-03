use crate::domain::JournalScopeRecoveryOutcome;

use super::super::classification_merge::downgrade_to_v72_for_test;
use super::super::current_schema::validate_current_schema_contract;

mod fixture;
use fixture::{RecoveryFixture, active_sibling, handoffs};

#[test]
fn scoped_recovery_completion_releases_handoffs_before_reopen() {
    let mut fixture = RecoveryFixture::new();
    assert_eq!(
        fixture.complete().unwrap(),
        Some(JournalScopeRecoveryOutcome::Completed)
    );
    assert_eq!(handoffs(&fixture.catalog.connection), 0);
    fixture.reopen().expect("completed recovery reopens");
}

#[test]
fn scoped_recovery_completion_retains_another_active_owner() {
    let mut fixture = RecoveryFixture::new();
    active_sibling(&mut fixture);
    assert_eq!(
        fixture.complete().unwrap(),
        Some(JournalScopeRecoveryOutcome::Completed)
    );
    assert_eq!(handoffs(&fixture.catalog.connection), 1);
    fixture.reopen().expect("shared handoff reopens");
    assert_eq!(handoffs(&fixture.catalog.connection), 1);
}

#[test]
fn scoped_recovery_completion_rolls_back_if_handoff_release_fails() {
    let mut fixture = RecoveryFixture::new();
    fixture.catalog.connection.execute_batch(
        "CREATE TRIGGER reject_handoff_release BEFORE DELETE ON library_change_catch_up_handoffs
         BEGIN SELECT RAISE(ABORT, 'fixture rejection'); END;",
    ).unwrap();
    fixture
        .complete()
        .expect_err("cleanup failure rolls back completion");
    assert_eq!(handoffs(&fixture.catalog.connection), 1);
    let status: String = fixture
        .catalog
        .connection
        .query_row(
            "SELECT status FROM library_change_queue WHERE id = ?1",
            [i64::try_from(fixture.parent.value()).unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "leased");
}

#[test]
fn retained_scoped_recovery_handoff_is_repaired_idempotently() {
    let mut fixture = RecoveryFixture::new();
    fixture.retain_legacy_completion();
    assert_eq!(
        validate_current_schema_contract(&fixture.catalog.connection)
            .unwrap_err()
            .code,
        "catalog_change_catch_up_contract_unverifiable"
    );
    fixture.reopen().expect("recover proven completed owner");
    assert_eq!(handoffs(&fixture.catalog.connection), 0);
    fixture.reopen().expect("recovery is idempotent");
    assert_eq!(handoffs(&fixture.catalog.connection), 0);
}

#[test]
fn retained_scoped_recovery_is_repaired_before_schema_upgrade() {
    let mut fixture = RecoveryFixture::new();
    fixture.retain_legacy_completion();
    downgrade_to_v72_for_test(&fixture.catalog.connection);
    fixture
        .reopen()
        .expect("repair precedes upgrade validation");
    assert_eq!(handoffs(&fixture.catalog.connection), 0);
    validate_current_schema_contract(&fixture.catalog.connection).unwrap();
}

#[test]
fn retained_scoped_recovery_does_not_release_active_shared_evidence() {
    let mut fixture = RecoveryFixture::new();
    active_sibling(&mut fixture);
    fixture.retain_legacy_completion();
    fixture.reopen().expect("active sibling retains evidence");
    assert_eq!(handoffs(&fixture.catalog.connection), 1);
}

#[test]
fn unknown_handoff_rolls_back_the_known_recovery_repair() {
    let mut fixture = RecoveryFixture::new();
    fixture.retain_legacy_completion();
    fixture.add_handoff("unproven-range", "unproven-asset");
    let error = fixture
        .reopen()
        .expect_err("unproven handoff remains rejected");
    assert_eq!(error.code, "catalog_change_catch_up_contract_unverifiable");
    assert_eq!(handoffs(&fixture.catalog.connection), 2);
}

#[test]
fn mismatched_recovery_child_cannot_authorize_handoff_release() {
    let mut fixture = RecoveryFixture::new();
    fixture.retain_legacy_completion();
    fixture
        .catalog
        .connection
        .execute(
            "UPDATE library_change_queue SET relative_path = 'unrelated'
         WHERE id IN (SELECT current_change_id FROM library_journal_scope_recoveries)",
            [],
        )
        .unwrap();
    assert_eq!(
        fixture.reopen().unwrap_err().code,
        "catalog_journal_scope_recovery_invalid"
    );
    assert_eq!(handoffs(&fixture.catalog.connection), 1);
}
