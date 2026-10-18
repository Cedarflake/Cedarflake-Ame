use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, migrate_v29_ambiguous_gap_fixture, publication_identity,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[test]
fn current_v30_rejects_an_inexact_explicit_recovery_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-explicit-claim-root";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, "C:\\InvalidExplicitClaim", false);
    let catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET status = 'pending'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt explicit claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact explicit claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_an_inexact_active_foreground_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-active-claim-root";
    let root_path = "C:\\InvalidActiveClaim";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("invalid-active-claim-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('d'),
        )
        .expect("begin foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = 'wrong_foreground_state',
                 last_failure_message = 'wrong foreground state'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt active foreground claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact active foreground claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_an_active_foreground_claim_bound_to_authoritative_recovery() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-active-owner-root";
    let root_path = "C:\\Private\\InvalidActiveOwner";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("explicit recovery catalog");
    let request = fixture_request("invalid-active-owner-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('6'),
        )
        .expect("begin foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET scan_owner = 'authoritative_recovery' WHERE id = ?1",
            [&request.scan_id],
        )
        .expect("bind claim to wrong active owner");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(opened) => {
            drop(opened);
            None
        }
        Err(error) => Some(error),
    };
    let connection = Connection::open(catalog_path).expect("inspect rejected active owner");
    let retained: (
        String,
        Option<String>,
        String,
        Option<String>,
        String,
        String,
    ) = connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id, gap.status,
                    gap.authoritative_scan_id, scans.status, scans.scan_owner
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
             WHERE claim.root_id = ?1",
            [root_id],
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
        .expect("retained malformed active owner");
    assert_eq!(
        retained,
        (
            "foreground_scan".to_owned(),
            Some(request.scan_id.clone()),
            "leased".to_owned(),
            Some(request.scan_id),
            "running".to_owned(),
            "authoritative_recovery".to_owned(),
        ),
        "validation must not terminate the foreign-owned scan or unblock its gap"
    );
    let error = error.expect("foreign-owned active claim must fail closed");
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
    assert!(!error.message.contains(root_path));
}

#[test]
fn current_v30_rejects_an_inexact_consumed_foreground_claim_on_reopen() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-consumed-claim-root";
    let root_path = "C:\\InvalidConsumedClaim";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("migrate ambiguous v29 gap");
    let request = fixture_request("invalid-consumed-claim-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('e'),
        )
        .expect("begin foreground recovery");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove consumed recovery first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("publish foreground recovery");
    catalog
        .connection
        .execute(
            "UPDATE library_change_queue
             SET last_failure_code = 'wrong_consumed_state',
                 last_failure_message = 'wrong consumed state'
             WHERE id = (
               SELECT gap_change_id FROM library_live_gap_recovery_claims
               WHERE root_id = ?1
             )",
            [root_id],
        )
        .expect("corrupt consumed foreground claim state");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path) {
        Ok(_) => panic!("inexact consumed foreground claim must fail closed"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
}

#[test]
fn current_v30_rejects_a_consumed_foreground_claim_bound_to_authoritative_recovery() {
    let directory = tempdir().expect("catalog directory");
    let catalog_path = directory.path().join("catalog.sqlite3");
    let root_id = "invalid-consumed-owner-root";
    let root_path = "C:\\Private\\InvalidConsumedOwner";
    migrate_v29_ambiguous_gap_fixture(&catalog_path, root_id, root_path, false);
    let mut catalog = SqliteCatalog::open(catalog_path.clone()).expect("explicit recovery catalog");
    let request = fixture_request("invalid-consumed-owner-scan", root_path);
    catalog
        .begin_scan_with_publication_namespace(
            &request,
            root_id,
            root_path,
            &publication_identity('7'),
        )
        .expect("begin foreground recovery");
    catalog
        .prove_live_only_first_import_handoff_for_test(&request.scan_id)
        .expect("prove consumed recovery owner first-import handoff");
    catalog
        .publish_scan(&request.scan_id, root_id, 0, 0)
        .expect("consume foreground claim");
    catalog
        .connection
        .execute(
            "UPDATE scan_runs SET scan_owner = 'authoritative_recovery' WHERE id = ?1",
            [&request.scan_id],
        )
        .expect("bind consumed claim to wrong owner");
    drop(catalog);

    let error = match SqliteCatalog::open(catalog_path.clone()) {
        Ok(opened) => {
            drop(opened);
            None
        }
        Err(error) => Some(error),
    };
    let connection = Connection::open(catalog_path).expect("inspect rejected consumed owner");
    let retained: (String, String, i64, String, String, String) = connection
        .query_row(
            "SELECT claim.consumer_kind, claim.foreground_scan_id,
                    claim.consumed_unix_ms IS NOT NULL, gap.status,
                    scans.status, scans.scan_owner
             FROM library_live_gap_recovery_claims AS claim
             JOIN library_change_queue AS gap ON gap.id = claim.gap_change_id
             JOIN scan_runs AS scans ON scans.id = claim.foreground_scan_id
             WHERE claim.root_id = ?1",
            [root_id],
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
        .expect("retained malformed consumed owner");
    assert_eq!(
        retained,
        (
            "foreground_scan".to_owned(),
            request.scan_id,
            1,
            "completed".to_owned(),
            "completed".to_owned(),
            "authoritative_recovery".to_owned(),
        )
    );
    let error = error.expect("foreign-owned consumed claim must fail closed");
    assert_eq!(
        error.code,
        "catalog_live_gap_recovery_contract_unverifiable"
    );
    assert!(!error.message.contains(root_path));
}
