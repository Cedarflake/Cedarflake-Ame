use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;

use crate::adapters::production_identity_journal_factory;
use crate::journal_broker::{
    BrokerResponse, JournalCapability, PersistentChangeJournalConnection, QueryJournalRequest,
    RegisterRootRequest, describe_production_persistent_journal_root,
    production_persistent_journal_caller_claim,
};

pub(super) fn journal_capable_fixture() -> TempDir {
    let parents = [
        std::env::current_dir().expect("working directory"),
        std::env::temp_dir(),
    ];
    for (index, parent) in parents.iter().enumerate() {
        let fixture = tempfile::tempdir_in(parent).expect("owned ordinary-journal fixture");
        let source = fixture.path().join("source");
        fs::create_dir(&source).expect("owned source directory");
        if has_supported_journal(&source) {
            let _ = writeln!(
                std::io::stderr().lock(),
                "ordinary journal fixture candidate={index} capability=supported"
            );
            return fixture;
        }
        let _ = writeln!(
            std::io::stderr().lock(),
            "ordinary journal fixture candidate={index} capability=live_only"
        );
    }
    panic!(
        "No generated fixture volume exposes a readable existing journal; restart coverage cannot be verified"
    );
}

fn has_supported_journal(source: &Path) -> bool {
    let connection = production_identity_journal_factory().connect();
    let PersistentChangeJournalConnection::Connected(session) = &connection else {
        panic!("The ordinary adapter must expose its native session");
    };
    let registration = describe_production_persistent_journal_root("ordinary-fixture", 1, source)
        .expect("owned root authorization");
    let caller = production_persistent_journal_caller_claim().expect("ordinary process identity");
    let capability = session
        .register_root(RegisterRootRequest {
            caller: caller.clone(),
            root: registration.authorization.clone(),
            client_root_handle: registration.client_root_handle,
            timeout_ms: 1_000,
        })
        .expect("native fixture registration");
    let response = session.query_journal(QueryJournalRequest {
        caller,
        root: registration.authorization.clone(),
        root_capability: capability,
        timeout_ms: 1_000,
    });
    connection.close().expect("retire native fixture probe");
    match response.expect("native fixture journal query") {
        BrokerResponse::Journal {
            capability: JournalCapability::Supported,
            journal_id: Some(journal_id),
            first_usn: Some(first_usn),
            next_usn: Some(next_usn),
            ..
        } if journal_id > 0 && first_usn >= 0 && next_usn >= first_usn => true,
        BrokerResponse::Journal {
            capability: JournalCapability::LiveOnly,
            journal_id: None,
            first_usn: None,
            next_usn: None,
            ..
        } => false,
        response => panic!("Invalid native fixture journal response: {response:?}"),
    }
}
