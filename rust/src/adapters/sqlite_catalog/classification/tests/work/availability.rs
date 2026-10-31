use crate::domain::LibraryRootGeneration;
use crate::ports::CatalogRepository;
use crate::ports::classification_availability::ClassificationAvailabilityRepository;

use super::*;

mod roots;

fn root(id: &str) -> ClassificationRecoveryRoot {
    ClassificationRecoveryRoot {
        id: id.into(),
        generation: LibraryRootGeneration::initial(),
    }
}

fn unavailable(catalog: &mut SqliteCatalog, lease: &ClassificationWorkLease) {
    assert!(
        catalog
            .fail_classification_work(
                lease,
                &ClassificationWorkIssue {
                    kind: ClassificationWorkFailure::Unavailable,
                    code: "file_unavailable".into(),
                    retry_unix_ms: 5000,
                }
            )
            .unwrap()
    );
}

fn claim(catalog: &mut SqliteCatalog, session: &ClassificationRecordId) -> ClassificationWorkLease {
    catalog
        .claim_classification_work(session, 10)
        .unwrap()
        .unwrap()
}

fn content_task(
    catalog: &mut SqliteCatalog,
    session: &ClassificationRecordId,
) -> ClassificationWorkLease {
    verify(catalog, "a", 'a');
    request(
        catalog,
        session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let source = claim(catalog, session);
    assert!(catalog.finish_classification_verification(&source).unwrap());
    claim(catalog, session)
}

#[test]
fn unavailable_copy_yields_to_a_peer_without_using_the_transient_retry_budget() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    verify(&mut catalog, "c", 'a');
    let first = content_task(&mut catalog, &session);
    catalog
        .connection
        .execute(
            "UPDATE classification_work_items SET failure_count = 2 WHERE work_id = ?1",
            [i64::try_from(first.work_id).unwrap()],
        )
        .unwrap();
    unavailable(&mut catalog, &first);
    assert!(
        !catalog
            .publish_classification_work(&first, &evidence(&run, &first))
            .unwrap()
    );
    let peer = claim(&mut catalog, &session);
    assert_eq!(peer.work_id, first.work_id);
    assert!(
        matches!(&peer.target, ClassificationWorkTarget::AnalyzeContent { source, .. } if source.location_id == "c")
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT failure_count FROM classification_work_items WHERE kind = 'content'"
        ),
        2
    );
    assert!(
        catalog
            .publish_classification_work(&peer, &evidence(&run, &peer))
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        0
    );
}

#[test]
fn a_late_verified_copy_resumes_the_waiting_content_once() {
    let (_directory, mut catalog) = fixture();
    let (run, session) = start(&mut catalog);
    let first = content_task(&mut catalog, &session);
    unavailable(&mut catalog, &first);
    assert!(
        catalog
            .claim_classification_work(&session, 100_000)
            .unwrap()
            .is_none()
    );
    verify(&mut catalog, "c", 'a');
    request(
        &mut catalog,
        &session,
        &["c"],
        ClassificationWorkPriority::NewContent,
    );
    let new_source = claim(&mut catalog, &session);
    assert!(
        catalog
            .finish_classification_verification(&new_source)
            .unwrap()
    );
    let resumed = claim(&mut catalog, &session);
    assert_eq!(resumed.work_id, first.work_id);
    assert!(
        matches!(&resumed.target, ClassificationWorkTarget::AnalyzeContent { source, .. } if source.location_id == "c")
    );
    assert!(
        catalog
            .publish_classification_work(&resumed, &evidence(&run, &resumed))
            .unwrap()
    );
    assert!(
        !catalog
            .finish_classification_verification(&new_source)
            .unwrap()
    );
    assert_eq!(
        scalar(&catalog, "SELECT count(*) FROM classification_predictions"),
        1
    );
}

#[test]
fn source_availability_refresh_is_scoped_guarded_and_transactional() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a", "b"],
        ClassificationWorkPriority::Requested,
    );
    let a = claim(&mut catalog, &session);
    unavailable(&mut catalog, &a);
    let b = claim(&mut catalog, &session);
    unavailable(&mut catalog, &b);
    let selected_a = selected(&catalog, "a");
    let mut stale_b = selected(&catalog, "b");
    stale_b.source_generation += 100;
    assert!(
        catalog
            .refresh_classification_sources(&session, &[selected_a.clone(), stale_b], 10)
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        2
    );
    catalog.connection.execute_batch(
        "CREATE TRIGGER reject_source_resume BEFORE UPDATE OF phase ON classification_work_items
         WHEN NEW.phase = 'queued' BEGIN SELECT RAISE(ABORT, 'injected'); END;",
    ).unwrap();
    assert!(
        catalog
            .refresh_classification_sources(&session, std::slice::from_ref(&selected_a), 10)
            .is_err()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        2
    );
    catalog
        .connection
        .execute_batch("DROP TRIGGER reject_source_resume")
        .unwrap();
    assert_eq!(
        catalog
            .refresh_classification_sources(&session, &[selected_a], 10)
            .unwrap(),
        1
    );
    let resumed = claim(&mut catalog, &session);
    assert_eq!(resumed.work_id, a.work_id);
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'waiting'"
        ),
        1
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        1
    );
}

#[test]
fn a_new_source_alias_can_resume_verification_without_retrying_the_unavailable_alias() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let first = claim(&mut catalog, &session);
    unavailable(&mut catalog, &first);
    let generation = selected(&catalog, "a").source_generation;
    catalog
        .connection
        .execute(
            "UPDATE asset_locations SET source_generation = ?1 WHERE location_id = 'b'",
            [i64::try_from(generation).unwrap()],
        )
        .unwrap();
    request(
        &mut catalog,
        &session,
        &["b"],
        ClassificationWorkPriority::NewContent,
    );
    let resumed = claim(&mut catalog, &session);
    assert_eq!(resumed.work_id, first.work_id);
    assert!(
        matches!(resumed.target, ClassificationWorkTarget::VerifySource(source) if source.location_id == "b")
    );
}

#[test]
fn removal_discards_source_failures_and_stale_notifications_cannot_restore_work() {
    let (_directory, mut catalog) = fixture();
    let (_, session) = start(&mut catalog);
    request(
        &mut catalog,
        &session,
        &["a"],
        ClassificationWorkPriority::Requested,
    );
    let first = claim(&mut catalog, &session);
    unavailable(&mut catalog, &first);
    let source = selected(&catalog, "a");
    catalog.unregister_root("root-a").unwrap();
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_unavailable_sources"
        ),
        0
    );
    assert!(
        catalog
            .refresh_classification_sources(&session, &[source], 10)
            .is_err()
    );
    assert!(
        catalog
            .recover_classification_root(&session, &root("root-a"), None, 10)
            .is_err()
    );
    assert!(
        catalog
            .claim_classification_work(&session, 100_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        scalar(
            &catalog,
            "SELECT count(*) FROM classification_work_items WHERE phase = 'waiting'"
        ),
        0
    );
}
