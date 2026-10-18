use crate::adapters::sqlite_catalog::tests::support::{
    fixture_request, insert_active_identity_alias, publish_fixture, publish_preview_artifact,
};

use tempfile::tempdir;

use crate::adapters::sqlite_catalog::*;

#[derive(Debug, PartialEq, Eq)]
struct AliasState {
    location_id: String,
    generation: i64,
    revision: Option<String>,
    file_size: i64,
    modified_unix_ms: i64,
    width: i64,
    height: i64,
    preview_path: String,
    preview_status: String,
}

struct Fixture {
    _directory: tempfile::TempDir,
    catalog: SqliteCatalog,
    active: Vec<AssetLocationView>,
    preview_references: Vec<(String, String)>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().expect("isolated catalog");
        let mut catalog =
            SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("fixture catalog");
        publish_fixture(&mut catalog, "published", "root", "C:\\Fixture", "a");
        insert_active_identity_alias(&mut catalog, "a", "b");
        catalog
            .connection
            .execute_batch(
                "UPDATE asset_locations
             SET source_revision_token = 'windows-file-change-time-100ns-v1:0000000000000001'",
            )
            .expect("known source revision for both aliases");
        publish_preview_artifact(&mut catalog, "a", "kept-preview", "C:\\Cache\\kept.jpg");
        let active = ["a", "b"]
            .iter()
            .map(|id| catalog.load_active_location(id).unwrap().unwrap())
            .collect::<Vec<_>>();
        let preview_references = references(&catalog.connection);
        assert!(!preview_references.is_empty());
        catalog
            .begin_scan(
                &fixture_request("replacement", "C:\\Fixture"),
                "root",
                "C:\\Fixture",
            )
            .expect("admit replacement scan");
        for prior in &active {
            let mut staged = prior.clone();
            staged.scan_id = "replacement".to_owned();
            catalog
                .stage_location("replacement", "root", &staged)
                .expect("stage aliases");
        }
        catalog
            .flush_pending_locations()
            .expect("commit coherent aliases");
        Self {
            _directory: directory,
            catalog,
            active,
            preview_references,
        }
    }

    fn pending(&self, location_id: &str, revision: &str) -> PendingLocation {
        let mut location = self
            .active
            .iter()
            .find(|location| location.location_id == location_id)
            .expect("known alias")
            .clone();
        location.scan_id = "replacement".to_owned();
        location
            .source_revision
            .as_mut()
            .expect("known revision")
            .value = revision.to_owned();
        location.source_generation = 0;
        location.file_size = 50;
        location.modified_unix_ms = 60;
        location.width = 70;
        location.height = 80;
        location.preview_path.clear();
        location.preview_status = PreviewStatus::Pending;
        let baseline = load_staging_identity_group_state(
            &self.catalog.connection,
            location.file_identity.as_ref().expect("known identity"),
            "replacement",
        )
        .expect("captured source state");
        PendingLocation {
            scan_id: "replacement".to_owned(),
            root_id: "root".to_owned(),
            location,
            identity_group_baseline: baseline,
        }
    }

    fn assert_active_unchanged(&self) {
        for before in &self.active {
            assert_eq!(
                self.catalog
                    .load_active_location(&before.location_id)
                    .unwrap(),
                Some(before.clone())
            );
        }
        assert_eq!(
            references(&self.catalog.connection),
            self.preview_references
        );
    }
}

fn references(connection: &Connection) -> Vec<(String, String)> {
    connection.prepare("SELECT artifact_key, location_id FROM preview_artifact_locations ORDER BY artifact_key, location_id")
        .expect("preview reference query").query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("preview references").collect::<Result<Vec<_>, _>>().expect("valid references")
}

fn aliases(connection: &Connection) -> Vec<AliasState> {
    scan_aliases(connection, "replacement")
}

fn scan_aliases(connection: &Connection, scan_id: &str) -> Vec<AliasState> {
    connection.prepare(
        "SELECT location_id, source_generation, source_revision_token, file_size, modified_unix_ms,
                width, height, preview_path, preview_status
         FROM asset_locations WHERE scan_id = ?1 ORDER BY location_id",
    ).expect("alias query").query_map([scan_id], |row| Ok(AliasState {
        location_id: row.get(0)?, generation: row.get(1)?, revision: row.get(2)?,
        file_size: row.get(3)?, modified_unix_ms: row.get(4)?, width: row.get(5)?, height: row.get(6)?,
        preview_path: row.get(7)?, preview_status: row.get(8)?,
    })).expect("alias rows").collect::<Result<Vec<_>, _>>().expect("valid aliases")
}

fn next_generation(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT next_source_generation FROM catalog_state",
            [],
            |row| row.get(0),
        )
        .expect("generation allocator")
}

#[test]
fn staged_hardlink_aliases_share_one_generation_without_changing_active_previews() {
    let mut fixture = Fixture::new();
    let generation = next_generation(&fixture.catalog.connection);
    fixture.catalog.pending_locations = vec![
        fixture.pending("a", "0000000000000002"),
        fixture.pending("b", "0000000000000002"),
    ];
    fixture
        .catalog
        .flush_pending_locations()
        .expect("publish one staged identity observation");
    assert_eq!(next_generation(&fixture.catalog.connection), generation + 1);
    let aliases = aliases(&fixture.catalog.connection);
    assert_eq!(aliases.len(), 2);
    for alias in aliases {
        assert_eq!(alias.generation, generation);
        assert_eq!(
            alias.revision.as_deref(),
            Some("windows-file-change-time-100ns-v1:0000000000000002")
        );
        assert_eq!(
            (
                alias.file_size,
                alias.modified_unix_ms,
                alias.width,
                alias.height
            ),
            (50, 60, 70, 80)
        );
        assert!(alias.preview_path.is_empty());
        assert_eq!(alias.preview_status, "pending");
    }
    fixture.assert_active_unchanged();
    fixture
        .catalog
        .abandon_scan("replacement", "cancelled", 0)
        .expect("cancel unpublished replacement");
    fixture.assert_active_unchanged();
    drop(SqliteCatalog::open(fixture.catalog.catalog_path().to_owned()).expect("full reopen"));
}

#[test]
fn rejected_alias_write_rolls_back_propagation_and_generation_allocation() {
    let mut fixture = Fixture::new();
    let before = aliases(&fixture.catalog.connection);
    let generation = next_generation(&fixture.catalog.connection);
    fixture
        .catalog
        .pending_locations
        .push(fixture.pending("a", "0000000000000002"));
    fixture
        .catalog
        .connection
        .execute_batch(
            "CREATE TEMP TRIGGER reject_replacement BEFORE INSERT ON main.asset_locations
         WHEN NEW.scan_id = 'replacement'
         BEGIN SELECT RAISE(ABORT, 'controlled alias write failure'); END",
        )
        .expect("reject target write after sibling propagation");
    assert!(fixture.catalog.flush_pending_locations().is_err());
    assert_eq!(aliases(&fixture.catalog.connection), before);
    assert_eq!(next_generation(&fixture.catalog.connection), generation);
    assert_eq!(fixture.catalog.pending_locations.len(), 1);
    fixture.assert_active_unchanged();
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_replacement")
        .expect("retire fixture failure");
    fixture
        .catalog
        .flush_pending_locations()
        .expect("retry retained atomic write");
    fixture.assert_active_unchanged();
    drop(SqliteCatalog::open(fixture.catalog.catalog_path().to_owned()).expect("full reopen"));
}

#[test]
fn conflicting_alias_batch_rolls_back_the_first_observation() {
    let mut fixture = Fixture::new();
    let before = aliases(&fixture.catalog.connection);
    let generation = next_generation(&fixture.catalog.connection);
    fixture.catalog.pending_locations = vec![
        fixture.pending("a", "0000000000000002"),
        fixture.pending("b", "0000000000000003"),
    ];
    let error = fixture
        .catalog
        .flush_pending_locations()
        .expect_err("reject conflicting source states");
    assert_eq!(error.code, "catalog_source_observation_superseded");
    assert_eq!(aliases(&fixture.catalog.connection), before);
    assert_eq!(next_generation(&fixture.catalog.connection), generation);
    fixture.assert_active_unchanged();
}

#[test]
fn stale_captured_alias_cannot_overwrite_a_newer_committed_generation() {
    let mut fixture = Fixture::new();
    let stale = fixture.pending("b", "0000000000000002");
    fixture
        .catalog
        .pending_locations
        .push(fixture.pending("a", "0000000000000003"));
    fixture
        .catalog
        .flush_pending_locations()
        .expect("newer observation commits first");
    let before = aliases(&fixture.catalog.connection);
    let generation = next_generation(&fixture.catalog.connection);
    fixture.catalog.pending_locations.push(stale);
    let error = fixture
        .catalog
        .flush_pending_locations()
        .expect_err("old captured state is superseded");
    assert_eq!(error.code, "catalog_source_observation_superseded");
    assert_eq!(aliases(&fixture.catalog.connection), before);
    assert_eq!(next_generation(&fixture.catalog.connection), generation);
    fixture.assert_active_unchanged();
}

#[test]
fn rejected_resume_checkpoint_rolls_back_aliases_allocator_and_pending_roster() {
    let mut fixture = Fixture::new();
    let request = fixture_request("retained", "C:\\Retained");
    fixture
        .catalog
        .begin_scan(&request, "retained-root", &request.root_path)
        .expect("first import without a published baseline");
    let mut locations = fixture.active.clone();
    for location in &mut locations {
        location.root_id = "retained-root".to_owned();
        location.scan_id = request.scan_id.clone();
        location.absolute_path = format!("{}\\{}", request.root_path, location.relative_path);
        location.display_path = location.absolute_path.clone();
        fixture
            .catalog
            .stage_location(&request.scan_id, &location.root_id, location)
            .expect("first-import aliases");
    }
    let checkpoint = ScanCheckpoint {
        accepted_items: 2,
        ..ScanCheckpoint::default()
    };
    fixture
        .catalog
        .pause_scan(&request.scan_id, &checkpoint)
        .expect("pause first import");
    fixture
        .catalog
        .resume_scan(&request, "retained-root", &request.root_path)
        .expect("retain the unverified namespace");
    let pending_count = |catalog: &SqliteCatalog| -> i64 {
        catalog
            .connection
            .query_row(
                "SELECT COUNT(*) FROM scan_resume_pending_locations WHERE scan_id = 'retained'",
                [],
                |row| row.get(0),
            )
            .expect("retained resume membership")
    };
    assert_eq!(pending_count(&fixture.catalog), 2);
    let before = scan_aliases(&fixture.catalog.connection, &request.scan_id);
    let generation = next_generation(&fixture.catalog.connection);
    for location in &mut locations {
        location
            .source_revision
            .as_mut()
            .expect("known revision")
            .value = "0000000000000002".to_owned();
        location.source_generation = 0;
        location.file_size = 50;
        location.modified_unix_ms = 60;
        let baseline = load_staging_identity_group_state(
            &fixture.catalog.connection,
            location.file_identity.as_ref().expect("known identity"),
            &request.scan_id,
        )
        .expect("capture resumed identity state");
        fixture.catalog.pending_locations.push(PendingLocation {
            scan_id: request.scan_id.clone(),
            root_id: location.root_id.clone(),
            location: location.clone(),
            identity_group_baseline: baseline,
        });
    }
    fixture.catalog.connection.execute_batch(
        "CREATE TEMP TRIGGER reject_resume_progress BEFORE UPDATE OF accepted_items ON main.scan_runs
         WHEN NEW.id = 'retained'
         BEGIN SELECT RAISE(ABORT, 'controlled checkpoint failure'); END",
    ).expect("reject the checkpoint after both alias writes and roster acceptance");
    assert!(
        fixture
            .catalog
            .checkpoint_scan(&request.scan_id, &checkpoint)
            .is_err()
    );
    assert_eq!(
        scan_aliases(&fixture.catalog.connection, &request.scan_id),
        before
    );
    assert_eq!(next_generation(&fixture.catalog.connection), generation);
    assert_eq!(pending_count(&fixture.catalog), 2);
    assert_eq!(fixture.catalog.pending_locations.len(), 2);
    fixture.assert_active_unchanged();
    fixture
        .catalog
        .connection
        .execute_batch("DROP TRIGGER reject_resume_progress")
        .expect("remove checkpoint rejection");
    fixture
        .catalog
        .checkpoint_scan(&request.scan_id, &checkpoint)
        .expect("retry the intact batch");
    assert_eq!(pending_count(&fixture.catalog), 0);
    assert_eq!(next_generation(&fixture.catalog.connection), generation + 1);
    assert!(
        scan_aliases(&fixture.catalog.connection, &request.scan_id)
            .iter()
            .all(|alias| alias.generation == generation && alias.file_size == 50)
    );
    fixture.assert_active_unchanged();
    drop(SqliteCatalog::open(fixture.catalog.catalog_path().to_owned()).expect("full reopen"));
}
