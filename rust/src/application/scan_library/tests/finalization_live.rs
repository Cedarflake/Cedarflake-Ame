use super::*;

struct Fixture {
    source: tempfile::TempDir,
    _storage: tempfile::TempDir,
    storage: StoragePaths,
    root_id: String,
}

impl Fixture {
    fn new(count: usize) -> Self {
        let source = tempdir().expect("source");
        let storage_directory = tempdir().expect("storage");
        for index in 0..count {
            RgbaImage::from_pixel(8, 6, Rgba([20, 40, 60, 255]))
                .save(source.path().join(format!("initial-{index}.png")))
                .expect("source fixture");
        }
        let storage = StoragePaths {
            catalog_path: storage_directory.path().join("catalog.sqlite3"),
            preview_root: storage_directory.path().join("previews"),
            preview_budget_bytes: 64 * 1024 * 1024,
            settings_path: storage_directory.path().join("settings.sqlite3"),
        };
        let canonical = FileDiscovery::new(&source.path().to_string_lossy())
            .expect("discovery")
            .canonical_root()
            .expect("canonical root")
            .to_string_lossy()
            .into_owned();
        let fixture = Self {
            source,
            _storage: storage_directory,
            storage,
            root_id: stable_id("library-root-v1", &canonical),
        };
        run_scan_with_storage(
            fixture.request("baseline"),
            |_| true,
            fixture.storage.clone(),
        )
        .expect("baseline scan");
        fixture
    }

    fn request(&self, scan_id: &str) -> ScanRequest {
        ScanRequest {
            scan_id: self.scan_id(scan_id),
            root_path: self.source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        }
    }

    fn scan_id(&self, phase: &str) -> String {
        format!("{}-{phase}", self.root_id)
    }

    fn image(&self, path: &str, width: u32) {
        RgbaImage::from_pixel(width, 7, Rgba([210, 70, 30, 255]))
            .save(self.source.path().join(path))
            .expect("controlled source change");
    }

    fn materialize_preview(&self, path: &str) -> crate::domain::AssetLocationView {
        let location = crate::application::catalog_session::open_catalog(
            &self.storage.catalog_path,
            LibraryChangeLane::Live,
        )
        .expect("active preview catalog")
        .load_active_location(&stable_location_id(&self.root_id, path))
        .expect("active preview location")
        .expect("active location exists");
        let previewed = crate::application::preview::materialize_preview_with_storage(
            crate::domain::PreviewRequest {
                location_id: location.location_id,
                expected_root_id: location.root_id,
                expected_scan_id: location.scan_id,
                expected_source_revision: location.source_revision,
                expected_source_generation: location.source_generation,
                preview_edge: 128,
                retry_failed: false,
                protected_location_ids: Vec::new(),
            },
            self.storage.clone(),
        )
        .expect("real preview publication");
        assert!(matches!(previewed.preview_status, PreviewStatus::Ready));
        assert!(PathBuf::from(&previewed.preview_path).is_file());
        previewed
    }

    fn payload(&self, phase: &str, path: &str) -> Vec<rusqlite::types::Value> {
        let connection = Connection::open(&self.storage.catalog_path).expect("payload catalog");
        let mut statement = connection
            .prepare("SELECT * FROM asset_locations WHERE scan_id = ?1 AND relative_path = ?2")
            .expect("complete payload");
        let columns = statement
            .column_names()
            .iter()
            .enumerate()
            .filter_map(|(index, name)| (*name != "scan_id").then_some(index))
            .collect::<Vec<_>>();
        statement
            .query_row(rusqlite::params![self.scan_id(phase), path], |row| {
                columns.iter().map(|index| row.get(*index)).collect()
            })
            .expect("published payload")
    }

    fn assert_preview_is_referenced(&self, preview: &crate::domain::AssetLocationView) {
        let connection = Connection::open(&self.storage.catalog_path).expect("preview references");
        let references: i64 = connection.query_row(
            "SELECT COUNT(*) FROM preview_artifacts AS artifact
             JOIN preview_artifact_locations AS reference ON reference.artifact_key = artifact.artifact_key
             JOIN asset_locations AS location ON location.location_id = reference.location_id
             JOIN library_roots AS root ON root.id = location.root_id AND root.active_scan_id = location.scan_id
             WHERE location.location_id = ?1 AND location.preview_path = ?2
               AND artifact.artifact_path = location.preview_path
               AND location.preview_status = 'ready' AND artifact.lifecycle_state = 'ready'
               AND artifact.source_generation = location.source_generation
               AND artifact.source_revision_token IS location.source_revision_token",
            rusqlite::params![preview.location_id, preview.preview_path], |row| row.get(0),
        ).expect("retained managed ready reference");
        assert_eq!(
            references, 1,
            "snapshot replacement must not orphan its ready preview"
        );
    }

    fn drain_live(&self, paths: &[String]) {
        let mut catalog = crate::application::catalog_session::open_catalog(
            &self.storage.catalog_path,
            LibraryChangeLane::Live,
        )
        .expect("cached live catalog");
        let now = current_unix_ms().expect("clock");
        let intents = paths
            .iter()
            .zip(1_u64..)
            .map(|(path, sequence)| LibraryChangeIntent {
                root_id: self.root_id.clone(),
                root_generation: LibraryRootGeneration::initial(),
                kind: LibraryChangeIntentKind::Reconcile,
                scope: LibraryChangeScope::Path,
                relative_path: path.clone(),
                previous_relative_path: None,
                origin: LibraryChangeOrigin::LiveNotification,
                first_observed_unix_ms: now,
                most_recent_observed_unix_ms: now,
                first_sequence: sequence,
                most_recent_sequence: sequence,
                coalesced_observation_count: 1,
            })
            .collect::<Vec<_>>();
        catalog
            .enqueue_library_change_intents(&intents, now, LibraryChangeQueuePolicy::default())
            .expect("enqueue live paths");
        let report = crate::application::process_ready_library_changes_in_lane(
            &mut catalog,
            &self.root_id,
            LibraryRootGeneration::initial(),
            LibraryChangeLane::Live,
            now.saturating_add(1_000),
            LibraryChangeQueuePolicy::default(),
        )
        .expect("process real P0 path");
        assert_eq!(
            report.applied_mutation_count,
            u32::try_from(paths.len()).expect("bounded fixture path count"),
        );
    }

    fn active_paths(&self) -> Vec<String> {
        let connection = Connection::open(&self.storage.catalog_path).expect("catalog assertions");
        let mut statement = connection.prepare(
            "SELECT locations.relative_path FROM asset_locations AS locations JOIN library_roots AS roots
             ON roots.id = locations.root_id AND roots.active_scan_id = locations.scan_id
             WHERE roots.id = ?1 ORDER BY locations.location_id"
        ).expect("active keyset");
        statement
            .query_map([&self.root_id], |row| row.get(0))
            .expect("keyset rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("active paths")
    }

    fn assert_completed(&self, expected_count: usize) {
        let connection = Connection::open(&self.storage.catalog_path).expect("catalog assertions");
        let (active, count): (String, i64) = connection
            .query_row(
                "SELECT roots.active_scan_id, scan.asset_count FROM library_roots AS roots
             JOIN scan_runs AS scan ON scan.id = roots.active_scan_id WHERE roots.id = ?1",
                [&self.root_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("published scan");
        assert_eq!(active, self.scan_id("replacement"));
        assert_eq!(count, expected_count as i64);
        assert_eq!(self.active_paths().len(), expected_count);
        let scan_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM scan_runs", [], |row| row.get(0))
            .expect("scan count");
        assert_eq!(scan_count, 2, "no replacement full-scan retry");
    }
}

#[test]
fn replacement_finalization_accepts_p0_addition_after_roster_capture() {
    let fixture = Fixture::new(1);
    let mut changed = false;
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if !changed && matches!(event, ScanEvent::Finalizing { .. }) {
                changed = true;
                fixture.image("added.png", 9);
                fixture.drain_live(&["added.png".to_owned()]);
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("replacement with live addition");
    assert!(changed);
    fixture.assert_completed(2);
}

#[test]
fn replacement_finalization_accepts_p0_deletion_after_roster_capture() {
    let fixture = Fixture::new(2);
    let mut changed = false;
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if !changed && matches!(event, ScanEvent::Finalizing { .. }) {
                changed = true;
                fs::remove_file(fixture.source.path().join("initial-0.png"))
                    .expect("controlled deletion");
                fixture.drain_live(&["initial-0.png".to_owned()]);
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("replacement with live deletion");
    assert!(changed);
    fixture.assert_completed(1);
}

#[test]
fn replacement_finalization_keeps_a_fixed_keyset_across_live_cursor_and_window_changes() {
    let fixture = Fixture::new(257);
    let initial = fixture.active_paths();
    let cursor = stable_location_id(&fixture.root_id, &initial[127]);
    let before = (0..10_000)
        .map(|index| format!("new-before-{index}.png"))
        .find(|path| stable_location_id(&fixture.root_id, path) < cursor)
        .expect("key before cursor");
    let after = (0..10_000)
        .map(|index| format!("new-after-{index}.png"))
        .find(|path| stable_location_id(&fixture.root_id, path) > cursor)
        .expect("key after cursor");
    let mut changed = false;
    let mut progress = Vec::new();
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if let ScanEvent::Finalizing {
                validated_items,
                total_items,
                ..
            } = event
            {
                progress.push((validated_items, total_items));
                if !changed && validated_items == 128 {
                    changed = true;
                    fixture.image(&before, 9);
                    fixture.image(&after, 10);
                    fs::remove_file(fixture.source.path().join(&initial[0]))
                        .expect("remove validated item");
                    fs::remove_file(fixture.source.path().join(&initial[200]))
                        .expect("remove queued window item");
                    fixture.image(&initial[10], 11);
                    fixture.image(&initial[190], 12);
                    fixture.drain_live(&[
                        before.clone(),
                        after.clone(),
                        initial[0].clone(),
                        initial[200].clone(),
                        initial[10].clone(),
                        initial[190].clone(),
                    ]);
                }
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("fixed-roster replacement");
    assert!(changed);
    assert!(
        progress
            .iter()
            .all(|(done, total)| *total == 257 && *done <= 257)
    );
    assert!(progress.contains(&(256, 257)));
    assert_eq!(progress.last(), Some(&(257, 257)));
    fixture.assert_completed(257);
    let paths = fixture.active_paths();
    assert!(paths.contains(&before) && paths.contains(&after));
    assert!(!paths.contains(&initial[0]) && !paths.contains(&initial[200]));
    let connection = Connection::open(&fixture.storage.catalog_path).expect("metadata assertions");
    for (path, expected_width) in [(&initial[10], 11_i64), (&initial[190], 12)] {
        let width: i64 = connection
            .query_row(
                "SELECT width FROM asset_locations WHERE scan_id = ?1 AND relative_path = ?2",
                rusqlite::params![fixture.scan_id("replacement"), path],
                |row| row.get(0),
            )
            .expect("current live dimensions");
        assert_eq!(width, expected_width);
    }
}

#[test]
fn replacement_finalization_rejects_unproven_payload_changes_after_validation() {
    let fixture = Fixture::new(1);
    let result = run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 1,
                    total_items: 1,
                    ..
                }
            ) {
                Connection::open(&fixture.storage.catalog_path)
                    .expect("tamper fixture")
                    .execute(
                        "UPDATE asset_locations SET file_size = file_size + 1 WHERE scan_id = ?1",
                        [fixture.scan_id("replacement")],
                    )
                    .expect("unproven staged source lease change");
            }
            true
        },
        fixture.storage.clone(),
    );
    assert_eq!(
        result.expect_err("unproven change must not publish").code,
        "catalog_scan_validation_proof_mismatch"
    );
    let connection = Connection::open(&fixture.storage.catalog_path).expect("retained baseline");
    let active: String = connection
        .query_row("SELECT active_scan_id FROM library_roots", [], |row| {
            row.get(0)
        })
        .expect("active scan");
    assert_eq!(active, fixture.scan_id("baseline"));
}

#[test]
fn replacement_finalization_does_not_reuse_a_roster_item_skipped_for_live_publication() {
    let fixture = Fixture::new(1);
    let connection = Connection::open(&fixture.storage.catalog_path).expect("initial payload");
    let mut statement = connection
        .prepare("PRAGMA table_info(asset_locations)")
        .expect("payload columns");
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("columns")
        .collect::<Result<Vec<_>, _>>()
        .expect("column names")
        .into_iter()
        .filter(|name| name != "scan_id")
        .collect::<Vec<_>>();
    drop(statement);
    drop(connection);
    let mut payload = None;
    let mut changed = false;
    let result = run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 0,
                    ..
                }
            ) && !changed
            {
                changed = true;
                payload = Some(
                    Connection::open(&fixture.storage.catalog_path)
                        .expect("captured staging")
                        .query_row(
                            &format!(
                                "SELECT {} FROM asset_locations WHERE scan_id = ?1",
                                columns.join(",")
                            ),
                            [fixture.scan_id("replacement")],
                            |row| {
                                (0..columns.len())
                                    .map(|index| row.get::<_, rusqlite::types::Value>(index))
                                    .collect::<Result<Vec<_>, _>>()
                            },
                        )
                        .expect("exact captured roster payload"),
                );
                fixture.image("initial-0.png", 9);
                fixture.drain_live(&["initial-0.png".to_owned()]);
            } else if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 1,
                    total_items: 1,
                    ..
                }
            ) {
                let placeholders = vec!["?"; columns.len()].join(",");
                let mut values = payload.as_ref().expect("captured payload").clone();
                values.push(rusqlite::types::Value::Text(fixture.scan_id("replacement")));
                Connection::open(&fixture.storage.catalog_path)
                    .expect("rollback fixture")
                    .execute(
                        &format!(
                            "UPDATE asset_locations SET ({}) = ({placeholders}) WHERE scan_id = ?",
                            columns.join(",")
                        ),
                        rusqlite::params_from_iter(values.iter()),
                    )
                    .expect("restore skipped original roster payload only in staging");
            }
            true
        },
        fixture.storage.clone(),
    );
    assert!(changed);
    assert_eq!(
        result
            .expect_err("skipped original evidence cannot authorize publication")
            .code,
        "catalog_scan_validation_proof_mismatch"
    );
}

#[test]
fn replacement_finalization_keeps_source_revalidation_without_live_evidence() {
    let fixture = Fixture::new(1);
    let mut changed = false;
    let mut saw_issue = false;
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if !changed
                && matches!(
                    event,
                    ScanEvent::Finalizing {
                        validated_items: 0,
                        ..
                    }
                )
            {
                changed = true;
                fixture.image("initial-0.png", 9);
            }
            if matches!(event, ScanEvent::Issue { .. }) {
                saw_issue = true;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("source race converges through durable P0 handoff");
    assert!(
        changed && saw_issue,
        "an unchanged active/roster pair cannot suppress source revalidation"
    );
    fixture.assert_completed(1);
}

fn assert_live_preview_survives_finalization(path: &str, expected_count: usize) {
    let fixture = Fixture::new(1);
    let mut preview = None;
    let mut expected_payload = None;
    let mut source_bytes = None;
    let mut issues = 0;
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if preview.is_none() && matches!(event, ScanEvent::Finalizing { .. }) {
                fixture.image(path, 13);
                source_bytes = Some(
                    fs::read(fixture.source.path().join(path)).expect("controlled source bytes"),
                );
                fixture.drain_live(&[path.to_owned()]);
                preview = Some(fixture.materialize_preview(path));
                expected_payload = Some(fixture.payload("baseline", path));
                assert_ne!(
                    fixture.payload("replacement", path),
                    *expected_payload.as_ref().expect("ready payload")
                );
            }
            if matches!(event, ScanEvent::Issue { .. }) {
                issues += 1;
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("live preview survives replacement publication");
    assert_eq!(
        issues, 0,
        "a published P0 source must not be re-enqueued because its preview completed"
    );
    fixture.assert_completed(expected_count);
    assert_eq!(
        fixture.payload("replacement", path),
        expected_payload.expect("materialized payload")
    );
    fixture.assert_preview_is_referenced(&preview.expect("materialized preview"));
    assert_eq!(
        fs::read(fixture.source.path().join(path)).expect("unchanged source"),
        source_bytes.expect("captured source bytes")
    );
}

#[test]
fn replacement_finalization_keeps_a_new_live_preview_and_its_artifact_reference() {
    assert_live_preview_survives_finalization("added.png", 2);
}

#[test]
fn replacement_finalization_keeps_a_replaced_live_preview_and_its_artifact_reference() {
    assert_live_preview_survives_finalization("initial-0.png", 1);
}

#[test]
fn replacement_finalization_normalizes_only_derived_changes_from_the_matching_active_lease() {
    let fixture = Fixture::new(1);
    let expected = fixture.payload("baseline", "initial-0.png");
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 1,
                    total_items: 1,
                    ..
                }
            ) {
                Connection::open(&fixture.storage.catalog_path)
                    .expect("derived fixture")
                    .execute(
                        "UPDATE asset_locations SET width = width + 1 WHERE scan_id = ?1",
                        [fixture.scan_id("replacement")],
                    )
                    .expect("unproven derived-only payload");
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("trusted active derived payload wins");
    fixture.assert_completed(1);
    assert_eq!(fixture.payload("replacement", "initial-0.png"), expected);
}

#[test]
fn replacement_finalization_does_not_overwrite_independently_verified_roster_metadata() {
    let fixture = Fixture::new(1);
    Connection::open(&fixture.storage.catalog_path).expect("historical metadata fixture")
        .execute("UPDATE asset_locations SET metadata_engine_version = 'historical-version' WHERE scan_id = ?1", [fixture.scan_id("baseline")])
        .expect("older engine result");
    let mut verified = None;
    run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if matches!(
                event,
                ScanEvent::Finalizing {
                    validated_items: 1,
                    total_items: 1,
                    ..
                }
            ) {
                verified = Some(fixture.payload("replacement", "initial-0.png"));
                assert_ne!(
                    verified.as_ref().expect("new metadata"),
                    &fixture.payload("baseline", "initial-0.png")
                );
            }
            true
        },
        fixture.storage.clone(),
    )
    .expect("independent scan metadata remains authoritative");
    fixture.assert_completed(1);
    assert_eq!(
        fixture.payload("replacement", "initial-0.png"),
        verified.expect("verified roster payload")
    );
}

#[test]
fn replacement_finalization_does_not_adopt_a_ready_preview_across_different_source_leases() {
    let fixture = Fixture::new(1);
    let mut preview = None;
    let result = run_scan_with_storage(
        fixture.request("replacement"),
        |event| {
            if preview.is_none() && matches!(event, ScanEvent::Finalizing { .. }) {
                fixture.image("added.png", 13);
                fixture.drain_live(&["added.png".to_owned()]);
                preview = Some(fixture.materialize_preview("added.png"));
                Connection::open(&fixture.storage.catalog_path).expect("source lease fixture")
                .execute("UPDATE asset_locations SET file_size = file_size + 1 WHERE scan_id = ?1 AND relative_path = 'added.png'", [fixture.scan_id("replacement")])
                .expect("unproven staged source change");
            }
            true
        },
        fixture.storage.clone(),
    );
    assert_eq!(
        result
            .expect_err("different source lease cannot inherit ready preview")
            .code,
        "catalog_scan_validation_proof_mismatch"
    );
    fixture.assert_preview_is_referenced(&preview.expect("retained active preview"));
}
