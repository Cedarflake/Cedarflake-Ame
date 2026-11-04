use super::*;

#[test]
fn cancellation_before_native_dispatch_retires_the_session_without_opening_storage() {
    let session = open("cancel-before-dispatch".into()).unwrap();
    session.cancel();
    let outcome = session.run().unwrap();
    assert_eq!(outcome.phase, SimilarAnalysisPhase::Cancelled);
    assert_eq!(outcome.checked_sources, 0);
    assert_eq!(
        session.progress().unwrap().phase,
        SimilarAnalysisPhase::Cancelled
    );
    assert_eq!(
        session.run().unwrap_err().code,
        "similarity_already_started"
    );
}
