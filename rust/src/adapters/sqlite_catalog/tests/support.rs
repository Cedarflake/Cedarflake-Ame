use crate::adapters::sqlite_catalog::migrations::downgrade_source_revision_contract_to_v30_for_test;
use crate::adapters::sqlite_catalog::*;

pub(super) const TEST_QUERY_ID: &str = "test-default-query";
type GalleryQueryFixture<'a> = (&'a str, &'a str, Option<&'a str>, Option<i64>, i64);

pub(super) fn load_default_snapshot(
    catalog: &mut SqliteCatalog,
    max_items: u32,
    after: Option<&CatalogCursor>,
) -> Result<CatalogSnapshot, ScanError> {
    catalog.load_snapshot(
        max_items,
        &GalleryQuery::default(),
        TEST_QUERY_ID,
        after,
        None,
        None,
    )
}

pub(super) fn load_default_timeline(
    catalog: &mut SqliteCatalog,
) -> Result<GalleryTimeline, ScanError> {
    catalog.load_gallery_timeline(&GalleryQuery::default(), TEST_QUERY_ID)
}

pub(super) fn publish_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    location_id: &str,
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin scan");
    catalog
        .prove_live_only_first_import_handoff_for_test(scan_id)
        .expect("prove fixture first-import handoff");
    catalog
        .stage_location(
            scan_id,
            root_id,
            &AssetLocationView {
                asset_id: format!("asset-{scan_id}"),
                location_id: location_id.to_owned(),
                root_id: root_id.to_owned(),
                scan_id: scan_id.to_owned(),
                absolute_path: format!("{root_path}\\one.png"),
                display_path: format!("{root_path}\\one.png"),
                relative_path: "one.png".to_owned(),
                preview_path: format!("C:\\Cache\\{scan_id}.jpg"),
                file_size: 20,
                created_unix_ms: Some(25),
                modified_unix_ms: 30,
                file_identity: None,
                source_revision: Some(SourceRevisionEvidence {
                    scheme: "windows-file-change-time-100ns-v1".to_owned(),
                    value: "0000000000000001".to_owned(),
                }),
                source_generation: 1,
                width: 40,
                height: 50,
                preview_status: PreviewStatus::Ready,
                preview_issue_code: None,
                preview_issue_message: None,
                metadata_engine_id: "fixture-metadata".to_owned(),
                metadata_engine_version: "1".to_owned(),
                camera: Default::default(),
                capture_time: None,
            },
        )
        .expect("stage location");
    catalog
        .publish_scan(scan_id, root_id, 1, 0)
        .expect("publish scan");
}

pub(super) fn insert_active_identity_alias(
    catalog: &mut SqliteCatalog,
    original_location_id: &str,
    alias_location_id: &str,
) {
    catalog
        .connection
        .execute(
            "UPDATE asset_locations
             SET file_identity_scheme = 'windows-file-id-128-v1',
                 file_identity_value =
                   '0000000000000001:00000000000000000000000000000001',
                 source_revision_token = NULL
             WHERE location_id = ?1",
            [original_location_id],
        )
        .expect("assign original identity");
    let alias_asset_id = format!("asset-{alias_location_id}");
    catalog
        .connection
        .execute(
            "INSERT INTO assets(id, created_unix_ms) VALUES (?1, 1)",
            [&alias_asset_id],
        )
        .expect("insert alias asset");
    catalog
        .connection
        .execute(
            "INSERT INTO asset_locations(
               scan_id, asset_id, location_id, root_id, absolute_path, relative_path,
               preview_path, file_size, created_unix_ms, modified_unix_ms,
               file_local_time, parent_relative_path, natural_name_key, width, height,
               preview_status, preview_issue_code, preview_issue_message,
               metadata_engine_id, metadata_engine_version, capture_local_time,
               capture_offset_minutes, capture_time_source, capture_raw_value,
               file_identity_scheme, file_identity_value, source_revision_token,
               source_generation
             )
             SELECT scan_id, ?2, ?3, root_id, absolute_path, 'alias.png',
                    '', file_size, created_unix_ms, modified_unix_ms,
                    file_local_time, '', 'alias.png', width, height,
                    'pending', NULL, NULL, metadata_engine_id, metadata_engine_version,
                    capture_local_time, capture_offset_minutes, capture_time_source,
                    capture_raw_value, file_identity_scheme, file_identity_value, NULL,
                    source_generation
             FROM asset_locations WHERE location_id = ?1",
            params![original_location_id, alias_asset_id, alias_location_id],
        )
        .expect("insert identity alias");
}

pub(super) fn publish_preview_artifact(
    catalog: &mut SqliteCatalog,
    location_id: &str,
    artifact_key: &str,
    artifact_path: &str,
) -> PreviewArtifact {
    let mut location = catalog
        .load_active_location(location_id)
        .expect("active location query")
        .expect("active location");
    let request = exact_preview_request(&location);
    if location.source_revision.is_none() {
        location.source_revision = Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: "0000000000000001".to_owned(),
        });
    }
    location.preview_path = artifact_path.to_owned();
    location.preview_status = PreviewStatus::Ready;
    let artifact = PreviewArtifact {
        artifact_key: artifact_key.to_owned(),
        algorithm_id: "ame-jpeg-thumbnail".to_owned(),
        algorithm_version: 2,
        orientation_contract: "exif-display-v1".to_owned(),
        size_bucket: 256,
        path: location.preview_path.clone(),
        byte_size: 1_024,
        encoded_width: 40,
        encoded_height: 50,
        width: location.width,
        height: location.height,
    };
    catalog
        .update_active_preview(&location, Some(&artifact), Some(&request))
        .expect("publish preview artifact");
    artifact
}

pub(super) fn exact_preview_request(location: &AssetLocationView) -> PreviewRequest {
    PreviewRequest {
        location_id: location.location_id.clone(),
        expected_root_id: location.root_id.clone(),
        expected_scan_id: location.scan_id.clone(),
        expected_source_revision: location.source_revision.clone(),
        expected_source_generation: location.source_generation,
        preview_edge: 256,
        retry_failed: false,
        protected_location_ids: Vec::new(),
    }
}

pub(super) fn ensure_legacy_scan_runs_contract(connection: &Connection) {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS scan_runs (
               id TEXT PRIMARY KEY,
               root_id TEXT NOT NULL,
               status TEXT NOT NULL,
               started_unix_ms INTEGER NOT NULL,
               completed_unix_ms INTEGER,
               asset_count INTEGER NOT NULL DEFAULT 0,
               issue_count INTEGER NOT NULL DEFAULT 0,
               max_items INTEGER,
               max_entries INTEGER,
               preview_edge INTEGER NOT NULL,
               current_directory_relative_path TEXT,
               current_directory_enumerated INTEGER NOT NULL DEFAULT 0,
               last_visited_relative_path TEXT,
               visited_entries INTEGER NOT NULL DEFAULT 0,
               accepted_items INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS catalog_state (
               revision INTEGER NOT NULL CHECK(revision >= 0)
             );
             INSERT INTO catalog_state(revision)
               SELECT (SELECT COUNT(*) FROM scan_runs WHERE status = 'completed')
               WHERE NOT EXISTS (SELECT 1 FROM catalog_state);",
        )
        .expect("legacy scan run contract");
    ensure_legacy_asset_location_contract(connection);
}

pub(super) fn ensure_legacy_asset_location_contract(connection: &Connection) {
    let version = connection
        .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("legacy asset location schema version");
    let columns = [
        (1, "absolute_path", "TEXT NOT NULL DEFAULT ''"),
        (1, "preview_path", "TEXT NOT NULL DEFAULT ''"),
        (1, "file_size", "INTEGER NOT NULL DEFAULT 0"),
        (1, "modified_unix_ms", "INTEGER NOT NULL DEFAULT 0"),
        (1, "width", "INTEGER NOT NULL DEFAULT 0"),
        (1, "height", "INTEGER NOT NULL DEFAULT 0"),
        (2, "asset_id", "TEXT NOT NULL DEFAULT ''"),
        (7, "preview_status", "TEXT NOT NULL DEFAULT 'pending'"),
        (7, "preview_issue_code", "TEXT"),
        (7, "preview_issue_message", "TEXT"),
        (8, "metadata_engine_id", "TEXT NOT NULL DEFAULT 'unknown'"),
        (8, "metadata_engine_version", "TEXT NOT NULL DEFAULT '0'"),
        (8, "capture_local_time", "TEXT"),
        (8, "capture_offset_minutes", "INTEGER"),
        (8, "capture_time_source", "TEXT"),
        (8, "capture_raw_value", "TEXT"),
        (9, "file_identity_scheme", "TEXT"),
        (9, "file_identity_value", "TEXT"),
        (12, "created_unix_ms", "INTEGER"),
        (12, "parent_relative_path", "TEXT NOT NULL DEFAULT ''"),
        (12, "natural_name_key", "TEXT NOT NULL DEFAULT ''"),
        (13, "file_local_time", "TEXT"),
    ];
    for (introduced_in, name, definition) in columns {
        if version < introduced_in {
            continue;
        }
        let exists = connection
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM pragma_table_info('asset_locations') WHERE name = ?1
                 )",
                [name],
                |row| row.get::<_, bool>(0),
            )
            .expect("legacy asset location column query");
        if !exists {
            connection
                .execute_batch(&format!(
                    "ALTER TABLE asset_locations ADD COLUMN {name} {definition}"
                ))
                .expect("complete legacy asset location fixture");
        }
    }
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS assets (
               id TEXT PRIMARY KEY, created_unix_ms INTEGER NOT NULL
             );
             INSERT OR IGNORE INTO assets(id, created_unix_ms)
               SELECT DISTINCT asset_id, 0 FROM asset_locations;",
        )
        .expect("retain the legacy asset identity table");
}

pub(super) fn preview_reference_count(catalog: &SqliteCatalog, artifact_key: &str) -> i64 {
    catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM preview_artifact_locations WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview reference count")
}

pub(super) fn preview_lifecycle_state(catalog: &SqliteCatalog, artifact_key: &str) -> String {
    catalog
        .connection
        .query_row(
            "SELECT lifecycle_state FROM preview_artifacts WHERE artifact_key = ?1",
            [artifact_key],
            |row| row.get(0),
        )
        .expect("preview lifecycle state")
}

pub(in crate::adapters::sqlite_catalog) fn publish_gallery_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64)],
) {
    let dimensioned_locations = locations
        .iter()
        .map(|(location_id, capture_local_time, modified_unix_ms)| {
            (*location_id, *capture_local_time, *modified_unix_ms, 40, 50)
        })
        .collect::<Vec<_>>();
    publish_gallery_dimension_fixture(catalog, scan_id, root_id, root_path, &dimensioned_locations);
}

pub(super) fn publish_gallery_query_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[GalleryQueryFixture<'_>],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin query fixture scan");
    prove_gallery_first_import_handoff_if_needed(catalog, scan_id, root_id);
    for (location_id, relative_path, capture_local_time, created_unix_ms, modified_unix_ms) in
        locations
    {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    scan_id: scan_id.to_owned(),
                    absolute_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    display_path: format!("{root_path}\\{}", relative_path.replace('/', "\\")),
                    relative_path: (*relative_path).to_owned(),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: *created_unix_ms,
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: 40,
                    height: 50,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    camera: Default::default(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage query fixture location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("query fixture location count"),
            0,
        )
        .expect("publish query fixture scan");
}

pub(super) fn publish_gallery_dimension_fixture(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
    root_path: &str,
    locations: &[(&str, Option<&str>, i64, u32, u32)],
) {
    let request = fixture_request(scan_id, root_path);
    catalog
        .begin_scan(&request, root_id, root_path)
        .expect("begin gallery scan");
    prove_gallery_first_import_handoff_if_needed(catalog, scan_id, root_id);
    for (location_id, capture_local_time, modified_unix_ms, width, height) in locations {
        catalog
            .stage_location(
                scan_id,
                root_id,
                &AssetLocationView {
                    asset_id: format!("asset-{scan_id}-{location_id}"),
                    location_id: (*location_id).to_owned(),
                    root_id: root_id.to_owned(),
                    scan_id: scan_id.to_owned(),
                    absolute_path: format!("{root_path}\\{location_id}.png"),
                    display_path: format!("{root_path}\\{location_id}.png"),
                    relative_path: format!("{location_id}.png"),
                    preview_path: String::new(),
                    file_size: 20,
                    created_unix_ms: Some(*modified_unix_ms - 1),
                    modified_unix_ms: *modified_unix_ms,
                    file_identity: None,
                    source_revision: None,
                    source_generation: 0,
                    width: *width,
                    height: *height,
                    preview_status: PreviewStatus::Pending,
                    preview_issue_code: None,
                    preview_issue_message: None,
                    metadata_engine_id: "fixture-metadata".to_owned(),
                    metadata_engine_version: "1".to_owned(),
                    camera: Default::default(),
                    capture_time: capture_local_time.map(|local_time| CaptureTimeEvidence {
                        local_time: local_time.to_owned(),
                        offset_minutes: None,
                        source: CaptureTimeSource::Original,
                        raw_value: local_time.to_owned(),
                    }),
                },
            )
            .expect("stage gallery location");
    }
    catalog
        .publish_scan(
            scan_id,
            root_id,
            u64::try_from(locations.len()).expect("gallery location count"),
            0,
        )
        .expect("publish gallery scan");
}

pub(super) fn prove_gallery_first_import_handoff_if_needed(
    catalog: &mut SqliteCatalog,
    scan_id: &str,
    root_id: &str,
) {
    let is_first_import = catalog
        .connection
        .query_row(
            "SELECT active_scan_id IS NULL FROM library_roots WHERE id = ?1",
            [root_id],
            |row| row.get::<_, bool>(0),
        )
        .expect("load gallery fixture publication state");
    if is_first_import {
        catalog
            .prove_live_only_first_import_handoff_for_test(scan_id)
            .expect("prove gallery fixture first-import handoff");
    }
}

pub(super) fn fixture_request(scan_id: &str, root_path: &str) -> ScanRequest {
    ScanRequest {
        scan_id: scan_id.to_owned(),
        root_path: root_path.to_owned(),
        max_items: Some(500),
        max_entries: Some(2_000),
        preview_edge: 512,
    }
}

pub(super) fn publication_identity(hex_digit: char) -> FileIdentityEvidence {
    FileIdentityEvidence {
        scheme: "windows-file-id-128-v1".to_owned(),
        value: format!("000000000000004d:{}", hex_digit.to_string().repeat(32)),
    }
}

pub(super) fn seed_current_explicit_recovery_claim(
    catalog: &SqliteCatalog,
    root_id: &str,
    observed_unix_ms: i64,
) {
    let catalog_revision: i64 = catalog
        .connection
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("catalog revision");
    catalog
        .connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, attempt_count, next_retry_unix_ms,
               last_failure_code, last_failure_message,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               ?2, ?2, ?3, ?3, 1, 'retry_wait', ?2, 0, NULL,
               'live_gap_v30_explicit_recovery_required',
               'The ambiguous historical gap requires an explicit library update',
               ?4, ?2, ?2
             )",
            params![
                root_id,
                observed_unix_ms,
                observed_unix_ms.to_string(),
                catalog_revision,
            ],
        )
        .expect("insert explicit recovery gap");
    let gap_change_id = catalog.connection.last_insert_rowid();
    catalog
        .connection
        .execute(
            "INSERT INTO library_live_gap_recovery_claims(
               gap_change_id, root_id, root_generation, consumer_kind,
               created_unix_ms
             ) VALUES (?1, ?2, 1, 'explicit_recovery_required', ?3)",
            params![gap_change_id, root_id, observed_unix_ms],
        )
        .expect("insert explicit recovery claim");
}

pub(super) fn downgrade_live_gap_contract_to_v29(connection: &Connection) {
    downgrade_source_revision_contract_to_v30_for_test(connection);
    connection
        .execute_batch(
            "DROP TRIGGER library_live_gap_recovery_claim_identity_update_guard;
             DROP TRIGGER library_live_gap_recovery_claim_insert_guard;
             DROP INDEX library_live_gap_recovery_claims_root;
             DROP TABLE library_live_gap_recovery_claims;
             DROP TABLE library_live_gap_recovery_contract;
             PRAGMA user_version = 29;
             UPDATE schema_info SET version = 29;",
        )
        .expect("downgrade fixture to v29");
}

pub(in crate::adapters::sqlite_catalog) fn migrate_v29_ambiguous_gap_fixture(
    catalog_path: &Path,
    root_id: &str,
    root_path: &str,
    with_publication_namespace: bool,
) {
    let mut connection = Connection::open(catalog_path).expect("open ambiguous v29 fixture");
    crate::adapters::sqlite_catalog::migrations::migrate_schema(&mut connection)
        .expect("create current fixture schema");
    connection
        .execute(
            "INSERT INTO library_roots(id, path, created_unix_ms) VALUES (?1, ?2, 1)",
            params![root_id, root_path],
        )
        .expect("insert ambiguous fixture root");
    connection
        .execute(
            "INSERT INTO library_change_root_state(
               root_id, generation, is_active, updated_unix_ms
             ) VALUES (?1, 1, 1, 1)",
            [root_id],
        )
        .expect("insert ambiguous fixture generation");
    connection
        .execute(
            "INSERT INTO library_persistent_journal_root_state(
               root_id, root_generation, protocol_version, contract_version,
               capability_state, continuity_state, updated_unix_ms
             ) VALUES (?1, 1, 0, 1, 'unknown', 'baseline_required', 1)",
            [root_id],
        )
        .expect("insert ambiguous fixture journal state");
    if with_publication_namespace {
        connection
            .execute(
                "INSERT INTO library_root_publication_namespaces(
                   root_id, root_generation, identity_scheme, identity_value,
                   authority_kind, established_catalog_revision,
                   established_unix_ms, updated_unix_ms
                 ) VALUES (
                   ?1, 1, 'windows-file-id-128-v1',
                   '000000000000004d:01010101010101010101010101010101',
                   'foreground_scan', 0, 1, 1
                 )",
                [root_id],
            )
            .expect("insert ambiguous fixture publication namespace");
    }
    connection
        .execute(
            "INSERT INTO library_change_queue(
               root_id, root_generation, intent_kind, scope, relative_path,
               origin, first_observed_unix_ms, most_recent_observed_unix_ms,
               first_sequence, most_recent_sequence, coalesced_observation_count,
               status, ready_unix_ms, lease_expires_unix_ms, attempt_count,
               catalog_revision_at_enqueue, created_unix_ms, updated_unix_ms
             ) VALUES (
               ?1, 1, 'freshness_unknown', 'root', '', 'startup_catch_up',
               41, 41, '1', '1', 1, 'leased', 41, 99, 3, 0, 41, 41
             )",
            [root_id],
        )
        .expect("insert ambiguous v29 gap");
    downgrade_live_gap_contract_to_v29(&connection);
}
