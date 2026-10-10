use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use tempfile::tempdir;

use super::*;

fn fixture(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("isolated identity-state database");
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE library_roots(id TEXT PRIMARY KEY, active_scan_id TEXT);
             CREATE TABLE asset_locations(
               root_id TEXT NOT NULL, scan_id TEXT NOT NULL,
               file_identity_scheme TEXT NOT NULL, file_identity_value TEXT NOT NULL,
               source_generation INTEGER NOT NULL, source_revision_token TEXT,
               file_size INTEGER NOT NULL, modified_unix_ms INTEGER NOT NULL
             );
             CREATE INDEX identity_lookup ON asset_locations(
               file_identity_scheme, file_identity_value, scan_id
             );
             INSERT INTO library_roots VALUES ('root', 'published');",
        )
        .expect("identity query fixture");
    connection
}

fn identity(value: &str) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "fixture-identity".to_owned(),
        value: value.to_owned(),
    }
}

fn state(generation: i64, revision: Option<&str>, file_size: i64) -> StoredIdentityGroupState {
    StoredIdentityGroupState {
        generation,
        source_revision_token: revision.map(str::to_owned),
        file_size,
        modified_unix_ms: 1000,
    }
}

fn insert(connection: &Connection, value: &str, scan: &str, state: &StoredIdentityGroupState) {
    connection
        .execute(
            "INSERT INTO asset_locations VALUES (
               'root', ?1, 'fixture-identity', ?2, ?3, ?4, ?5, ?6
             )",
            params![
                scan,
                value,
                state.generation,
                state.source_revision_token,
                state.file_size,
                state.modified_unix_ms,
            ],
        )
        .expect("insert physical-file observation");
}

#[test]
fn unchanged_staging_reuses_two_statements_across_distinct_file_parameters() {
    let directory = tempdir().expect("isolated storage");
    let connection = fixture(&directory.path().join("identity.sqlite3"));
    for index in 0..256 {
        insert(
            &connection,
            &index.to_string(),
            "published",
            &state(index + 1, Some("revision"), index + 100),
        );
    }
    let preparations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&preparations);
    connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            if matches!(context.action, AuthAction::Select) {
                observed.fetch_add(1, Ordering::Relaxed);
            }
            Authorization::Allow
        }))
        .expect("observe actual query compilations");

    for index in 0..256 {
        let current = load_staging_identity_group_state(
            &connection,
            &identity(&index.to_string()),
            "replacement",
        )
        .expect("current identity state");
        assert_eq!(
            current,
            Some(state(index + 1, Some("revision"), index + 100))
        );
    }
    assert_eq!(preparations.load(Ordering::Relaxed), 2);
}

#[test]
fn staging_precedence_and_empty_results_do_not_reuse_previous_parameters() {
    let directory = tempdir().expect("isolated storage");
    let connection = fixture(&directory.path().join("identity.sqlite3"));
    insert(
        &connection,
        "first",
        "published",
        &state(1, Some("old"), 100),
    );
    insert(
        &connection,
        "first",
        "replacement",
        &state(2, Some("new"), 200),
    );
    insert(
        &connection,
        "first",
        "retired",
        &state(3, Some("retired"), 300),
    );
    insert(&connection, "second", "published", &state(4, None, 400));

    assert_eq!(
        load_staging_identity_group_state(&connection, &identity("first"), "replacement")
            .expect("staged observation takes precedence"),
        Some(state(2, Some("new"), 200)),
    );
    assert_eq!(
        load_staging_identity_group_state(&connection, &identity("second"), "replacement")
            .expect("fall back only for the requested identity"),
        Some(state(4, None, 400)),
    );
    assert_eq!(
        load_staging_identity_group_state(&connection, &identity("first"), "empty")
            .expect("fall back only for the requested scan"),
        Some(state(1, Some("old"), 100)),
    );
    assert_eq!(
        load_staging_identity_group_state(&connection, &identity("missing"), "replacement")
            .expect("absent identity"),
        None,
    );
    assert_eq!(
        load_scan_identity_group_state(&connection, &identity("second"), "replacement")
            .expect("absent staged observation"),
        None,
    );
}

#[test]
fn cached_queries_observe_peer_commits_and_release_transaction_snapshots() {
    let directory = tempdir().expect("isolated storage");
    let path = directory.path().join("identity.sqlite3");
    let mut connection = fixture(&path);
    insert(&connection, "file", "published", &state(1, None, 100));
    let file = identity("file");
    assert_eq!(
        load_active_identity_group_state(&connection, &file).expect("initial state"),
        Some(state(1, None, 100)),
    );

    let peer = Connection::open(&path).expect("independent writer");
    peer.execute(
        "UPDATE asset_locations SET source_generation = 2, source_revision_token = 'new'",
        [],
    )
    .expect("commit a later observation");
    assert_eq!(
        load_active_identity_group_state(&connection, &file).expect("current committed state"),
        Some(state(2, Some("new"), 100)),
    );
    let transaction = connection.transaction().expect("provisional observation");
    transaction
        .execute("UPDATE asset_locations SET source_generation = 3", [])
        .expect("write within the current transaction");
    assert_eq!(
        load_active_identity_group_state(&transaction, &file).expect("transaction-local state"),
        Some(state(3, Some("new"), 100)),
    );
    transaction.rollback().expect("roll back provisional state");
    assert_eq!(
        load_active_identity_group_state(&connection, &file).expect("state after rollback"),
        Some(state(2, Some("new"), 100)),
    );
    peer.execute("UPDATE library_roots SET active_scan_id = NULL", [])
        .expect("retire published authority");
    assert_eq!(
        load_active_identity_group_state(&connection, &file).expect("retired publication"),
        None,
    );
}

#[test]
fn physical_alias_conflicts_and_unknown_revisions_keep_their_existing_meaning() {
    let directory = tempdir().expect("isolated storage");
    let connection = fixture(&directory.path().join("identity.sqlite3"));
    let cases = [
        ("known with unknown", state(1, None, 100), false),
        ("same known revision", state(1, Some("known"), 100), false),
        ("different generation", state(2, Some("known"), 100), true),
        ("different revision", state(1, Some("different"), 100), true),
        ("different size", state(1, Some("known"), 101), true),
        (
            "different modification time",
            StoredIdentityGroupState {
                modified_unix_ms: 1001,
                ..state(1, Some("known"), 100)
            },
            true,
        ),
    ];
    for (name, alias, conflicts) in cases {
        for scan in ["published", "replacement"] {
            connection
                .execute("DELETE FROM asset_locations", [])
                .expect("clear observations");
            insert(&connection, "file", scan, &state(1, Some("known"), 100));
            insert(&connection, "file", scan, &alias);
            let result =
                load_staging_identity_group_state(&connection, &identity("file"), "replacement");
            if conflicts {
                assert_eq!(
                    result.expect_err(name).code,
                    "catalog_source_identity_state_unverifiable"
                );
            } else {
                assert_eq!(result.expect(name), Some(state(1, Some("known"), 100)));
            }
        }
    }
    connection
        .execute("DELETE FROM asset_locations", [])
        .expect("clear observations");
    insert(&connection, "file", "published", &state(1, None, 100));
    insert(&connection, "file", "published", &state(1, None, 100));
    assert_eq!(
        load_active_identity_group_state(&connection, &identity("file"))
            .expect("unknown revisions"),
        Some(state(1, None, 100)),
    );
}
