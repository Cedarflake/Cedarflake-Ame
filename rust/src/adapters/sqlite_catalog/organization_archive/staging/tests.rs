use std::fs;

use serde_json::{Value, json};

use crate::ports::organization_archive::OrganizationArchiveRepository;

use super::super::tests::{assert_no_scratch, fixture};
use super::*;

#[test]
fn organization_archive_inspection_distinguishes_storage_failure_from_invalid_records() {
    let storage = write_error(
        "album",
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_FULL), None),
    );
    assert_ne!(storage.code, "organization_archive_records_invalid");
    let invalid = write_error(
        "album",
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY),
            None,
        ),
    );
    assert_eq!(invalid.code, "organization_archive_records_invalid");
}

fn entries(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn write_archive(path: &Path, entries: &[Value]) {
    let mut bytes = Vec::new();
    let mut counts = vec![0_u64; records::KINDS.len()];
    for entry in entries.iter().filter(|entry| entry["type"] != "footer") {
        serde_json::to_writer(&mut bytes, entry).unwrap();
        bytes.push(b'\n');
        if entry["type"] == "row" {
            let kind = records::KINDS
                .iter()
                .position(|kind| kind.name == entry["kind"])
                .unwrap();
            counts[kind] += 1;
        }
    }
    let checksum = blake3::hash(&bytes).to_hex().to_string();
    serde_json::to_writer(
        &mut bytes,
        &json!({"type":"footer", "counts":counts, "checksum":checksum}),
    )
    .unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
}

#[test]
fn organization_archive_inspection_upgrades_v1_native_copy_evidence_without_changing_input() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("version-one.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let mut rows = entries(&archive);
    rows[0]["header"]["version"] = json!(1);
    let index = rows
        .iter()
        .position(|row| row["kind"] == "version_evidence")
        .unwrap();
    let values = rows[index]["values"].as_array_mut().unwrap();
    values.truncate(10);
    values[5] = json!("windows-file-id-128-v1");
    values[6] = json!("0000000000000001:00000000000000000000000000000001");
    let mut alias = rows[index].clone();
    alias["values"][0] = json!("old-hardlink");
    rows.insert(index + 1, alias);
    write_archive(&archive, &rows);
    let original = fs::read(&archive).unwrap();
    let staged = read(&archive, directory.path(), &mut |_| true).unwrap();
    assert_eq!(
        staged
            .connection
            .query_row(
                "SELECT COUNT(*) FROM archive_version_evidence WHERE native_copy_count = 2",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        2
    );
    assert_eq!(fs::read(&archive).unwrap(), original);
    drop(staged);
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_inspection_rejects_unproven_v2_native_copy_counts() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("version-two.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let original = entries(&archive);
    assert_eq!(original[0]["header"]["version"], 2);
    for copies in [json!(0), json!(-1), Value::Null] {
        let mut rows = original.clone();
        rows.iter_mut()
            .find(|row| row["kind"] == "version_evidence")
            .unwrap()["values"][10] = copies;
        write_archive(&archive, &rows);
        assert_eq!(
            read(&archive, directory.path(), &mut |_| true)
                .err()
                .unwrap()
                .code,
            "organization_archive_invalid"
        );
        assert_no_scratch(directory.path());
    }
}

#[test]
fn organization_archive_inspection_rejects_identifiers_that_cannot_be_used_by_commands() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("identifiers.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let original = entries(&archive);
    for identifier in [String::new(), "a".repeat(257), "album\nidentifier".into()] {
        let mut rows = original.clone();
        rows.iter_mut().find(|row| row["kind"] == "album").unwrap()["values"][0] =
            json!(identifier);
        write_archive(&archive, &rows);
        assert_eq!(
            read(&archive, directory.path(), &mut |_| true)
                .err()
                .unwrap()
                .code,
            "organization_archive_records_invalid"
        );
        assert_no_scratch(directory.path());
    }
}

#[test]
fn organization_archive_inspection_validates_and_counts_logical_groups_without_live_writes() {
    let (directory, catalog) = fixture();
    catalog
        .connection
        .execute_batch(
            "INSERT INTO organization_records
        VALUES ('alias', NULL, 'pending', 0, 0, 0, 1, 1, NULL);
        INSERT INTO organization_record_groups VALUES ('alias', 'record-a');",
        )
        .unwrap();
    let archive = directory.path().join("恢复.ame-backup");
    let exported = catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let changes = catalog.connection.total_changes();
    let before = fs::read(&archive).unwrap();
    let inspection = catalog
        .inspect_organization_archive(&archive, &mut |_| true)
        .unwrap();
    assert_eq!(inspection.archive_id, exported.archive_id);
    assert_eq!(inspection.record_count, exported.record_count);
    assert_eq!(inspection.checksum.len(), 64);
    assert_eq!(inspection.active_albums, 1);
    assert_eq!(inspection.groups, 1);
    assert_eq!(inspection.favorite_groups, 1);
    assert_eq!(inspection.conflicting_groups, 1);
    assert_eq!(inspection.duplicate_choices, 1);
    assert!(inspection.has_continuation);
    assert_eq!(catalog.connection.total_changes(), changes);
    assert_eq!(fs::read(archive).unwrap(), before);
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_inspection_rejects_valid_checksums_with_broken_relationships() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("input.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let original = entries(&archive);
    for scenario in [
        "missing_album",
        "missing_group",
        "missing_target",
        "future_revision",
        "bookmark_scope",
        "retired_assignment",
        "duplicate_album",
        "non_deferred_conflict",
        "invalid_favorite",
        "unowned_admission",
        "history_revision",
    ] {
        let mut altered = original.clone();
        match scenario {
            "missing_album" => altered
                .retain(|entry| !(entry["kind"] == "album" && entry["values"][0] == "album-a")),
            "missing_group" => altered.retain(|entry| entry["kind"] != "group"),
            "missing_target" => altered.retain(|entry| entry["kind"] != "operation_target"),
            "unowned_admission" => altered.retain(|entry| entry["kind"] != "admission_source"),
            "history_revision" => {
                altered
                    .iter_mut()
                    .find(|entry| entry["kind"] == "operation_change")
                    .unwrap()["values"][6] = json!(2)
            }
            "future_revision" => {
                altered
                    .iter_mut()
                    .find(|entry| entry["kind"] == "record")
                    .unwrap()["values"][4] = json!(4)
            }
            "bookmark_scope" => altered.retain(|entry| entry["kind"] != "review_scope"),
            "retired_assignment" => {
                let record = altered
                    .iter_mut()
                    .find(|entry| entry["kind"] == "record")
                    .unwrap();
                record["values"][1] = json!("album-b");
                record["values"][2] = json!("organized");
            }
            "duplicate_album" => {
                let index = altered
                    .iter()
                    .position(|entry| entry["kind"] == "album")
                    .unwrap();
                altered.insert(index, altered[index].clone());
            }
            "non_deferred_conflict" => {
                altered
                    .iter_mut()
                    .find(|entry| entry["kind"] == "record")
                    .unwrap()["values"][2] = json!("ignored")
            }
            "invalid_favorite" => {
                altered
                    .iter_mut()
                    .find(|entry| entry["kind"] == "record")
                    .unwrap()["values"][3] = json!(2)
            }
            _ => unreachable!(),
        }
        write_archive(&archive, &altered);
        let failure = catalog
            .inspect_organization_archive(&archive, &mut |_| true)
            .unwrap_err();
        assert_eq!(
            failure.code, "organization_archive_records_invalid",
            "{scenario}"
        );
        assert_no_scratch(directory.path());
        assert_eq!(
            catalog
                .connection
                .query_row("SELECT revision FROM organization_state", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
    }
}

#[test]
fn organization_archive_inspection_rejects_group_cycles_but_preserves_deleted_undo_targets() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("groups.ame-backup");
    catalog.connection.execute_batch("INSERT INTO organization_records
        VALUES ('record-b', NULL, 'pending', 0, 0, 0, 1, 1, NULL);
        INSERT INTO organization_record_groups VALUES ('record-b', 'record-b');
        INSERT INTO organization_operation_targets VALUES ('operation-a', 'deleted-record', 'organization');").unwrap();
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    catalog
        .inspect_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let mut altered = entries(&archive);
    for entry in altered.iter_mut().filter(|entry| entry["kind"] == "group") {
        entry["values"][1] = if entry["values"][0] == "record-a" {
            json!("record-b")
        } else {
            json!("record-a")
        };
    }
    write_archive(&archive, &altered);
    assert_eq!(
        catalog
            .inspect_organization_archive(&archive, &mut |_| true)
            .unwrap_err()
            .code,
        "organization_archive_records_invalid"
    );
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_inspection_handles_empty_and_sql_like_names_as_data() {
    let directory = tempfile::tempdir().unwrap();
    let catalog =
        super::super::SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let archive = directory.path().join("empty.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let empty = catalog
        .inspect_organization_archive(&archive, &mut |_| true)
        .unwrap();
    assert_eq!(empty.groups, 0);
    assert_eq!(empty.record_count, 0);
    assert!(!empty.has_continuation);
    let mut altered = entries(&archive);
    altered[0]["header"]["organization_revision"] = json!(1);
    altered.insert(1, json!({"type":"row", "kind":"album", "values":["a", "'); DROP TABLE archive_album; --", 1, 0, 0]}));
    write_archive(&archive, &altered);
    assert_eq!(
        catalog
            .inspect_organization_archive(&archive, &mut |_| true)
            .unwrap()
            .active_albums,
        1
    );
    assert_no_scratch(directory.path());
}

#[test]
fn organization_archive_inspection_cancels_while_staging_without_touching_input_or_catalog() {
    let directory = tempfile::tempdir().unwrap();
    let catalog =
        super::super::SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
    let archive = directory.path().join("many.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let mut altered = entries(&archive);
    altered[0]["header"]["organization_revision"] = json!(1);
    for number in 0..512 {
        altered.insert(altered.len() - 1, json!({"type":"row", "kind":"album", "values":[format!("album-{number}"), "测试", 1, 0, 0]}));
    }
    write_archive(&archive, &altered);
    let before = fs::read(&archive).unwrap();
    let failure = catalog
        .inspect_organization_archive(&archive, &mut |progress| progress.completed_units < 256)
        .unwrap_err();
    assert_eq!(failure.code, "organization_archive_cancelled");
    assert_eq!(before, fs::read(archive).unwrap());
    assert_no_scratch(directory.path());
}

#[cfg(windows)]
#[test]
fn organization_archive_inspection_holds_input_against_concurrent_writers() {
    let (directory, catalog) = fixture();
    let archive = directory.path().join("held.ame-backup");
    catalog
        .export_organization_archive(&archive, &mut |_| true)
        .unwrap();
    let input = input::open(&archive).unwrap();
    assert!(fs::OpenOptions::new().write(true).open(&archive).is_err());
    drop(input);
    let writer = fs::OpenOptions::new().write(true).open(&archive).unwrap();
    assert_eq!(
        catalog
            .inspect_organization_archive(&archive, &mut |_| true)
            .unwrap_err()
            .code,
        "organization_archive_io"
    );
    drop(writer);
    catalog
        .inspect_organization_archive(&archive, &mut |_| true)
        .unwrap();
    assert_no_scratch(directory.path());
}
