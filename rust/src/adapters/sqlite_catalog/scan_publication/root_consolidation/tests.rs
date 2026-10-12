use tempfile::{TempDir, tempdir};

use crate::domain::ScanRequest;
use crate::ports::{CatalogRepository, RootConsolidationCandidate};

use super::*;

#[test]
fn child_generation_change_rejects_a_previously_captured_plan() {
    let (mut catalog, _directory, plan) = fixture();
    catalog.connection.execute(
        "UPDATE library_change_root_state SET generation = generation + 1 WHERE root_id = 'child'",
        [],
    ).expect("a newer child generation");
    let transaction = catalog
        .connection
        .transaction()
        .expect("publication transaction");
    assert_eq!(
        require_current(&transaction, "parent-scan", "parent", &plan)
            .expect_err("stale child authority cannot be retired")
            .code,
        "catalog_root_consolidation_changed"
    );
}

#[test]
fn new_child_registration_rejects_an_incomplete_captured_roster() {
    let (mut catalog, _directory, plan) = fixture();
    let root_path = Path::new(&plan.destination_path)
        .join("new-child")
        .to_string_lossy()
        .into_owned();
    let request = request("new-child-scan", &root_path);
    catalog
        .begin_scan(&request, "new-child", &root_path)
        .expect("concurrent child admission");
    catalog
        .abandon_scan(&request.scan_id, "cancelled", 0)
        .expect("new inactive registration");
    let transaction = catalog
        .connection
        .transaction()
        .expect("publication transaction");
    assert_eq!(
        require_current(&transaction, "parent-scan", "parent", &plan)
            .expect_err("a newly registered child was not proven by the captured plan")
            .code,
        "catalog_root_consolidation_changed"
    );
}

fn fixture() -> (SqliteCatalog, TempDir, RootConsolidationPlan) {
    let directory = tempdir().expect("isolated catalog");
    let parent_path = directory
        .path()
        .join("source")
        .to_string_lossy()
        .into_owned();
    let child_path = Path::new(&parent_path)
        .join("child")
        .to_string_lossy()
        .into_owned();
    let mut catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("catalog");
    catalog
        .begin_scan(&request("child-scan", &child_path), "child", &child_path)
        .expect("child admission");
    catalog
        .abandon_scan("child-scan", "cancelled", 0)
        .expect("retained inactive child registration");
    catalog
        .begin_scan(
            &request("parent-scan", &parent_path),
            "parent",
            &parent_path,
        )
        .expect("parent admission");
    let mut registrations = catalog
        .load_root_consolidation_registrations("parent")
        .expect("captured registrations");
    assert_eq!(registrations.len(), 1);
    let plan = RootConsolidationPlan {
        scan_id: "parent-scan".to_owned(),
        destination_root_id: "parent".to_owned(),
        destination_path: parent_path,
        candidates: vec![RootConsolidationCandidate {
            expected: registrations.remove(0),
            relative_path: "child".to_owned(),
        }],
    };
    {
        let transaction = catalog
            .connection
            .transaction()
            .expect("initial validation");
        require_current(&transaction, "parent-scan", "parent", &plan)
            .expect("unchanged plan remains valid");
    }
    (catalog, directory, plan)
}

fn request(scan_id: &str, root_path: &str) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_path.to_owned(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    }
}
