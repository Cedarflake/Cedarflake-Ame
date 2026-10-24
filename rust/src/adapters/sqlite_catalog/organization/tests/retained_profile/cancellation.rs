use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::types::ValueRef;

use crate::domain::organization_batch::OrganizationBatchPhase;

use super::*;

#[test]
#[ignore = "manual retained-catalog cancellation; requires a populated isolated organization snapshot"]
fn profile_retained_organization_cancellation() {
    let (directory, path, mut profile) = prepare_copy("organization-cancellation.jsonl");
    let catalog = profile.measure("open_populated_copy", || SqliteCatalog::open(path.clone()));
    let groups = scalar(&catalog, "SELECT COUNT(*) FROM organization_records");
    assert_eq!(groups, 47_833);
    assert_eq!(
        scalar(&catalog, "SELECT SUM(favorite) FROM organization_records"),
        groups
    );
    let original = durable_digest(&catalog.connection);
    let revision = scalar(&catalog, "SELECT revision FROM catalog_state");
    let query = GalleryQuery {
        collection: GalleryCollection::Favorites,
        duplicate_mode: GalleryDuplicateMode::Merged,
        ..GalleryQuery::default()
    };

    let control = Arc::new(OrganizationBatchControl::new());
    let mut batch = SqliteOrganizationBatch::new(
        SqliteCatalog::open(path.clone()).unwrap(),
        query.clone(),
        revision,
        true,
        Arc::clone(&control),
    )
    .unwrap();
    cancel_during(
        &mut profile,
        &control,
        OrganizationBatchPhase::Preparing,
        groups / 4,
        || batch.prepare(),
    );
    drop(batch);
    assert_eq!(durable_digest(&catalog.connection), original);

    let control = Arc::new(OrganizationBatchControl::new());
    let mut batch = SqliteOrganizationBatch::new(
        SqliteCatalog::open(path.clone()).unwrap(),
        query,
        revision,
        true,
        Arc::clone(&control),
    )
    .unwrap();
    let prepared = profile.measure("complete_preparation", || batch.prepare());
    assert_eq!(prepared.group_count, groups);
    assert_eq!(prepared.favorite_count, groups);
    cancel_during(
        &mut profile,
        &control,
        OrganizationBatchPhase::Saving,
        groups / 2,
        || {
            batch.apply(
                "cancel-retained-favorites",
                &OrganizationBatchAction::Favorite { favorite: false },
            )
        },
    );
    drop(batch);
    drop(catalog);
    let reopened = profile.measure("reopen_after_cancellation", || SqliteCatalog::open(path));
    assert_eq!(durable_digest(&reopened.connection), original);
    assert_eq!(
        scalar(&reopened, "SELECT SUM(favorite) FROM organization_records"),
        groups
    );
    assert_eq!(
        scalar(
            &reopened,
            "SELECT COUNT(*) FROM organization_operations WHERE operation_id = 'cancel-retained-favorites'"
        ),
        0,
    );
    profile.record(json!({
        "complete": true,
        "groups": groups,
        "durable_digest_unchanged": true,
        "source_media_reads": 0,
        "scope": "catalog_cancellation_and_reopen_no_client",
    }));
    drop(reopened);
    directory.close().unwrap();
}

struct CancellationObservation {
    requested: Instant,
    completed_groups: u64,
    total_groups: u64,
}

fn cancel_during<T: std::fmt::Debug>(
    profile: &mut Profile,
    control: &Arc<OrganizationBatchControl>,
    phase: OrganizationBatchPhase,
    threshold: u64,
    operation: impl FnOnce() -> Result<T, ScanError>,
) {
    let started = Instant::now();
    let finished = AtomicBool::new(false);
    let (result, returned_at, observation) = std::thread::scope(|scope| {
        let observer = scope.spawn(|| {
            while !finished.load(Ordering::Acquire) {
                let progress = control.snapshot();
                if progress.phase == phase && progress.completed_groups >= threshold {
                    let requested = Instant::now();
                    control.cancelled.store(true, Ordering::Release);
                    return Some(CancellationObservation {
                        requested,
                        completed_groups: progress.completed_groups,
                        total_groups: progress.total_groups,
                    });
                }
                if started.elapsed() >= Duration::from_secs(180) {
                    control.cancelled.store(true, Ordering::Release);
                    return None;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            None
        });
        let result = operation();
        let returned_at = Instant::now();
        finished.store(true, Ordering::Release);
        let observation = observer
            .join()
            .unwrap()
            .expect("required batch progress was not reached");
        (result, returned_at, observation)
    });
    let error = result.unwrap_err();
    let terminal = control.snapshot();
    profile.record(json!({
        "phase": format!("{phase:?}"),
        "elapsed_ms": returned_at.duration_since(started).as_secs_f64() * 1000.0,
        "cancellation_latency_ms": returned_at.duration_since(observation.requested).as_secs_f64() * 1000.0,
        "completed_groups_when_requested": observation.completed_groups,
        "completed_groups_when_stopped": terminal.completed_groups,
        "total_groups": observation.total_groups,
        "error_code": error.code,
    }));
    assert_eq!(error.code, "organization_selection_cancelled");
    assert_eq!(terminal.phase, OrganizationBatchPhase::Cancelled);
    assert_eq!(observation.total_groups, 47_833);
    assert!(terminal.completed_groups >= threshold);
    assert!(terminal.completed_groups < observation.total_groups);
}

fn durable_digest(connection: &Connection) -> blake3::Hash {
    let mut names = connection
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table'
         AND (name GLOB 'organization_*' OR name = 'catalog_state') ORDER BY name",
        )
        .unwrap();
    let tables = names.query_map([], |row| row.get::<_, String>(0)).unwrap();
    let mut digest = blake3::Hasher::new();
    for table in tables {
        let table = table.unwrap();
        super::super::super::authority::field(&mut digest, table.as_bytes());
        let quoted = table.replace('"', "\"\"");
        let mut statement = connection
            .prepare(&format!("SELECT * FROM \"{quoted}\""))
            .unwrap();
        let columns = statement.column_count();
        let ordering = (1..=columns)
            .map(|column| column.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        statement = connection
            .prepare(&format!("SELECT * FROM \"{quoted}\" ORDER BY {ordering}"))
            .unwrap();
        let mut rows = statement.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            for column in 0..columns {
                let value = row.get_ref(column).unwrap();
                let encoded = match value {
                    ValueRef::Null => vec![0],
                    ValueRef::Integer(value) => [vec![1], value.to_le_bytes().to_vec()].concat(),
                    ValueRef::Real(value) => [vec![2], value.to_le_bytes().to_vec()].concat(),
                    ValueRef::Text(value) => [vec![3], value.to_vec()].concat(),
                    ValueRef::Blob(value) => [vec![4], value.to_vec()].concat(),
                };
                super::super::super::authority::field(&mut digest, &encoded);
            }
        }
    }
    digest.finalize()
}
