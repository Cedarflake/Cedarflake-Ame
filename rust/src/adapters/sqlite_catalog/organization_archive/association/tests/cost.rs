use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn organization_archive_matching_does_not_multiply_historical_bindings_by_hardlink_copies() {
    let (directory, catalog) = fixture();
    native(&catalog, IDENTITY, REVISION);
    content(&catalog, 'a', true);
    catalog.connection.execute_batch(
        "WITH RECURSIVE sequence(value) AS (VALUES(1) UNION ALL SELECT value + 1 FROM sequence WHERE value < 600)
         INSERT INTO organization_records
           SELECT 'alias-' || value, NULL, 'pending', 0, 0, 0, 1, 1, NULL FROM sequence;
         INSERT INTO organization_record_groups
           SELECT record_id, 'record-a' FROM organization_records WHERE record_id <> 'record-a';
         INSERT INTO organization_bindings
           SELECT alias.record_id, alias.record_id, binding.source_generation + 1000, binding.byte_length,
             binding.file_identity_scheme, binding.file_identity_value, binding.source_revision_token,
             binding.fingerprint_scheme, binding.digest
           FROM organization_records AS alias CROSS JOIN organization_bindings AS binding
           WHERE alias.record_id <> 'record-a' AND binding.record_id = 'record-a';",
    ).unwrap();
    let archive = backup(directory.path(), &catalog);
    let names = (0..600)
        .map(|value| format!("copy-{value:04}"))
        .collect::<Vec<_>>();
    let locations = names
        .iter()
        .map(|name| (name.as_str(), None, 1))
        .collect::<Vec<_>>();
    let rebuilt = target(directory.path(), &locations);
    native(&rebuilt, IDENTITY, REVISION);
    content(&rebuilt, 'b', true);
    let stage =
        super::super::super::staging::read(&archive, directory.path(), &mut |_| true).unwrap();
    let snapshot =
        Connection::open_with_flags(&rebuilt.path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    snapshot
        .execute(
            "ATTACH DATABASE ?1 AS recovery",
            [stage.path().to_str().unwrap()],
        )
        .unwrap();
    current::capture(&snapshot, &mut |_| Ok(())).unwrap();
    let versions: i64 = snapshot
        .query_row(
            "SELECT COUNT(*) FROM temp.organization_restore_native",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(versions, 1);
    let steps = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&steps);
    snapshot
        .progress_handler(
            100,
            Some(move || {
                observed.fetch_add(100, Ordering::Relaxed);
                false
            }),
        )
        .unwrap();
    matching::groups(&snapshot, &mut |_| Ok(())).unwrap();
    snapshot.progress_handler(0, None::<fn() -> bool>).unwrap();
    let count: i64 = snapshot
        .query_row(
            "SELECT COUNT(*) FROM temp.organization_restore_matched_groups",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    let steps = steps.load(Ordering::Relaxed);
    assert!(
        steps < 150_000,
        "native-version matching used {steps} SQLite steps"
    );
}
