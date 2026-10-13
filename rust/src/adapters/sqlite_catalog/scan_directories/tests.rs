use tempfile::{TempDir, tempdir};

use crate::domain::{ScanCheckpoint, ScanRequest};
use crate::ports::CatalogRepository;

use super::*;

struct Fixture {
    _directory: TempDir,
    catalog: SqliteCatalog,
    request: ScanRequest,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("derived storage");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
        let request = ScanRequest {
            scan_id: "directory-scan".to_owned(),
            root_path: directory
                .path()
                .join("source")
                .to_string_lossy()
                .into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        catalog
            .begin_scan(&request, "root", &request.root_path)
            .expect("scan");
        assert_eq!(
            catalog
                .claim_next_directory(&request.scan_id)
                .expect("root frontier"),
            Some(String::new())
        );
        Self {
            _directory: directory,
            catalog,
            request,
        }
    }

    fn capture(&mut self) -> Result<(), ScanError> {
        self.catalog
            .complete_directory_with_identity(&self.request.scan_id, "", Some(&identity()))
    }

    fn count(&self) -> i64 {
        self.catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM scan_directory_identities",
                [],
                |row| row.get(0),
            )
            .expect("retained bindings")
    }
}

fn identity() -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: "0000000000000001:00000000000000000000000000000002".to_owned(),
    }
}

#[test]
fn directory_binding_and_enumeration_checkpoint_commit_atomically() {
    let mut fixture = Fixture::new();
    fixture
        .catalog
        .connection
        .execute_batch(
            "CREATE TRIGGER reject_directory_identity BEFORE INSERT ON scan_directory_identities
         BEGIN SELECT RAISE(ABORT, 'controlled write failure'); END;",
        )
        .expect("rollback fixture");
    assert!(fixture.capture().is_err());
    assert_eq!(fixture.count(), 0);
    assert!(
        !fixture
            .catalog
            .is_current_directory_enumerated(&fixture.request.scan_id, "")
            .expect("rolled back enumeration marker")
    );
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_directory_identity")
        .expect("remove controlled failure");
    fixture.capture().expect("atomic capture");
    assert_eq!(fixture.count(), 1);
    assert!(
        fixture
            .catalog
            .is_current_directory_enumerated(&fixture.request.scan_id, "")
            .expect("committed enumeration marker")
    );
}

#[test]
fn consumed_frontier_does_not_remove_published_directory_history() {
    let mut fixture = Fixture::new();
    fixture.capture().expect("root binding");
    fixture
        .catalog
        .complete_directory(&fixture.request.scan_id, &ScanCheckpoint::default())
        .expect("consume frontier");
    assert_eq!(fixture.count(), 1);
    let active_count = |catalog: &SqliteCatalog| {
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM scan_directory_identities AS directory
         JOIN library_roots AS root ON root.active_scan_id = directory.scan_id",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("published directory lookup")
    };
    assert_eq!(active_count(&fixture.catalog), 0);
    fixture
        .catalog
        .publish_scan(&fixture.request.scan_id, "root", 0, 0)
        .expect("publish scan");
    assert_eq!(active_count(&fixture.catalog), 1);
    let path = fixture.catalog.catalog_path().to_path_buf();
    drop(fixture.catalog);
    let reopened = SqliteCatalog::open(path).expect("reopen published catalog");
    assert_eq!(active_count(&reopened), 1);
}

#[test]
fn explicit_first_import_resume_discards_previous_directory_bindings() {
    let mut fixture = Fixture::new();
    fixture.capture().expect("root binding");
    fixture
        .catalog
        .pause_scan(&fixture.request.scan_id, &ScanCheckpoint::default())
        .expect("pause first import");
    fixture
        .catalog
        .resume_scan(&fixture.request, "root", &fixture.request.root_path)
        .expect("resume first import");
    assert_eq!(fixture.count(), 0);
}

#[test]
fn abandoned_scan_retires_only_its_unpublished_bindings() {
    let mut fixture = Fixture::new();
    fixture.capture().expect("root binding");
    fixture
        .catalog
        .abandon_scan(&fixture.request.scan_id, "cancelled", 0)
        .expect("cancel scan");
    assert_eq!(fixture.count(), 0);
}

#[test]
fn stale_root_generation_cannot_complete_directory_capture() {
    let mut fixture = Fixture::new();
    fixture.catalog.connection.execute(
        "UPDATE library_change_root_state SET generation = generation + 1 WHERE root_id = 'root'", [],
    ).expect("retire previous generation");
    let error = fixture.capture().expect_err("stale capture");
    assert_eq!(error.code, "catalog_scan_directory_not_enumerable");
    assert_eq!(fixture.count(), 0);
}
