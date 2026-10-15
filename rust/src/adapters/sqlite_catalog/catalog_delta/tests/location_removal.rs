use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;

#[test]
fn removing_one_location_keeps_work_bounded_as_the_library_grows() {
    let mut counts = Vec::new();
    for entries in [64, 4096] {
        let directory = tempdir().unwrap();
        let mut catalog = SqliteCatalog::open(directory.path().join("catalog.sqlite3")).unwrap();
        let locations = (0..entries)
            .map(|index| {
                location(
                    &format!("asset-{index}"),
                    &format!("location-{index}"),
                    "root-a",
                    &format!("photo-{index}.jpg"),
                )
            })
            .collect::<Vec<_>>();
        seed_catalog(&mut catalog, "root-a", "C:/source-a", &locations);
        seed_catalog(
            &mut catalog,
            "root-b",
            "C:/source-b",
            &[location("peer-asset", "location-0", "root-b", "peer.jpg")],
        );
        let steps = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&steps);
        catalog
            .connection
            .progress_handler(
                1,
                Some(move || {
                    observed.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
            .unwrap();
        let transaction = catalog.connection.transaction().unwrap();
        super::super::location_removal::remove(
            &transaction,
            "scan-root-a",
            None,
            "root-a",
            "location-0",
        )
        .unwrap();
        transaction.commit().unwrap();
        catalog
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .unwrap();
        counts.push(steps.load(Ordering::Relaxed));

        let remaining = catalog
            .connection
            .query_row(
                "SELECT count(*) FROM asset_locations WHERE root_id = 'root-a'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        assert_eq!(remaining, entries - 1);
        assert!(
            catalog
                .load_incremental_location_by_relative_path("root-a", "photo-0.jpg")
                .unwrap()
                .is_none()
        );
        assert!(
            catalog
                .load_incremental_location_by_relative_path("root-b", "peer.jpg")
                .unwrap()
                .is_some()
        );
    }
    assert!(
        counts[1] <= counts[0] * 2,
        "Removing one location must not traverse unrelated library rows: {counts:?}"
    );
}
