use crate::domain::{FileIdentityEvidence, SourceRevisionEvidence};

use super::*;

fn rejected_file(path: &str, revision: u64) -> DiscoveredFile {
    DiscoveredFile {
        source_root_path: "fixture-root".to_owned(),
        absolute_path: format!("fixture-root/{path}"),
        relative_path: path.to_owned(),
        file_size: 2,
        created_unix_ms: None,
        modified_unix_ms: 42,
        file_identity: Some(FileIdentityEvidence {
            scheme: "windows-file-id-v1".to_owned(),
            value: "fixture-file-identity".to_owned(),
        }),
        source_revision: Some(SourceRevisionEvidence {
            scheme: "windows-file-change-time-100ns-v1".to_owned(),
            value: format!("{revision:016x}"),
        }),
        source_generation: 0,
        issues: Vec::new(),
    }
}

#[test]
fn rejected_input_roster_is_keyset_bounded_and_survives_positive_roster_capture() {
    let storage = tempfile::tempdir().expect("isolated storage");
    let mut catalog = SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("catalog");
    let roster = RejectedInputValidationRoster::create(&catalog).expect("negative roster");
    for index in 0..260 {
        roster
            .record(&catalog, &rejected_file(&format!("坏头-{index:03}.bmp"), 1))
            .expect("record exact rejected observation");
    }
    let replacement = rejected_file("坏头-000.bmp", 2);
    roster
        .record(&catalog, &replacement)
        .expect("latest observation replaces earlier version");
    let positive =
        super::super::validation::StagedValidationRoster::capture(&mut catalog, "empty-scan")
            .expect("independent positive roster");
    assert_eq!(positive.total_items(), 0);
    let first = roster
        .load_window(&catalog, None, 256)
        .expect("bounded first window");
    assert_eq!(first.len(), 256);
    assert_eq!(first[0].1.source_revision, replacement.source_revision);
    assert_eq!(first[0].1.file_identity, replacement.file_identity);
    assert_eq!(first[0].1.absolute_path, replacement.absolute_path);
    assert_eq!(first[0].1.file_size, replacement.file_size);
    assert_eq!(first[0].1.modified_unix_ms, replacement.modified_unix_ms);
    let second = roster
        .load_window(&catalog, Some(&first[255].0), 256)
        .expect("last window");
    assert_eq!(second.len(), 4);
    assert_eq!(second[0].0, "坏头-256.bmp");
    assert!(
        roster
            .load_window(&catalog, Some(&second[3].0), 256)
            .expect("exhausted roster")
            .is_empty()
    );
}

#[test]
fn rejected_input_evidence_is_connection_scoped_and_cannot_survive_a_checkpoint_reopen() {
    let storage = tempfile::tempdir().expect("isolated storage");
    let path = storage.path().join("catalog.sqlite3");
    let catalog = SqliteCatalog::open(path.clone()).expect("catalog");
    let first = RejectedInputValidationRoster::create(&catalog).expect("first roster");
    first
        .record(&catalog, &rejected_file("same.bmp", 1))
        .expect("first evidence");
    let second = RejectedInputValidationRoster::create(&catalog).expect("second roster");
    assert!(
        second
            .load_window(&catalog, None, 256)
            .expect("independent roster")
            .is_empty()
    );
    assert_eq!(
        first
            .load_window(&catalog, None, 256)
            .expect("first evidence retained")
            .len(),
        1
    );
    let persistent: i64 = catalog
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name LIKE 'ame_rejected_input_validation_%'",
            [],
            |row| row.get(0),
        )
        .expect("no durable counterfeit proof");
    assert_eq!(persistent, 0);
    drop(catalog);
    let reopened = SqliteCatalog::open(path).expect("catalog reopens without TEMP evidence");
    assert!(
        first.load_window(&reopened, None, 256).is_err(),
        "a missing observation is not an empty proven roster"
    );
}
